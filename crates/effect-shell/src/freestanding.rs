//! The runtime of the freestanding build (cargo feature `freestanding`): a static arena as the
//! global allocator, a panic handler, the memory functions the compiler emits calls to, and the
//! DLL entry. With these the Windows cdylib needs no C runtime and imports nothing from the system.
//!
//! Single threaded by contract: the host calls the effect from one thread at a time during
//! GLOBAL_SETUP and PARAMS_SETUP, and the allocator is a bump allocator with no locking. It is
//! built only for the emulator image; the std build never includes this module.
//!
//! The arena is emptied by [`reset`] at the end of every call of the entry point that parses the
//! plan, so a long session cannot exhaust it. The registration items (global allocator, panic
//! handler, unmangled symbols) are left out when the file is compiled as a host test
//! (`tests/arena.rs` includes this file by path), so the allocator logic is tested on the host.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ffi::c_void;

/// Size of the static arena (zero initialised, so it costs no file size).
const ARENA_BYTES: usize = 8 * 1024 * 1024;

#[repr(C, align(16))]
struct Arena {
    mem: UnsafeCell<[u8; ARENA_BYTES]>,
    top: UnsafeCell<usize>,
}

// SAFETY: single threaded contract, see the module comment.
unsafe impl Sync for Arena {}

static ARENA: Arena = Arena {
    mem: UnsafeCell::new([0; ARENA_BYTES]),
    top: UnsafeCell::new(0),
};

/// A bump allocator that gives the arena back from the top: freeing the most recent block moves
/// the top down, any other free is a no-op, so memory is in practice only reclaimed by [`reset`].
/// An exhausted arena returns null (which `alloc` turns into a panic and so into an abort).
pub struct Bump;

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let base = ARENA.mem.get() as usize;
        let top = &mut *ARENA.top.get();
        let start = (base + *top + l.align() - 1) & !(l.align() - 1);
        let end = match start.checked_add(l.size()) {
            Some(e) => e,
            None => return core::ptr::null_mut(),
        };
        if end > base + ARENA_BYTES {
            return core::ptr::null_mut();
        }
        *top = end - base;
        start as *mut u8
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let base = ARENA.mem.get() as usize;
        let top = &mut *ARENA.top.get();
        if p as usize + l.size() == base + *top {
            *top = p as usize - base;
        }
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let base = ARENA.mem.get() as usize;
        let top = &mut *ARENA.top.get();
        // The last block grows or shrinks in place.
        if p as usize + l.size() == base + *top && p as usize + new <= base + ARENA_BYTES {
            *top = p as usize + new - base;
            return p;
        }
        let q = self.alloc(Layout::from_size_align_unchecked(new, l.align()));
        if !q.is_null() {
            memcpy(q, p, l.size().min(new));
        }
        q
    }
}

#[cfg_attr(not(test), global_allocator)]
static GLOBAL: Bump = Bump;

/// Gives the whole arena back: every block handed out so far becomes invalid.
///
/// # Safety
/// No block allocated from the arena may be used afterwards, and no other thread may allocate
/// during the call. The entry point calls it after the plan and every other allocation of the
/// call have been dropped; the crate keeps no allocation in a static.
pub unsafe fn reset() {
    *ARENA.top.get() = 0;
}

#[cfg_attr(not(test), panic_handler)]
#[allow(dead_code)]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: an undefined instruction ends the process, which is the abort the build asks for.
    unsafe { core::arch::asm!("ud2", options(noreturn)) }
}

/// The DLL entry (the image is linked with this as its entry point): nothing to initialise.
#[cfg_attr(not(test), no_mangle)]
pub extern "system" fn DllMainCRTStartup(_h: *mut c_void, _reason: u32, _r: *mut c_void) -> i32 {
    1
}

// The memory functions. Written with `rep` string instructions so the compiler cannot turn the
// loop back into a call of the function itself.

/// # Safety
/// C `memcpy` contract.
#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    core::arch::asm!("rep movsb", inout("rcx") n => _, inout("rdi") d => _, inout("rsi") s => _,
        options(nostack, preserves_flags));
    d
}

/// # Safety
/// C `memmove` contract.
#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn memmove(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    if (d as usize) <= (s as usize) || (d as usize) >= (s as usize) + n {
        return memcpy(d, s, n);
    }
    // Overlap with the destination above the source: copy backwards.
    core::arch::asm!("std", "rep movsb", "cld",
        inout("rcx") n => _, inout("rdi") d.add(n).wrapping_sub(1) => _,
        inout("rsi") s.add(n).wrapping_sub(1) => _, options(nostack));
    d
}

/// # Safety
/// C `memset` contract.
#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 {
    core::arch::asm!("rep stosb", inout("rcx") n => _, inout("rdi") d => _, in("al") c as u8,
        options(nostack, preserves_flags));
    d
}

/// # Safety
/// C `memcmp` contract.
#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let (x, y) = (
            core::ptr::read_volatile(a.add(i)),
            core::ptr::read_volatile(b.add(i)),
        );
        if x != y {
            return x as i32 - y as i32;
        }
        i += 1;
    }
    0
}

/// # Safety
/// C `bcmp` contract.
#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    memcmp(a, b, n)
}

/// The precompiled core and alloc libraries name the unwinding personality routine. With
/// panic=abort nothing ever unwinds, so the symbol only has to exist.
#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn rust_eh_personality() {}

/// Landing pads of the precompiled libraries call this on the unwinding path only; with
/// panic=abort it is never reached.
#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn _Unwind_Resume() -> ! {
    // SAFETY: an undefined instruction ends the process.
    unsafe { core::arch::asm!("ud2", options(noreturn)) }
}

/// # Safety
/// C `strlen` contract.
#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut n = 0;
    while core::ptr::read_volatile(s.add(n)) != 0 {
        n += 1;
    }
    n
}
