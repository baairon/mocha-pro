//! Command handlers: ABOUT, GLOBAL_SETUP and PARAMS_SETUP over a plan.

use crate::layout::*;
use crate::plan::{Options, Plan, Reset, Row};
#[cfg(feature = "freestanding")]
use alloc::{ffi::CString, vec::Vec};
use effect_params::Cell;
#[cfg(not(feature = "freestanding"))]
use std::ffi::CString;

fn copy_text(dst: &mut [u8], s: &str) {
    for b in dst.iter_mut() {
        *b = 0;
    }
    let n = s.len().min(dst.len() - 1);
    // Cut at a char boundary so the stored bytes are valid text.
    let mut n2 = n;
    while !s.is_char_boundary(n2) {
        n2 -= 1;
    }
    dst[..n2].copy_from_slice(&s.as_bytes()[..n2]);
}

fn cell_f64(c: &Cell, otherwise: f64) -> f64 {
    match c {
        Cell::Unknown => otherwise,
        Cell::None => 0.0,
        Cell::Int(v) => *v as f64,
        Cell::Float(v) => *v,
    }
}

fn cell_i64(c: &Cell, otherwise: i64) -> i64 {
    match c {
        Cell::Unknown => otherwise,
        Cell::None => 0,
        Cell::Int(v) => *v,
        Cell::Float(v) => *v as i64,
    }
}

/// `f64::round` (ties away from zero) without std. Exact for every finite value: a magnitude of
/// 2^52 or more is already an integer; NaN stays NaN and casts to 0 like the std result.
pub fn round_half_away(x: f64) -> f64 {
    // NaN and infinities fail this test too and come back unchanged.
    if x.is_nan() || x.abs() >= 4503599627370496.0 {
        return x;
    }
    let t = x as i64 as f64;
    let d = x - t;
    if d >= 0.5 {
        t + 1.0
    } else if d <= -0.5 {
        t - 1.0
    } else {
        t
    }
}

fn fixed(v: f64) -> i32 {
    round_half_away(v * 65536.0) as i32
}

/// The strings a definition points at. They live for the whole PARAMS_SETUP so a pointer that
/// carries over into a later definition (reset mode `never`) stays valid.
pub type Keep = Vec<CString>;

fn cstr(keep: &mut Keep, s: &str) -> *const u8 {
    let c = CString::new(s.replace('\0', "")).expect("no NUL left");
    let p = c.as_ptr() as *const u8;
    keep.push(c);
    p
}

/// Fills every byte of the definition structure with `byte` (padding included).
pub fn fill_def(d: &mut ParamDef, byte: u8) {
    // SAFETY: the structure is plain data and every bit pattern is a valid value of its members.
    unsafe {
        core::ptr::write_bytes(
            d as *mut ParamDef as *mut u8,
            byte,
            core::mem::size_of::<ParamDef>(),
        )
    }
}

/// Writes the name like a string copy: the bytes and one NUL, the rest of the field is untouched.
/// A name longer than 31 bytes is cut at a character boundary.
fn write_name(dst: &mut [u8; 32], s: &str) {
    let mut n = s.len().min(31);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    dst[..n].copy_from_slice(&s.as_bytes()[..n]);
    dst[n] = 0;
}

/// Builds the definition of one row on a zeroed structure (the `each` mode with fill 0).
pub fn build_def(row: &Row, engine_flags: u64, appl_id: u32) -> (ParamDef, Keep) {
    let mut d = ParamDef::zeroed();
    let mut keep = Vec::new();
    apply_row(
        &mut d,
        &mut keep,
        row,
        engine_flags,
        appl_id,
        &Options::default(),
        core::ptr::null_mut(),
    );
    (d, keep)
}

/// Writes the fields of one row into `d`. A field the row does not state is not written.
pub fn apply_row(
    d: &mut ParamDef,
    keep: &mut Keep,
    row: &Row,
    engine_flags: u64,
    appl_id: u32,
    opts: &Options,
    handle: *mut core::ffi::c_void,
) {
    if let Some(v) = row.id {
        d.uu_id = v;
    }
    if let Some(v) = row.ui_width {
        d.ui_width = v;
    }
    if let Some(v) = row.ui_height {
        d.ui_height = v;
    }
    if let Some(v) = row.flags {
        d.flags = v;
    }
    if let Some(v) = row.ui_for(engine_flags) {
        d.ui_flags = v;
    }
    d.param_type = row.ty as i32;
    write_name(&mut d.name, &row.name);
    if row.ae_only && appl_id != APPL_ID_AE {
        // Any other host: the same id as an invisible placeholder button, so the count is unchanged.
        d.param_type = ParamType::Button as i32;
        d.flags = opts.placeholder_flags;
        d.ui_flags = PUI_INVISIBLE;
        d.ui_width = 0;
        d.ui_height = 0;
        write_name(&mut d.name, &opts.placeholder_name);
        d.u.button_d.namesptr = cstr(keep, "");
        return;
    }
    let def_i = cell_i64(&row.default, 0);
    let def_f = cell_f64(&row.default, 0.0);
    // SAFETY: every union member written below is plain data.
    unsafe {
        match row.ty {
            ParamType::Layer => d.u.ld.dephault = def_i as i32,
            ParamType::Slider => {
                let (smin, smax) = row
                    .slider
                    .unwrap_or((cell_f64(&row.min, 0.0), cell_f64(&row.max, 1.0)));
                let u = &mut d.u.sd;
                u.value = def_i as i32;
                u.dephault = def_i as i32;
                u.valid_min = cell_i64(&row.min, i32::MIN as i64) as i32;
                u.valid_max = cell_i64(&row.max, i32::MAX as i64) as i32;
                u.slider_min = smin as i32;
                u.slider_max = smax as i32;
            }
            ParamType::FixSlider => {
                let (smin, smax) = row
                    .slider
                    .unwrap_or((cell_f64(&row.min, 0.0), cell_f64(&row.max, 1.0)));
                let u = &mut d.u.fd;
                u.value = fixed(def_f);
                u.dephault = fixed(def_f);
                u.valid_min = fixed(cell_f64(&row.min, -32768.0));
                u.valid_max = fixed(cell_f64(&row.max, 32767.0));
                u.slider_min = fixed(smin);
                u.slider_max = fixed(smax);
                u.precision = row.precision;
                u.display_flags = row.display;
            }
            ParamType::Angle => {
                let u = &mut d.u.ad;
                u.value = fixed(def_f);
                u.dephault = fixed(def_f);
                u.valid_min = fixed(cell_f64(&row.min, 0.0));
                u.valid_max = fixed(cell_f64(&row.max, 0.0));
            }
            ParamType::Checkbox => {
                let u = &mut d.u.bd;
                u.value = (def_i != 0) as i32;
                u.dephault = (def_i != 0) as u8;
                u.nameptr = cstr(keep, &row.label);
            }
            ParamType::Color => {
                let v = def_i as u32;
                let argb = [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8];
                d.u.cd.value = argb;
                d.u.cd.dephault = argb;
            }
            ParamType::Point => {
                let (x, y) = row.xy.unwrap_or((0.0, 0.0));
                let u = &mut d.u.td;
                u.x_value = fixed(x);
                u.y_value = fixed(y);
                u.x_dephault = fixed(x);
                u.y_dephault = fixed(y);
                u.restrict_bounds = row.restrict as u8;
            }
            ParamType::Popup => {
                let list = row.choices_for(engine_flags);
                let n = list.len() as i64;
                // Version 1 tables: the default is a 1 based index kept inside the shown list.
                // Version 2 tables: the stated default is written as it is.
                let pick = if row.clamp_default {
                    if n == 0 {
                        0
                    } else {
                        def_i.clamp(1, n)
                    }
                } else {
                    def_i
                };
                let u = &mut d.u.pd;
                u.num_choices = n as i16;
                u.value = pick as i32;
                u.dephault = pick as i16;
                u.namesptr = cstr(keep, &list.join("|"));
            }
            ParamType::FloatSlider => {
                let (smin, smax) = row
                    .slider
                    .unwrap_or((cell_f64(&row.min, 0.0), cell_f64(&row.max, 1.0)));
                let u = &mut d.u.fs_d;
                u.value = def_f;
                u.dephault = def_f as f32;
                u.valid_min = cell_f64(&row.min, f32::MIN as f64) as f32;
                u.valid_max = cell_f64(&row.max, f32::MAX as f64) as f32;
                u.slider_min = smin as f32;
                u.slider_max = smax as f32;
                u.precision = row.precision;
                u.display_flags = row.display;
                u.fs_flags = row.fs_flags;
                u.curve_tolerance = row.tolerance;
            }
            ParamType::ArbitraryData => {
                let u = &mut d.u.arb_d;
                if let Some(id) = row.id {
                    u.id = id as i16;
                }
                u.dephault = handle;
                u.value = core::ptr::null_mut();
                u.refcon = opts.refcon as usize as *mut core::ffi::c_void;
            }
            ParamType::Path => d.u.path_d.dephault = def_i as i32,
            ParamType::Button => d.u.button_d.namesptr = cstr(keep, &row.label),
            ParamType::Point3d => {
                let u = &mut d.u.point3d_d;
                u.x_value = def_f;
                u.x_dephault = def_f;
            }
            ParamType::Custom | ParamType::NoData | ParamType::GroupStart | ParamType::GroupEnd => {
            }
        }
    }
}

/// The default handle of an arbitrary data parameter, through the host handle suite: acquire the
/// suite, make a handle of the size of the stated bytes (and copy them in when there are any),
/// release the suite. A host that refuses the suite or lacks a needed member gives
/// `ERR_BAD_CALLBACK_PARAM`.
///
/// # Safety
/// The pica basic suite pointer of `inp` must be null or valid.
unsafe fn default_handle(inp: &InData, data: &[u8]) -> Result<*mut core::ffi::c_void, PfErr> {
    if inp.pica_basic.is_null() {
        return Err(ERR_BAD_CALLBACK_PARAM);
    }
    let basic = &*inp.pica_basic;
    let acquire = basic.acquire_suite.ok_or(ERR_BAD_CALLBACK_PARAM)?;
    let mut table: *const core::ffi::c_void = core::ptr::null();
    let e = acquire(HANDLE_SUITE_NAME.as_ptr(), HANDLE_SUITE_VERSION, &mut table);
    if e != 0 || table.is_null() {
        return Err(ERR_BAD_CALLBACK_PARAM);
    }
    let suite = &*(table as *const HandleSuite1);
    let made = (|| {
        let new = suite.new_handle.ok_or(ERR_BAD_CALLBACK_PARAM)?;
        let h = new(data.len() as u64);
        if !data.is_empty() && !h.is_null() {
            let lock = suite.lock_handle.ok_or(ERR_BAD_CALLBACK_PARAM)?;
            let unlock = suite.unlock_handle.ok_or(ERR_BAD_CALLBACK_PARAM)?;
            let p = lock(h) as *mut u8;
            if !p.is_null() {
                core::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
            }
            unlock(h);
        }
        Ok(h)
    })();
    if let Some(release) = basic.release_suite {
        release(HANDLE_SUITE_NAME.as_ptr(), HANDLE_SUITE_VERSION);
    }
    made
}

/// Sets the options button name through the effect ui suite. Every refusal is tolerated: only a
/// non null suite table leads to the name call and the release. Skipped in Premiere.
///
/// # Safety
/// The pica basic suite pointer of `inp` must be null or valid.
unsafe fn set_options_button(inp: &InData, text: &str) {
    if inp.appl_id == APPL_ID_PREMIERE || inp.pica_basic.is_null() {
        return;
    }
    let basic = &*inp.pica_basic;
    let Some(acquire) = basic.acquire_suite else {
        return;
    };
    let mut table: *const core::ffi::c_void = core::ptr::null();
    let _ = acquire(
        EFFECT_UI_SUITE_NAME.as_ptr(),
        EFFECT_UI_SUITE_VERSION,
        &mut table,
    );
    if table.is_null() {
        return;
    }
    let suite = &*(table as *const EffectUiSuite1);
    if let Some(set) = suite.set_options_button_name {
        let c = CString::new(text.replace('\0', "")).expect("no NUL left");
        let _ = set(inp.effect_ref, c.as_ptr() as *const u8);
    }
    if let Some(release) = basic.release_suite {
        release(EFFECT_UI_SUITE_NAME.as_ptr(), EFFECT_UI_SUITE_VERSION);
    }
}

/// GLOBAL_SETUP: my_version, out_flags and out_flags2 from the plan.
pub fn global_setup(plan: &Plan, out: &mut OutData) -> PfErr {
    out.my_version = plan.globals.my_version;
    out.out_flags = plan.globals.out_flags;
    out.out_flags2 = plan.globals.out_flags2;
    ERR_NONE
}

/// ABOUT: writes the about text into return_msg.
pub fn about(plan: &Plan, out: &mut OutData) -> PfErr {
    copy_text(&mut out.return_msg, &plan.globals.about);
    ERR_NONE
}

/// PARAMS_SETUP, in this order for each row: the effect ui suite step when the row asks for it;
/// for an arbitrary data row the handle suite round trip; one add_param with index -1 on the
/// definition structure (refilled first in reset mode `each`). A nonzero host code stops the setup
/// and is returned unchanged, except for a row marked `defer_err`: its first code is kept, the
/// setup goes on, and the kept code is returned at the end without the final steps. After the last
/// row the custom ui registration (when the plan has one) is called, num_params is written (rows
/// plus the input layer) and the registration code is returned.
///
/// # Safety
/// `in_data` must point to a valid record whose callbacks (if set) are callable.
pub unsafe fn params_setup(
    plan: &Plan,
    in_data: &InData,
    out: &mut OutData,
    engine_flags: u64,
) -> PfErr {
    let add = match in_data.inter.add_param {
        Some(f) => f,
        None => return ERR_BAD_CALLBACK_PARAM,
    };
    let opts = &plan.options;
    let mut def = ParamDef::zeroed();
    fill_def(&mut def, opts.fill);
    let mut keep: Keep = Vec::new();
    let mut deferred = ERR_NONE;
    for row in &plan.rows {
        if let Some(text) = &row.ui_suite {
            set_options_button(in_data, text);
        }
        let placeholder = row.ae_only && in_data.appl_id != APPL_ID_AE;
        let handle = if row.ty == ParamType::ArbitraryData && !placeholder {
            match default_handle(in_data, &row.data) {
                Ok(h) => h,
                Err(e) => return e,
            }
        } else {
            core::ptr::null_mut()
        };
        if opts.reset == Reset::Each {
            fill_def(&mut def, opts.fill);
        }
        apply_row(
            &mut def,
            &mut keep,
            row,
            engine_flags,
            in_data.appl_id,
            opts,
            handle,
        );
        let err = add(in_data.effect_ref, -1, &mut def);
        if err != ERR_NONE {
            if !row.defer_err {
                return err;
            }
            if deferred == ERR_NONE {
                deferred = err;
            }
        }
    }
    if deferred != ERR_NONE {
        return deferred;
    }
    let mut code = ERR_NONE;
    if let Some(events) = opts.register_ui {
        let mut info = CustomUiInfo {
            events,
            ..CustomUiInfo::default()
        };
        code = match in_data.inter.register_ui {
            Some(f) => f(in_data.effect_ref, &mut info),
            None => ERR_BAD_CALLBACK_PARAM,
        };
    }
    out.num_params = plan.rows.len() as i32 + 1;
    code
}
