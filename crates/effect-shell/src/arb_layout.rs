//! The records of the arbitrary data callback command (command 22): the extra structure the host
//! passes, with one arm per sub selector, and the handle members of the older utility callback
//! table. Layouts only, no behaviour; the sizes and offsets are tested against the recorded layout
//! tables. A crate that serves the command reads the arm named by `which_function`.

use crate::layout::PfErr;
use crate::render_layout::{slot_fn, UtilCallbacks};
use core::ffi::c_void;

/// The command number of the arbitrary data callback.
pub const CMD_ARBITRARY_CALLBACK: i32 = 22;

pub const ARB_NEW: i32 = 0;
pub const ARB_DISPOSE: i32 = 1;
pub const ARB_COPY: i32 = 2;
pub const ARB_FLAT_SIZE: i32 = 3;
pub const ARB_FLATTEN: i32 = 4;
pub const ARB_UNFLATTEN: i32 = 5;
pub const ARB_INTERP: i32 = 6;
pub const ARB_COMPARE: i32 = 7;
pub const ARB_PRINT_SIZE: i32 = 8;
pub const ARB_PRINT: i32 = 9;
pub const ARB_SCAN: i32 = 10;

pub const ARB_COMPARE_EQUAL: i32 = 0;
pub const ARB_COMPARE_LESS: i32 = 1;
pub const ARB_COMPARE_MORE: i32 = 2;
pub const ARB_COMPARE_NOT_EQUAL: i32 = 3;

/// A host handle to an arbitrary value (opaque).
pub type ArbH = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbNew {
    pub refcon: *mut c_void,
    pub arb_ph: *mut ArbH,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbDispose {
    pub refcon: *mut c_void,
    pub arb_h: ArbH,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbCopy {
    pub refcon: *mut c_void,
    pub src_arb_h: ArbH,
    pub dst_arb_ph: *mut ArbH,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbFlatSize {
    pub refcon: *mut c_void,
    pub arb_h: ArbH,
    pub flat_data_size: *mut u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbFlatten {
    pub refcon: *mut c_void,
    pub arb_h: ArbH,
    pub buf_size: u32,
    pub flat_data: *mut c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbUnflatten {
    pub refcon: *mut c_void,
    pub buf_size: u32,
    pub flat_data: *const c_void,
    pub arb_ph: *mut ArbH,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbInterp {
    pub refcon: *mut c_void,
    pub left_arb_h: ArbH,
    pub right_arb_h: ArbH,
    pub t: f64,
    pub interp_ph: *mut ArbH,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbCompare {
    pub refcon: *mut c_void,
    pub a_arb_h: ArbH,
    pub b_arb_h: ArbH,
    pub compare: *mut i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbPrintSize {
    pub refcon: *mut c_void,
    pub arb_h: ArbH,
    pub print_size: *mut u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbPrint {
    pub refcon: *mut c_void,
    pub print_flags: i32,
    pub arb_h: ArbH,
    pub print_size: u32,
    pub print_buffer: *mut u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbScan {
    pub refcon: *mut c_void,
    pub buf: *const u8,
    pub bytes_to_scan: u32,
    pub arb_ph: *mut ArbH,
}

/// The arms of the extra structure (40 bytes). The first member of every arm is the refcon.
#[repr(C)]
#[derive(Clone, Copy)]
pub union ArbUnion {
    pub new: ArbNew,
    pub dispose: ArbDispose,
    pub copy: ArbCopy,
    pub flat_size: ArbFlatSize,
    pub flatten: ArbFlatten,
    pub unflatten: ArbUnflatten,
    pub interp: ArbInterp,
    pub compare: ArbCompare,
    pub print_size: ArbPrintSize,
    pub print: ArbPrint,
    pub scan: ArbScan,
    raw: [u64; 5],
}

/// The extra structure of the arbitrary data callback (48 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArbParamsExtra {
    pub which_function: i32,
    pub id: i16,
    pub padding: i16,
    pub u: ArbUnion,
}

impl ArbParamsExtra {
    /// An all zero record.
    pub fn zeroed() -> ArbParamsExtra {
        ArbParamsExtra {
            which_function: 0,
            id: 0,
            padding: 0,
            u: ArbUnion { raw: [0; 5] },
        }
    }

    /// The refcon, the first member of every arm.
    ///
    /// # Safety
    /// The record must have been filled by the host (every arm starts with the refcon).
    pub unsafe fn refcon(&self) -> *mut c_void {
        self.u.new.refcon
    }
}

// Handle members of the older utility callback table, by slot (offset / 8).
pub const UTIL_SLOT_NEW_HANDLE: usize = 20;
pub const UTIL_SLOT_LOCK_HANDLE: usize = 21;
pub const UTIL_SLOT_UNLOCK_HANDLE: usize = 22;
pub const UTIL_SLOT_DISPOSE_HANDLE: usize = 23;
pub const UTIL_SLOT_GET_HANDLE_SIZE: usize = 55;
pub const UTIL_SLOT_RESIZE_HANDLE: usize = 58;

pub type UtilNewHandleFn = unsafe extern "C" fn(u64) -> *mut c_void;
pub type UtilLockHandleFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type UtilUnlockHandleFn = unsafe extern "C" fn(*mut c_void);
pub type UtilGetHandleSizeFn = unsafe extern "C" fn(*mut c_void) -> u64;
pub type UtilResizeHandleFn = unsafe extern "C" fn(u64, *mut *mut c_void) -> PfErr;

/// Typed readers of the handle members of the utility table.
pub trait UtilHandleMembers {
    /// # Safety
    /// The table must be valid.
    unsafe fn new_handle(&self) -> Option<UtilNewHandleFn>;
    /// # Safety
    /// The table must be valid.
    unsafe fn lock_handle(&self) -> Option<UtilLockHandleFn>;
    /// # Safety
    /// The table must be valid.
    unsafe fn unlock_handle(&self) -> Option<UtilUnlockHandleFn>;
    /// # Safety
    /// The table must be valid.
    unsafe fn get_handle_size(&self) -> Option<UtilGetHandleSizeFn>;
    /// # Safety
    /// The table must be valid.
    unsafe fn resize_handle(&self) -> Option<UtilResizeHandleFn>;
}

impl UtilHandleMembers for UtilCallbacks {
    unsafe fn new_handle(&self) -> Option<UtilNewHandleFn> {
        slot_fn::<UtilNewHandleFn>(self.slots[UTIL_SLOT_NEW_HANDLE])
    }
    unsafe fn lock_handle(&self) -> Option<UtilLockHandleFn> {
        slot_fn::<UtilLockHandleFn>(self.slots[UTIL_SLOT_LOCK_HANDLE])
    }
    unsafe fn unlock_handle(&self) -> Option<UtilUnlockHandleFn> {
        slot_fn::<UtilUnlockHandleFn>(self.slots[UTIL_SLOT_UNLOCK_HANDLE])
    }
    unsafe fn get_handle_size(&self) -> Option<UtilGetHandleSizeFn> {
        slot_fn::<UtilGetHandleSizeFn>(self.slots[UTIL_SLOT_GET_HANDLE_SIZE])
    }
    unsafe fn resize_handle(&self) -> Option<UtilResizeHandleFn> {
        slot_fn::<UtilResizeHandleFn>(self.slots[UTIL_SLOT_RESIZE_HANDLE])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn sizes_match_the_layout_tables() {
        assert_eq!(size_of::<ArbParamsExtra>(), 48);
        assert_eq!(size_of::<ArbUnion>(), 40);
        assert_eq!(size_of::<ArbNew>(), 16);
        assert_eq!(size_of::<ArbDispose>(), 16);
        assert_eq!(size_of::<ArbCopy>(), 24);
        assert_eq!(size_of::<ArbFlatSize>(), 24);
        assert_eq!(size_of::<ArbFlatten>(), 32);
        assert_eq!(size_of::<ArbUnflatten>(), 32);
        assert_eq!(size_of::<ArbInterp>(), 40);
        assert_eq!(size_of::<ArbCompare>(), 32);
        assert_eq!(size_of::<ArbPrintSize>(), 24);
        assert_eq!(size_of::<ArbPrint>(), 40);
        assert_eq!(size_of::<ArbScan>(), 32);
    }

    // Offsets are from the start of the extra structure: the union starts at 8.
    #[test]
    fn member_offsets_match_the_layout_tables() {
        assert_eq!(offset_of!(ArbParamsExtra, which_function), 0);
        assert_eq!(offset_of!(ArbParamsExtra, id), 4);
        assert_eq!(offset_of!(ArbParamsExtra, padding), 6);
        assert_eq!(offset_of!(ArbParamsExtra, u), 8);
        let u = 8;
        assert_eq!(u + offset_of!(ArbNew, refcon), 8);
        assert_eq!(u + offset_of!(ArbNew, arb_ph), 16);
        assert_eq!(u + offset_of!(ArbDispose, arb_h), 16);
        assert_eq!(u + offset_of!(ArbCopy, src_arb_h), 16);
        assert_eq!(u + offset_of!(ArbCopy, dst_arb_ph), 24);
        assert_eq!(u + offset_of!(ArbFlatSize, arb_h), 16);
        assert_eq!(u + offset_of!(ArbFlatSize, flat_data_size), 24);
        assert_eq!(u + offset_of!(ArbFlatten, arb_h), 16);
        assert_eq!(u + offset_of!(ArbFlatten, buf_size), 24);
        assert_eq!(u + offset_of!(ArbFlatten, flat_data), 32);
        assert_eq!(u + offset_of!(ArbUnflatten, buf_size), 16);
        assert_eq!(u + offset_of!(ArbUnflatten, flat_data), 24);
        assert_eq!(u + offset_of!(ArbUnflatten, arb_ph), 32);
        assert_eq!(u + offset_of!(ArbInterp, left_arb_h), 16);
        assert_eq!(u + offset_of!(ArbInterp, right_arb_h), 24);
        assert_eq!(u + offset_of!(ArbInterp, t), 32);
        assert_eq!(u + offset_of!(ArbInterp, interp_ph), 40);
        assert_eq!(u + offset_of!(ArbCompare, a_arb_h), 16);
        assert_eq!(u + offset_of!(ArbCompare, b_arb_h), 24);
        assert_eq!(u + offset_of!(ArbCompare, compare), 32);
        assert_eq!(u + offset_of!(ArbPrintSize, arb_h), 16);
        assert_eq!(u + offset_of!(ArbPrintSize, print_size), 24);
        assert_eq!(u + offset_of!(ArbPrint, print_flags), 16);
        assert_eq!(u + offset_of!(ArbPrint, arb_h), 24);
        assert_eq!(u + offset_of!(ArbPrint, print_size), 32);
        assert_eq!(u + offset_of!(ArbPrint, print_buffer), 40);
        assert_eq!(u + offset_of!(ArbScan, buf), 16);
        assert_eq!(u + offset_of!(ArbScan, bytes_to_scan), 24);
        assert_eq!(u + offset_of!(ArbScan, arb_ph), 32);
    }

    #[test]
    fn selector_numbers_and_util_slots_match_the_tables() {
        assert_eq!(CMD_ARBITRARY_CALLBACK, 22);
        for (got, want) in [
            (ARB_NEW, 0),
            (ARB_DISPOSE, 1),
            (ARB_COPY, 2),
            (ARB_FLAT_SIZE, 3),
            (ARB_FLATTEN, 4),
            (ARB_UNFLATTEN, 5),
            (ARB_INTERP, 6),
            (ARB_COMPARE, 7),
            (ARB_PRINT_SIZE, 8),
            (ARB_PRINT, 9),
            (ARB_SCAN, 10),
        ] {
            assert_eq!(got, want);
        }
        for (got, want) in [
            (ARB_COMPARE_EQUAL, 0),
            (ARB_COMPARE_LESS, 1),
            (ARB_COMPARE_MORE, 2),
            (ARB_COMPARE_NOT_EQUAL, 3),
        ] {
            assert_eq!(got, want);
        }
        assert_eq!(UTIL_SLOT_NEW_HANDLE * 8, 160);
        assert_eq!(UTIL_SLOT_LOCK_HANDLE * 8, 168);
        assert_eq!(UTIL_SLOT_UNLOCK_HANDLE * 8, 176);
        assert_eq!(UTIL_SLOT_DISPOSE_HANDLE * 8, 184);
        assert_eq!(UTIL_SLOT_GET_HANDLE_SIZE * 8, 440);
        assert_eq!(UTIL_SLOT_RESIZE_HANDLE * 8, 464);
    }

    #[test]
    fn member_widths_match_the_layout_tables() {
        use core::mem::size_of_val;
        let e = ArbParamsExtra::zeroed();
        assert_eq!(size_of_val(&e.which_function), 4);
        assert_eq!(size_of_val(&e.id), 2);
        assert_eq!(size_of_val(&e.padding), 2);
        // SAFETY: the record is all zero, every arm is plain data.
        unsafe {
            assert_eq!(size_of_val(&e.u.print.print_flags), 4);
            assert_eq!(size_of_val(&e.u.print.print_size), 4);
            assert_eq!(size_of_val(&e.u.flatten.buf_size), 4);
            assert_eq!(size_of_val(&e.u.unflatten.buf_size), 4);
            assert_eq!(size_of_val(&e.u.scan.bytes_to_scan), 4);
            assert_eq!(size_of_val(&e.u.interp.t), 8);
        }
    }

    unsafe extern "C" fn fake_new(_: u64) -> *mut c_void {
        core::ptr::null_mut()
    }
    unsafe extern "C" fn fake_lock(_: *mut c_void) -> *mut c_void {
        core::ptr::null_mut()
    }
    unsafe extern "C" fn fake_unlock(_: *mut c_void) {}
    unsafe extern "C" fn fake_dispose(_: *mut c_void) {}
    unsafe extern "C" fn fake_size(_: *mut c_void) -> u64 {
        0
    }
    unsafe extern "C" fn fake_resize(_: u64, _: *mut *mut c_void) -> PfErr {
        0
    }

    #[test]
    fn each_typed_reader_returns_the_function_of_its_own_slot() {
        let mut t = UtilCallbacks {
            slots: [core::ptr::null(); 69],
        };
        t.slots[UTIL_SLOT_NEW_HANDLE] = fake_new as UtilNewHandleFn as *const c_void;
        t.slots[UTIL_SLOT_LOCK_HANDLE] = fake_lock as UtilLockHandleFn as *const c_void;
        t.slots[UTIL_SLOT_UNLOCK_HANDLE] = fake_unlock as UtilUnlockHandleFn as *const c_void;
        t.slots[UTIL_SLOT_DISPOSE_HANDLE] = fake_dispose as UtilUnlockHandleFn as *const c_void;
        t.slots[UTIL_SLOT_GET_HANDLE_SIZE] = fake_size as UtilGetHandleSizeFn as *const c_void;
        t.slots[UTIL_SLOT_RESIZE_HANDLE] = fake_resize as UtilResizeHandleFn as *const c_void;
        // SAFETY: the table was filled above with functions of the stated types.
        unsafe {
            assert_eq!(
                t.new_handle().unwrap() as usize,
                fake_new as *const () as usize
            );
            assert_eq!(
                t.lock_handle().unwrap() as usize,
                fake_lock as *const () as usize
            );
            assert_eq!(
                t.unlock_handle().unwrap() as usize,
                fake_unlock as *const () as usize
            );
            assert_eq!(
                t.get_handle_size().unwrap() as usize,
                fake_size as *const () as usize
            );
            assert_eq!(
                t.resize_handle().unwrap() as usize,
                fake_resize as *const () as usize
            );
        }
    }

    #[test]
    fn a_reader_of_an_empty_slot_gives_none() {
        let mut t = UtilCallbacks {
            slots: [core::ptr::null(); 69],
        };
        t.slots[UTIL_SLOT_DISPOSE_HANDLE] = fake_dispose as UtilUnlockHandleFn as *const c_void;
        // SAFETY: the table is valid; only the sixth handle slot is filled.
        unsafe {
            assert!(t.new_handle().is_none());
            assert!(t.lock_handle().is_none());
            assert!(t.unlock_handle().is_none());
            assert!(t.get_handle_size().is_none());
            assert!(t.resize_handle().is_none());
        }
    }

    #[test]
    fn the_refcon_is_shared_by_every_arm() {
        let mut e = ArbParamsExtra::zeroed();
        e.u.scan.refcon = 0x55 as *mut c_void;
        // SAFETY: the record was filled above.
        unsafe {
            assert_eq!(e.refcon() as usize, 0x55);
            assert_eq!(e.u.flatten.refcon as usize, 0x55);
        }
    }
}
