//! Hand written repr(C) definitions of the host structures the render commands use, under this
//! crate's own names. The sizes, member offsets, slot indices and constant values stated here are
//! public interface facts of the host's 64 bit plug-in interface, written by this project for
//! interoperability; the tests pin every size and offset. No text or file of the host's SDK is
//! included. Nothing here belongs to a particular plug-in.

use crate::layout::{InData, InteractCallbacks, ParamDef, PfErr};
use core::ffi::c_void;

/// Error codes used by render units.
pub const ERR_OUT_OF_MEMORY: PfErr = 4;
pub const ERR_INTERRUPT_CANCEL: PfErr = 517;

/// Command numbers (public SDK order).
pub const CMD_GLOBAL_SETDOWN: i32 = 3;
pub const CMD_SEQUENCE_SETUP: i32 = 5;
pub const CMD_SEQUENCE_SETDOWN: i32 = 8;
pub const CMD_RENDER: i32 = 11;
pub const CMD_SMART_PRE_RENDER: i32 = 23;
pub const CMD_SMART_RENDER: i32 = 24;

/// The render output flag that marks an output with extra pixels.
pub const RENDER_OUTPUT_RETURNS_EXTRA_PIXELS: u16 = 1;

/// A rectangle (16 bytes): left, top, right, bottom.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    /// Empty when left is not below right or top is not below bottom.
    pub fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Union rule: an empty source leaves the target, an empty target takes the source, else the
    /// smallest left and top and the largest right and bottom.
    pub fn union_into(&mut self, src: &Rect) {
        if src.is_empty() {
            return;
        }
        if self.is_empty() {
            *self = *src;
            return;
        }
        self.left = self.left.min(src.left);
        self.top = self.top.min(src.top);
        self.right = self.right.max(src.right);
        self.bottom = self.bottom.max(src.bottom);
    }
}

/// The host's render request structure (44 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderRequest {
    pub rect: Rect,
    pub field: i32,
    pub channel_mask: i32,
    pub preserve_rgb_of_zero_alpha: u8,
    pub unused: [u8; 3],
    pub reserved: [i32; 4],
}

/// The host's pre render input structure (64 bytes).
#[repr(C)]
pub struct PreRenderInput {
    pub output_request: RenderRequest,
    pub bitdepth: i16,
    pub pad0: [u8; 2],
    pub gpu_data: *mut c_void,
    pub what_gpu: i32,
    pub device_index: u32,
}

/// The host's pre render output structure (56 bytes).
#[repr(C)]
pub struct PreRenderOutput {
    pub result_rect: Rect,
    pub max_result_rect: Rect,
    pub solid: u8,
    pub reserved: u8,
    pub flags: u16,
    pub pad: u32,
    pub pre_render_data: *mut c_void,
    pub delete_pre_render_data_func: *mut c_void,
}

/// The host's checkout result structure (76 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CheckoutResult {
    pub result_rect: Rect,
    pub max_result_rect: Rect,
    pub par_num: i32,
    pub par_den: u32,
    pub solid: u8,
    pub reserved_b: [u8; 3],
    pub ref_width: i32,
    pub ref_height: i32,
    pub reserved: [i32; 6],
}

pub type CheckoutLayerFn = unsafe extern "C" fn(
    *mut c_void,
    i32,
    i32,
    *const RenderRequest,
    i32,
    i32,
    u32,
    *mut CheckoutResult,
) -> PfErr;

/// The host's pre render callbacks structure (16 bytes).
#[repr(C)]
pub struct PreRenderCallbacks {
    pub checkout_layer: Option<CheckoutLayerFn>,
    pub guid_mix_in_ptr: *mut c_void,
}

/// The host's pre render extra structure (24 bytes).
#[repr(C)]
pub struct PreRenderExtra {
    pub input: *mut PreRenderInput,
    pub output: *mut PreRenderOutput,
    pub cb: *mut PreRenderCallbacks,
}

/// The host's smart render input structure (72 bytes).
#[repr(C)]
pub struct SmartRenderInput {
    pub output_request: RenderRequest,
    pub bitdepth: i16,
    pub pad0: [u8; 2],
    pub pre_render_data: *mut c_void,
    pub gpu_data: *mut c_void,
    pub what_gpu: i32,
    pub device_index: u32,
}

pub type CheckoutLayerPixelsFn = unsafe extern "C" fn(*mut c_void, i32, *mut *mut World) -> PfErr;
pub type CheckinLayerPixelsFn = unsafe extern "C" fn(*mut c_void, i32) -> PfErr;
pub type CheckoutOutputFn = unsafe extern "C" fn(*mut c_void, *mut *mut World) -> PfErr;

/// The host's smart render callbacks structure (24 bytes).
#[repr(C)]
pub struct SmartRenderCallbacks {
    pub checkout_layer_pixels: Option<CheckoutLayerPixelsFn>,
    pub checkin_layer_pixels: Option<CheckinLayerPixelsFn>,
    pub checkout_output: Option<CheckoutOutputFn>,
}

/// The host's smart render extra structure (16 bytes).
#[repr(C)]
pub struct SmartRenderExtra {
    pub input: *mut SmartRenderInput,
    pub cb: *mut SmartRenderCallbacks,
}

/// The host's world structure (120 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct World {
    pub reserved0: *mut c_void,
    pub reserved1: *mut c_void,
    pub world_flags: u32,
    pub pad0: u32,
    pub data: *mut c_void,
    pub rowbytes: i32,
    pub width: i32,
    pub height: i32,
    pub extent_hint: Rect,
    pub pad1: u32,
    pub platform_ref: *mut c_void,
    pub reserved_long1: i32,
    pub pad2: u32,
    pub reserved_long4: *mut c_void,
    pub pix_aspect_num: i32,
    pub pix_aspect_den: u32,
    pub reserved_long2: *mut c_void,
    pub origin_x: i32,
    pub origin_y: i32,
    pub reserved_long3: i32,
    pub dephault: i32,
}

impl World {
    pub fn zeroed() -> World {
        // SAFETY: integers and raw pointers only; all zero bits is valid.
        unsafe { core::mem::zeroed() }
    }
}

pub type CheckoutParamFn =
    unsafe extern "C" fn(*mut c_void, i32, i32, i32, u32, *mut ParamDef) -> PfErr;
pub type CheckinParamFn = unsafe extern "C" fn(*mut c_void, *mut ParamDef) -> PfErr;

impl InteractCallbacks {
    /// The checkout_param callback with its real signature.
    pub fn typed_checkout_param(&self) -> Option<CheckoutParamFn> {
        // SAFETY: both are optional C function pointers of the same size.
        unsafe {
            core::mem::transmute::<Option<unsafe extern "C" fn()>, Option<CheckoutParamFn>>(
                self.checkout_param,
            )
        }
    }

    /// The checkin_param callback with its real signature.
    pub fn typed_checkin_param(&self) -> Option<CheckinParamFn> {
        // SAFETY: as above.
        unsafe {
            core::mem::transmute::<Option<unsafe extern "C" fn()>, Option<CheckinParamFn>>(
                self.checkin_param,
            )
        }
    }
}

/// The utility callback table (552 bytes, 69 pointer slots); members are read by slot.
#[repr(C)]
pub struct UtilCallbacks {
    pub slots: [*const c_void; 69],
}

pub const UTIL_SLOT_COPY: usize = 8;
pub const UTIL_SLOT_GET_PLATFORM_DATA: usize = 54;

pub type UtilCopyFn =
    unsafe extern "C" fn(*mut c_void, *mut World, *mut World, *mut Rect, *mut Rect) -> PfErr;
pub type UtilGetPlatformDataFn = unsafe extern "C" fn(*mut c_void, i32, *mut c_void) -> PfErr;

impl UtilCallbacks {
    /// The copy member (source, destination, source rect, destination rect).
    ///
    /// # Safety
    /// `self` must be a valid table.
    pub unsafe fn copy(&self) -> Option<UtilCopyFn> {
        slot_fn::<UtilCopyFn>(self.slots[UTIL_SLOT_COPY])
    }

    /// The get_platform_data member (effect ref, which, out pointer).
    ///
    /// # Safety
    /// `self` must be a valid table.
    pub unsafe fn get_platform_data(&self) -> Option<UtilGetPlatformDataFn> {
        slot_fn::<UtilGetPlatformDataFn>(self.slots[UTIL_SLOT_GET_PLATFORM_DATA])
    }
}

/// The handle suite with every member typed (48 bytes).
#[repr(C)]
pub struct HandleSuiteFull {
    pub new_handle: Option<unsafe extern "C" fn(u64) -> *mut c_void>,
    pub lock_handle: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
    pub unlock_handle: Option<unsafe extern "C" fn(*mut c_void)>,
    pub dispose_handle: Option<unsafe extern "C" fn(*mut c_void)>,
    pub get_handle_size: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
    pub resize_handle: Option<unsafe extern "C" fn(u64, *mut c_void) -> i32>,
}

/// Reinterprets a raw pointer read from a table as an optional function pointer.
///
/// # Safety
/// `F` must be a C function pointer type and `p` null or a function of that type.
pub unsafe fn slot_fn<F: Copy>(p: *const c_void) -> Option<F> {
    debug_assert_eq!(
        core::mem::size_of::<F>(),
        core::mem::size_of::<*const c_void>()
    );
    if p.is_null() {
        None
    } else {
        Some(core::mem::transmute_copy::<*const c_void, F>(&p))
    }
}

/// Reads member `slot` (pointer sized) of a suite table as a function pointer.
///
/// # Safety
/// `suite` must point to a table with at least `slot + 1` pointer sized members.
pub unsafe fn suite_slot<F: Copy>(suite: *const c_void, slot: usize) -> Option<F> {
    if suite.is_null() {
        return None;
    }
    let p = *(suite as *const *const c_void).add(slot);
    slot_fn::<F>(p)
}

/// The A_Time record (8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ATime {
    pub value: i32,
    pub scale: u32,
}

/// Convenience: the in data record reference of a raw pointer.
///
/// # Safety
/// `p` must be null or valid.
pub unsafe fn in_ref<'a>(p: *mut InData) -> Option<&'a InData> {
    p.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn sizes_match_the_interface_facts() {
        assert_eq!(size_of::<Rect>(), 16);
        assert_eq!(size_of::<RenderRequest>(), 44);
        assert_eq!(size_of::<PreRenderInput>(), 64);
        assert_eq!(size_of::<PreRenderOutput>(), 56);
        assert_eq!(size_of::<CheckoutResult>(), 76);
        assert_eq!(size_of::<PreRenderCallbacks>(), 16);
        assert_eq!(size_of::<PreRenderExtra>(), 24);
        assert_eq!(size_of::<SmartRenderInput>(), 72);
        assert_eq!(size_of::<SmartRenderCallbacks>(), 24);
        assert_eq!(size_of::<SmartRenderExtra>(), 16);
        assert_eq!(size_of::<World>(), 120);
        assert_eq!(size_of::<UtilCallbacks>(), 552);
        assert_eq!(size_of::<HandleSuiteFull>(), 48);
        assert_eq!(size_of::<ATime>(), 8);
    }

    #[test]
    fn member_offsets_match_the_interface_facts() {
        use core::mem::offset_of;
        assert_eq!(offset_of!(RenderRequest, field), 16);
        assert_eq!(offset_of!(RenderRequest, channel_mask), 20);
        assert_eq!(offset_of!(RenderRequest, preserve_rgb_of_zero_alpha), 24);
        assert_eq!(offset_of!(PreRenderInput, bitdepth), 44);
        assert_eq!(offset_of!(PreRenderInput, gpu_data), 48);
        assert_eq!(offset_of!(PreRenderOutput, solid), 32);
        assert_eq!(offset_of!(PreRenderOutput, flags), 34);
        assert_eq!(offset_of!(PreRenderOutput, pre_render_data), 40);
        assert_eq!(offset_of!(CheckoutResult, par_num), 32);
        assert_eq!(offset_of!(CheckoutResult, solid), 40);
        assert_eq!(offset_of!(CheckoutResult, ref_width), 44);
        assert_eq!(offset_of!(SmartRenderInput, pre_render_data), 48);
        assert_eq!(offset_of!(SmartRenderInput, device_index), 68);
        assert_eq!(offset_of!(World, world_flags), 16);
        assert_eq!(offset_of!(World, data), 24);
        assert_eq!(offset_of!(World, rowbytes), 32);
        assert_eq!(offset_of!(World, width), 36);
        assert_eq!(offset_of!(World, extent_hint), 44);
        assert_eq!(offset_of!(World, platform_ref), 64);
        assert_eq!(offset_of!(World, pix_aspect_num), 88);
        assert_eq!(offset_of!(World, origin_x), 104);
        assert_eq!(offset_of!(World, dephault), 116);
    }

    #[test]
    fn remaining_member_offsets_match_the_interface_facts() {
        use core::mem::offset_of;
        assert_eq!(offset_of!(Rect, top), 4);
        assert_eq!(offset_of!(Rect, right), 8);
        assert_eq!(offset_of!(Rect, bottom), 12);
        assert_eq!(offset_of!(RenderRequest, rect), 0);
        assert_eq!(offset_of!(RenderRequest, unused), 25);
        assert_eq!(offset_of!(RenderRequest, reserved), 28);
        assert_eq!(offset_of!(PreRenderInput, output_request), 0);
        assert_eq!(offset_of!(PreRenderInput, what_gpu), 56);
        assert_eq!(offset_of!(PreRenderInput, device_index), 60);
        assert_eq!(offset_of!(PreRenderOutput, result_rect), 0);
        assert_eq!(offset_of!(PreRenderOutput, max_result_rect), 16);
        assert_eq!(offset_of!(PreRenderOutput, reserved), 33);
        assert_eq!(offset_of!(PreRenderOutput, delete_pre_render_data_func), 48);
        assert_eq!(offset_of!(CheckoutResult, max_result_rect), 16);
        assert_eq!(offset_of!(CheckoutResult, par_den), 36);
        assert_eq!(offset_of!(CheckoutResult, reserved_b), 41);
        assert_eq!(offset_of!(CheckoutResult, ref_height), 48);
        assert_eq!(offset_of!(CheckoutResult, reserved), 52);
        assert_eq!(offset_of!(PreRenderCallbacks, guid_mix_in_ptr), 8);
        assert_eq!(offset_of!(PreRenderExtra, output), 8);
        assert_eq!(offset_of!(PreRenderExtra, cb), 16);
        assert_eq!(offset_of!(SmartRenderInput, output_request), 0);
        assert_eq!(offset_of!(SmartRenderInput, bitdepth), 44);
        assert_eq!(offset_of!(SmartRenderInput, gpu_data), 56);
        assert_eq!(offset_of!(SmartRenderInput, what_gpu), 64);
        assert_eq!(offset_of!(SmartRenderCallbacks, checkin_layer_pixels), 8);
        assert_eq!(offset_of!(SmartRenderCallbacks, checkout_output), 16);
        assert_eq!(offset_of!(SmartRenderExtra, cb), 8);
        assert_eq!(offset_of!(World, reserved1), 8);
        assert_eq!(offset_of!(World, height), 40);
        assert_eq!(offset_of!(World, reserved_long1), 72);
        assert_eq!(offset_of!(World, reserved_long4), 80);
        assert_eq!(offset_of!(World, pix_aspect_den), 92);
        assert_eq!(offset_of!(World, reserved_long2), 96);
        assert_eq!(offset_of!(World, origin_y), 108);
        assert_eq!(offset_of!(World, reserved_long3), 112);
        assert_eq!(offset_of!(HandleSuiteFull, lock_handle), 8);
        assert_eq!(offset_of!(HandleSuiteFull, unlock_handle), 16);
        assert_eq!(offset_of!(HandleSuiteFull, dispose_handle), 24);
        assert_eq!(offset_of!(HandleSuiteFull, get_handle_size), 32);
        assert_eq!(offset_of!(HandleSuiteFull, resize_handle), 40);
        assert_eq!(offset_of!(ATime, scale), 4);
    }

    #[test]
    fn utility_table_slots_are_pinned_and_read_by_slot() {
        // Byte offsets 64 and 432 of the 552 byte table, in pointer slots of 8 bytes.
        assert_eq!(UTIL_SLOT_COPY * size_of::<*const c_void>(), 64);
        assert_eq!(
            UTIL_SLOT_GET_PLATFORM_DATA * size_of::<*const c_void>(),
            432
        );
        assert_eq!(size_of::<UtilCallbacks>() / size_of::<*const c_void>(), 69);
        unsafe extern "C" fn fake_copy(
            _: *mut c_void,
            _: *mut World,
            _: *mut World,
            _: *mut Rect,
            _: *mut Rect,
        ) -> PfErr {
            11
        }
        unsafe extern "C" fn fake_platform(_: *mut c_void, which: i32, _: *mut c_void) -> PfErr {
            20 + which
        }
        let mut t = UtilCallbacks {
            slots: [core::ptr::null(); 69],
        };
        // SAFETY: the table is local; a slot holds null or a function of the stated type.
        unsafe {
            assert!(t.copy().is_none());
            assert!(t.get_platform_data().is_none());
            t.slots[UTIL_SLOT_COPY] = fake_copy as UtilCopyFn as *const c_void;
            t.slots[UTIL_SLOT_GET_PLATFORM_DATA] =
                fake_platform as UtilGetPlatformDataFn as *const c_void;
            let np: *mut c_void = core::ptr::null_mut();
            assert_eq!(
                t.copy().unwrap()(np, np.cast(), np.cast(), np.cast(), np.cast()),
                11
            );
            assert_eq!(t.get_platform_data().unwrap()(np, 3, np), 23);
            // A neighbouring slot stays empty: the two accessors read distinct slots.
            t.slots[UTIL_SLOT_COPY] = core::ptr::null();
            assert!(t.copy().is_none());
            assert!(t.get_platform_data().is_some());
        }
    }

    #[test]
    fn typed_param_callbacks_and_in_ref() {
        unsafe extern "C" fn fake_checkout(
            _: *mut c_void,
            _: i32,
            _: i32,
            _: i32,
            _: u32,
            _: *mut ParamDef,
        ) -> PfErr {
            5
        }
        unsafe extern "C" fn fake_checkin(_: *mut c_void, _: *mut ParamDef) -> PfErr {
            6
        }
        // SAFETY: every member of the callback table is an optional function pointer or a raw
        // pointer array, for which all zero bits is valid.
        let mut cb: InteractCallbacks = unsafe { core::mem::zeroed() };
        assert!(cb.typed_checkout_param().is_none());
        assert!(cb.typed_checkin_param().is_none());
        // SAFETY: each pointer is converted to the untyped slot type and back to its own type.
        unsafe {
            cb.checkout_param = Some(core::mem::transmute::<
                CheckoutParamFn,
                unsafe extern "C" fn(),
            >(fake_checkout));
            cb.checkin_param =
                Some(core::mem::transmute::<CheckinParamFn, unsafe extern "C" fn()>(fake_checkin));
            let np: *mut c_void = core::ptr::null_mut();
            assert_eq!(
                cb.typed_checkout_param().unwrap()(np, 0, 0, 0, 0, np.cast()),
                5
            );
            assert_eq!(cb.typed_checkin_param().unwrap()(np, np.cast()), 6);
            assert!(in_ref(core::ptr::null_mut()).is_none());
            let mut d: InData = core::mem::zeroed();
            d.num_params = 9;
            assert_eq!(in_ref(&mut d).unwrap().num_params, 9);
        }
    }

    #[test]
    fn rect_union_rules() {
        let empty = Rect::default();
        let a = Rect {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
        };
        let b = Rect {
            left: 5,
            top: -2,
            right: 20,
            bottom: 8,
        };
        let mut t = empty;
        t.union_into(&a);
        assert_eq!(t, a);
        t.union_into(&empty);
        assert_eq!(t, a);
        t.union_into(&b);
        assert_eq!(
            t,
            Rect {
                left: 0,
                top: -2,
                right: 20,
                bottom: 10
            }
        );
        assert!(Rect {
            left: 3,
            top: 0,
            right: 3,
            bottom: 5
        }
        .is_empty());
        assert!(Rect {
            left: 0,
            top: 5,
            right: 4,
            bottom: 5
        }
        .is_empty());
        assert!(!a.is_empty());
    }

    #[test]
    fn slot_reads() {
        extern "C" fn f(_: *mut c_void) -> i32 {
            7
        }
        let table = [core::ptr::null::<c_void>(), f as *const c_void];
        // SAFETY: the table holds one null and one function of the stated type.
        unsafe {
            assert!(suite_slot::<unsafe extern "C" fn(*mut c_void) -> i32>(
                table.as_ptr() as *const c_void,
                0
            )
            .is_none());
            let g = suite_slot::<unsafe extern "C" fn(*mut c_void) -> i32>(
                table.as_ptr() as *const c_void,
                1,
            )
            .unwrap();
            assert_eq!(g(core::ptr::null_mut()), 7);
            assert!(suite_slot::<unsafe extern "C" fn()>(core::ptr::null(), 0).is_none());
        }
    }
}
