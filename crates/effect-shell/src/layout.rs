//! Hand written repr(C) definitions of the host's plug-in interface structures, under this crate's
//! own names. The sizes, member offsets and constant values stated here are public interface facts
//! of the host's 64 bit plug-in interface, written by this project for interoperability; the tests
//! pin every size and offset. No text or file of the host's SDK is included.

use core::ffi::c_void;

pub type PfErr = i32;
pub const ERR_NONE: PfErr = 0;
pub const ERR_INTERNAL_STRUCT_DAMAGED: PfErr = 512;
pub const ERR_UNRECOGNIZED_PARAM_TYPE: PfErr = 514;
pub const ERR_BAD_CALLBACK_PARAM: PfErr = 516;

/// Command numbers.
pub const CMD_ABOUT: i32 = 0;
pub const CMD_GLOBAL_SETUP: i32 = 1;
pub const CMD_PARAMS_SETUP: i32 = 4;

/// The host application id of After Effects as a four character code.
pub const APPL_ID_AE: u32 = u32::from_be_bytes(*b"FXTC");
/// The host application id of Premiere as a four character code.
pub const APPL_ID_PREMIERE: u32 = u32::from_be_bytes(*b"PrMr");

/// Suite names and versions the shell requests (public SDK facts).
pub const HANDLE_SUITE_NAME: &[u8] = b"PF Handle Suite\0";
pub const HANDLE_SUITE_VERSION: i32 = 2;
pub const EFFECT_UI_SUITE_NAME: &[u8] = b"PF Effect UI Suite\0";
pub const EFFECT_UI_SUITE_VERSION: i32 = 1;

/// Parameter ui flags used by the shell.
pub const PUI_INVISIBLE: u32 = 512;

/// Parameter flag words the shell sets: the host supervises changes of the parameter.
pub const PARAM_FLAG_SUPERVISE: u32 = 0x40;
/// Parameter flag words the shell sets: the group starts collapsed.
pub const PARAM_FLAG_START_COLLAPSED: u32 = 0x20;
/// Parameter flag words the shell sets: the parameter cannot vary over time.
pub const PARAM_FLAG_CANNOT_TIME_VARY: u32 = 0x2;

/// Parameter types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ParamType {
    Layer = 0,
    Slider = 1,
    FixSlider = 2,
    Angle = 3,
    Checkbox = 4,
    Color = 5,
    Point = 6,
    Popup = 7,
    Custom = 8,
    NoData = 9,
    FloatSlider = 10,
    ArbitraryData = 11,
    Path = 12,
    GroupStart = 13,
    GroupEnd = 14,
    Button = 15,
    Point3d = 18,
}

/// The ABI is C-unwind so a panic in a host callback can be caught by the entry point (tests use this).
pub type AddParamFn = unsafe extern "C-unwind" fn(*mut c_void, i32, *mut ParamDef) -> PfErr;
pub type RegisterUiFn = unsafe extern "C" fn(*mut c_void, *mut CustomUiInfo) -> PfErr;
pub type AcquireSuiteFn = unsafe extern "C" fn(*const u8, i32, *mut *const c_void) -> i32;
pub type ReleaseSuiteFn = unsafe extern "C" fn(*const u8, i32) -> i32;
pub type NewHandleFn = unsafe extern "C" fn(u64) -> *mut c_void;
pub type LockHandleFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type UnlockHandleFn = unsafe extern "C" fn(*mut c_void);
pub type SetOptionsButtonNameFn = unsafe extern "C" fn(*mut c_void, *const u8) -> PfErr;

/// The plug-in manager basic suite (56 bytes); only the first two members are used.
#[repr(C)]
pub struct SpBasicSuite {
    pub acquire_suite: Option<AcquireSuiteFn>,
    pub release_suite: Option<ReleaseSuiteFn>,
    pub is_equal: Option<unsafe extern "C" fn()>,
    pub allocate_block: Option<unsafe extern "C" fn()>,
    pub free_block: Option<unsafe extern "C" fn()>,
    pub reallocate_block: Option<unsafe extern "C" fn()>,
    pub undefined: Option<unsafe extern "C" fn()>,
}

/// The handle suite (48 bytes).
#[repr(C)]
pub struct HandleSuite1 {
    pub new_handle: Option<NewHandleFn>,
    pub lock_handle: Option<LockHandleFn>,
    pub unlock_handle: Option<UnlockHandleFn>,
    pub dispose_handle: Option<unsafe extern "C" fn()>,
    pub get_handle_size: Option<unsafe extern "C" fn()>,
    pub resize_handle: Option<unsafe extern "C" fn()>,
}

/// The effect ui suite (8 bytes).
#[repr(C)]
pub struct EffectUiSuite1 {
    pub set_options_button_name: Option<SetOptionsButtonNameFn>,
}

/// The custom ui registration record (44 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CustomUiInfo {
    pub reserved: i32,
    pub events: u32,
    pub comp_ui_width: i32,
    pub comp_ui_height: i32,
    pub comp_ui_alignment: i32,
    pub layer_ui_width: i32,
    pub layer_ui_height: i32,
    pub layer_ui_alignment: i32,
    pub preview_ui_width: i32,
    pub preview_ui_height: i32,
    pub preview_ui_alignment: i32,
}

/// The callback table at the start of the in data record (176 bytes).
#[repr(C)]
pub struct InteractCallbacks {
    pub checkout_param: Option<unsafe extern "C" fn()>,
    pub checkin_param: Option<unsafe extern "C" fn()>,
    pub add_param: Option<AddParamFn>,
    pub abort: Option<unsafe extern "C" fn()>,
    pub progress: Option<unsafe extern "C" fn()>,
    pub register_ui: Option<RegisterUiFn>,
    pub checkout_layer_audio: Option<unsafe extern "C" fn()>,
    pub checkin_layer_audio: Option<unsafe extern "C" fn()>,
    pub get_audio_data: Option<unsafe extern "C" fn()>,
    pub reserved_str: [*mut c_void; 3],
    pub reserved: [*mut c_void; 10],
}

/// The sound world record (32 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SoundWorld {
    pub rate: f64,
    pub num_channels: i16,
    pub format: i16,
    pub sample_size: i16,
    pub num_samples: i32,
    pub data: *mut c_void,
}

/// The host to plug-in record (408 bytes).
#[repr(C)]
pub struct InData {
    pub inter: InteractCallbacks,
    pub utils: *mut c_void,
    pub effect_ref: *mut c_void,
    pub quality: i32,
    pub version_major: i16,
    pub version_minor: i16,
    pub serial_num: i32,
    pub appl_id: u32,
    pub num_params: i32,
    pub reserved: i32,
    pub what_cpu: i32,
    pub what_fpu: i32,
    pub current_time: i32,
    pub time_step: i32,
    pub total_time: i32,
    pub local_time_step: i32,
    pub time_scale: u32,
    pub field: i32,
    pub shutter_angle: i32,
    pub width: i32,
    pub height: i32,
    pub extent_hint: [i32; 4],
    pub output_origin_x: i32,
    pub output_origin_y: i32,
    pub downsample_x: [i32; 2],
    pub downsample_y: [i32; 2],
    pub pixel_aspect_ratio: [i32; 2],
    pub in_flags: i32,
    pub global_data: *mut c_void,
    pub sequence_data: *mut c_void,
    pub frame_data: *mut c_void,
    pub start_samp: i32,
    pub dur_samp: i32,
    pub total_samp: i32,
    pub src_snd: SoundWorld,
    pub pica_basic: *mut SpBasicSuite,
    pub pre_effect_source_origin_x: i32,
    pub pre_effect_source_origin_y: i32,
    pub shutter_phase: i32,
}

/// The plug-in to host record (408 bytes).
#[repr(C)]
pub struct OutData {
    pub my_version: u32,
    pub name: [u8; 32],
    pub global_data: *mut c_void,
    pub num_params: i32,
    pub sequence_data: *mut c_void,
    pub flat_sdata_size: i32,
    pub frame_data: *mut c_void,
    pub width: i32,
    pub height: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub out_flags: u32,
    pub return_msg: [u8; 256],
    pub start_samp: i32,
    pub dur_samp: i32,
    pub dest_snd: SoundWorld,
    pub out_flags2: u32,
}

/// The layer parameter member of the union (120 bytes; only the default is used).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LayerDef {
    pub pad0: [u8; 116],
    pub dephault: i32,
}
/// The integer slider member (88 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SliderDef {
    pub value: i32,
    pub value_str: [u8; 32],
    pub value_desc: [u8; 32],
    pub valid_min: i32,
    pub valid_max: i32,
    pub slider_min: i32,
    pub slider_max: i32,
    pub dephault: i32,
}
/// The fixed point slider member (92 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FixedSliderDef {
    pub value: i32,
    pub value_str: [u8; 32],
    pub value_desc: [u8; 32],
    pub valid_min: i32,
    pub valid_max: i32,
    pub slider_min: i32,
    pub slider_max: i32,
    pub dephault: i32,
    pub precision: i16,
    pub display_flags: i16,
}
/// The angle member (16 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AngleDef {
    pub value: i32,
    pub dephault: i32,
    pub valid_min: i32,
    pub valid_max: i32,
}
/// The checkbox member (16 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CheckBoxDef {
    pub value: i32,
    pub dephault: u8,
    pub reserved: u8,
    pub reserved1: i16,
    pub nameptr: *const u8,
}
/// The color member (8 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ColorDef {
    /// alpha, red, green, blue
    pub value: [u8; 4],
    pub dephault: [u8; 4],
}
/// The point member (20 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PointDef {
    pub x_value: i32,
    pub y_value: i32,
    pub reserved: [u8; 3],
    pub restrict_bounds: u8,
    pub x_dephault: i32,
    pub y_dephault: i32,
}
/// The popup member (16 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PopupDef {
    pub value: i32,
    pub num_choices: i16,
    pub dephault: i16,
    pub namesptr: *const u8,
}
/// The floating point slider member (88 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FloatSliderDef {
    pub value: f64,
    pub phase: f64,
    pub value_desc: [u8; 32],
    pub valid_min: f32,
    pub valid_max: f32,
    pub slider_min: f32,
    pub slider_max: f32,
    pub dephault: f32,
    pub precision: i16,
    pub display_flags: i16,
    pub fs_flags: i32,
    pub curve_tolerance: f32,
    pub use_exponent: u8,
    pub exponent: f32,
}
/// The arbitrary data member (32 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbitraryDef {
    pub id: i16,
    pub pad: i16,
    pub dephault: *mut c_void,
    pub value: *mut c_void,
    pub refcon: *mut c_void,
}
/// The path member (12 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PathDef {
    pub path_id: i32,
    pub reserved0: i32,
    pub dephault: i32,
}
/// The button member (16 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ButtonDef {
    pub value: i32,
    pub namesptr: *const u8,
}
/// The three dimensional point member (64 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Point3dDef {
    pub x_value: f64,
    pub y_value: f64,
    pub z_value: f64,
    pub x_dephault: f64,
    pub y_dephault: f64,
    pub z_dephault: f64,
    pub reserved: [u8; 16],
}

/// The union at the end of a parameter definition (120 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub union ParamUnion {
    pub raw: [u8; 120],
    pub ld: LayerDef,
    pub sd: SliderDef,
    pub fd: FixedSliderDef,
    pub ad: AngleDef,
    pub bd: CheckBoxDef,
    pub cd: ColorDef,
    pub td: PointDef,
    pub pd: PopupDef,
    pub fs_d: FloatSliderDef,
    pub arb_d: ArbitraryDef,
    pub path_d: PathDef,
    pub button_d: ButtonDef,
    pub point3d_d: Point3dDef,
}

/// One parameter definition (176 bytes). `uu_id` is the id slot at offset 0.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ParamDef {
    pub uu_id: i32,
    pub ui_flags: u32,
    pub ui_width: i16,
    pub ui_height: i16,
    pub param_type: i32,
    pub name: [u8; 32],
    pub flags: u32,
    pub unused: i32,
    pub u: ParamUnion,
}

impl ParamDef {
    pub fn zeroed() -> ParamDef {
        // SAFETY: every member is an integer, a float or a raw pointer, for which all zero bits is valid.
        unsafe { core::mem::zeroed() }
    }
}
