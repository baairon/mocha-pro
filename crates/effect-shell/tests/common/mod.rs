//! A mock host written for the tests (made up data). It logs every host call in order as text,
//! keeps a copy of every definition handed to add_param and can be told to refuse things.
#![allow(dead_code)]

use effect_shell::layout::*;
use std::cell::RefCell;
use std::ffi::{c_void, CStr};

#[derive(Clone, Default)]
pub struct Cfg {
    /// The basic suite refuses the handle suite with this code (and a null table).
    pub handle_acquire_err: i32,
    /// The basic suite answers success but a null table for the handle suite.
    pub handle_null_table: bool,
    /// The handle suite has no lock member.
    pub no_lock: bool,
    /// The effect ui suite is refused (null table, code 5).
    pub refuse_ui: bool,
    /// add_param call number (1 based) that fails, with the code.
    pub add_fail: Vec<(usize, i32)>,
    pub register_ui_err: i32,
    pub set_name_err: i32,
    /// new_handle returns null.
    pub null_handle: bool,
    /// lock_handle returns null.
    pub null_lock: bool,
}

thread_local! {
    pub static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    pub static DEFS: RefCell<Vec<ParamDef>> = const { RefCell::new(Vec::new()) };
    pub static CFG: RefCell<Cfg> = RefCell::new(Cfg::default());
    pub static TEXTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    pub static HANDLES: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    pub static UI_INFO: RefCell<Vec<CustomUiInfo>> = const { RefCell::new(Vec::new()) };
}

pub fn log(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}

pub fn events() -> Vec<String> {
    LOG.with(|l| l.borrow().clone())
}

pub fn defs() -> Vec<ParamDef> {
    DEFS.with(|d| d.borrow().clone())
}

pub fn texts() -> Vec<String> {
    TEXTS.with(|d| d.borrow().clone())
}

pub fn handle_bytes(h: usize) -> Vec<u8> {
    let i = h - 1;
    HANDLES.with(|v| v.borrow()[i].clone())
}

pub fn reset(cfg: Cfg) {
    LOG.with(|l| l.borrow_mut().clear());
    DEFS.with(|l| l.borrow_mut().clear());
    TEXTS.with(|l| l.borrow_mut().clear());
    HANDLES.with(|l| l.borrow_mut().clear());
    UI_INFO.with(|l| l.borrow_mut().clear());
    CFG.with(|c| *c.borrow_mut() = cfg);
}

pub unsafe fn text_of(p: *const u8) -> String {
    if p.is_null() {
        "<null>".into()
    } else {
        CStr::from_ptr(p as *const _).to_string_lossy().into_owned()
    }
}

unsafe extern "C-unwind" fn add_param(r: *mut c_void, index: i32, def: *mut ParamDef) -> PfErr {
    let n = DEFS.with(|d| {
        d.borrow_mut().push(*def);
        d.borrow().len()
    });
    let text = match (*def).param_type {
        x if x == ParamType::Popup as i32 => text_of((*def).u.pd.namesptr),
        x if x == ParamType::Checkbox as i32 => text_of((*def).u.bd.nameptr),
        x if x == ParamType::Button as i32 => text_of((*def).u.button_d.namesptr),
        _ => String::new(),
    };
    TEXTS.with(|t| t.borrow_mut().push(text));
    log(format!(
        "add_param ref={:#x} index={} id={}",
        r as usize,
        index,
        (*def).uu_id
    ));
    CFG.with(|c| {
        c.borrow()
            .add_fail
            .iter()
            .find(|(at, _)| *at == n)
            .map_or(0, |(_, code)| *code)
    })
}

unsafe extern "C" fn register_ui(r: *mut c_void, info: *mut CustomUiInfo) -> PfErr {
    UI_INFO.with(|u| u.borrow_mut().push(*info));
    log(format!(
        "register_ui ref={:#x} events={}",
        r as usize,
        (*info).events
    ));
    CFG.with(|c| c.borrow().register_ui_err)
}

unsafe extern "C" fn acquire(name: *const u8, version: i32, out: *mut *const c_void) -> i32 {
    let n = text_of(name);
    log(format!("acquire {n} {version}"));
    let cfg = CFG.with(|c| c.borrow().clone());
    if n == "PF Handle Suite" {
        if cfg.handle_acquire_err != 0 {
            *out = std::ptr::null();
            return cfg.handle_acquire_err;
        }
        if cfg.handle_null_table {
            *out = std::ptr::null();
            return 0;
        }
        *out = &HANDLE_SUITE as *const _ as *const c_void;
        return 0;
    }
    if n == "PF Effect UI Suite" {
        if cfg.refuse_ui {
            *out = std::ptr::null();
            return 5;
        }
        *out = &UI_SUITE as *const _ as *const c_void;
        return 0;
    }
    *out = std::ptr::null();
    7
}

unsafe extern "C" fn release(name: *const u8, version: i32) -> i32 {
    log(format!("release {} {}", text_of(name), version));
    0
}

unsafe extern "C" fn new_handle(size: u64) -> *mut c_void {
    log(format!("new_handle {size}"));
    if CFG.with(|c| c.borrow().null_handle) {
        return std::ptr::null_mut();
    }
    HANDLES.with(|h| {
        h.borrow_mut().push(vec![0u8; size as usize]);
        h.borrow().len() as *mut c_void
    })
}

unsafe extern "C" fn lock_handle(h: *mut c_void) -> *mut c_void {
    log(format!("lock {}", h as usize));
    if CFG.with(|c| c.borrow().null_lock) {
        return std::ptr::null_mut();
    }
    HANDLES.with(|v| v.borrow_mut()[h as usize - 1].as_mut_ptr() as *mut c_void)
}

unsafe extern "C" fn unlock_handle(h: *mut c_void) {
    log(format!("unlock {}", h as usize));
}

unsafe extern "C" fn set_name(r: *mut c_void, text: *const u8) -> PfErr {
    log(format!(
        "set_options_name ref={:#x} {:?}",
        r as usize,
        text_of(text)
    ));
    CFG.with(|c| c.borrow().set_name_err)
}

pub static HANDLE_SUITE: HandleSuite1 = HandleSuite1 {
    new_handle: Some(new_handle),
    lock_handle: Some(lock_handle),
    unlock_handle: Some(unlock_handle),
    dispose_handle: None,
    get_handle_size: None,
    resize_handle: None,
};
pub static UI_SUITE: EffectUiSuite1 = EffectUiSuite1 {
    set_options_button_name: Some(set_name),
};
pub static BASIC: SpBasicSuite = SpBasicSuite {
    acquire_suite: Some(acquire),
    release_suite: Some(release),
    is_equal: None,
    allocate_block: None,
    free_block: None,
    reallocate_block: None,
    undefined: None,
};

/// A host record with every callback the mock answers. The add_param callback is the mock's.
pub fn host(appl: u32) -> InData {
    let mut i: InData = unsafe { std::mem::zeroed() };
    i.inter.add_param = Some(add_param);
    i.inter.register_ui = Some(register_ui);
    i.effect_ref = 0x1234 as *mut c_void;
    i.appl_id = appl;
    i.pica_basic = &BASIC as *const SpBasicSuite as *mut SpBasicSuite;
    i
}

/// Gives a host record the mock suites only (the caller keeps its own add_param).
pub fn install_suites(i: &mut InData) {
    i.pica_basic = &BASIC as *const SpBasicSuite as *mut SpBasicSuite;
    i.inter.register_ui = Some(register_ui);
}

pub fn out() -> OutData {
    unsafe { std::mem::zeroed() }
}

pub fn bytes_of(d: &ParamDef) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(
            d as *const ParamDef as *const u8,
            std::mem::size_of::<ParamDef>(),
        )
        .to_vec()
    }
}
