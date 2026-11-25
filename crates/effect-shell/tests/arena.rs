//! Host tests of the freestanding arena allocator and memory functions. The module is included by
//! path; with `cfg(test)` it registers no global allocator, panic handler or unmangled symbol.
#![cfg(target_arch = "x86_64")]
#![allow(dead_code, non_snake_case)]

#[path = "../src/freestanding.rs"]
mod freestanding;

use core::alloc::{GlobalAlloc, Layout};
use freestanding::*;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());
const ARENA: usize = 8 * 1024 * 1024;

fn lock() -> std::sync::MutexGuard<'static, ()> {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { reset() };
    g
}
fn lay(s: usize, a: usize) -> Layout {
    Layout::from_size_align(s, a).unwrap()
}
fn alloc(s: usize, a: usize) -> *mut u8 {
    unsafe { Bump.alloc(lay(s, a)) }
}
#[test]
fn every_power_of_two_alignment_up_to_64k_is_honoured() {
    let _g = lock();
    for sh in 0..=16 {
        let a = 1usize << sh;
        let _ = alloc(1, 1);
        let p = alloc(5, a);
        assert!(!p.is_null());
        assert_eq!(p as usize % a, 0, "align {a}");
        unsafe { reset() };
    }
}

#[test]
fn blocks_do_not_overlap_and_keep_their_bytes() {
    let _g = lock();
    let mut v = vec![];
    for i in 0..50usize {
        let l = lay(1 + i * 7, 1 << (i % 6));
        let p = unsafe { Bump.alloc(l) };
        unsafe { core::ptr::write_bytes(p, i as u8, l.size()) };
        v.push((p, l, i as u8));
    }
    for (p, l, b) in &v {
        for k in 0..l.size() {
            assert_eq!(unsafe { *p.add(k) }, *b);
        }
    }
    for w in v.windows(2) {
        assert!(w[0].0 as usize + w[0].1.size() <= w[1].0 as usize);
    }
}

#[test]
fn exhaustion_returns_null_at_the_exact_boundary_and_keeps_the_state() {
    let _g = lock();
    let p = alloc(ARENA, 1);
    assert!(!p.is_null());
    assert!(alloc(1, 1).is_null());
    unsafe { Bump.dealloc(p, lay(ARENA, 1)) };
    assert!(alloc(ARENA + 1, 1).is_null());
    assert!(alloc(isize::MAX as usize - 15, 16).is_null());
    let q = alloc(ARENA - 1, 1);
    assert!(!q.is_null());
    assert!(!alloc(1, 1).is_null());
    assert!(alloc(1, 1).is_null());
}

#[test]
fn alignment_padding_counts_toward_exhaustion() {
    let _g = lock();
    let _ = alloc(1, 1);
    assert!(alloc(ARENA - 1, 4096).is_null());
}

#[test]
fn zero_size_blocks_are_aligned_and_harmless() {
    let _g = lock();
    let a = alloc(8, 8);
    let z = alloc(0, 16);
    assert!(!z.is_null());
    assert_eq!(z as usize % 16, 0);
    assert!(z as usize >= a as usize + 8);
    unsafe { Bump.dealloc(z, lay(0, 16)) };
    let z2 = alloc(0, 1);
    assert!(!z2.is_null());
    unsafe { Bump.dealloc(z2, lay(0, 1)) };
}

#[test]
fn freeing_the_last_block_rewinds_and_any_other_free_is_a_no_op() {
    let _g = lock();
    let a = alloc(32, 8);
    let b = alloc(32, 8);
    unsafe { Bump.dealloc(a, lay(32, 8)) };
    let c = alloc(8, 8);
    assert!(
        c as usize >= b as usize + 32,
        "a free of a non last block must not rewind"
    );
    unsafe { Bump.dealloc(c, lay(8, 8)) };
    unsafe { Bump.dealloc(b, lay(32, 8)) };
    let d = alloc(32, 8);
    assert_eq!(d, b, "the last block is reused after its free");
}

#[test]
fn realloc_of_the_last_block_grows_and_shrinks_in_place() {
    let _g = lock();
    let a = alloc(16, 8);
    unsafe { core::ptr::write_bytes(a, 7, 16) };
    let g = unsafe { Bump.realloc(a, lay(16, 8), 64) };
    assert_eq!(g, a);
    let s = unsafe { Bump.realloc(g, lay(64, 8), 4) };
    assert_eq!(s, a);
    for k in 0..4 {
        assert_eq!(unsafe { *s.add(k) }, 7);
    }
}

#[test]
fn realloc_to_the_end_of_the_arena_works_and_one_more_byte_fails_cleanly() {
    let _g = lock();
    unsafe { reset() };
    let a = alloc(16, 16);
    unsafe { core::ptr::write_bytes(a, 9, 16) };
    let e = unsafe { Bump.realloc(a, lay(16, 16), ARENA) };
    assert_eq!(e, a);
    let f = unsafe { Bump.realloc(e, lay(ARENA, 16), ARENA + 1) };
    assert!(f.is_null());
    assert_eq!(
        unsafe { *a },
        9,
        "the old block stays valid after a failed realloc"
    );
    let huge = unsafe { Bump.realloc(e, lay(ARENA, 16), 1usize << 40) };
    assert!(huge.is_null());
}

#[test]
fn realloc_of_an_inner_block_moves_copies_and_keeps_the_alignment() {
    let _g = lock();
    let a = alloc(8, 64);
    unsafe {
        for k in 0..8 {
            *a.add(k) = k as u8 + 1
        }
    }
    let _b = alloc(8, 8);
    let m = unsafe { Bump.realloc(a, lay(8, 64), 100) };
    assert_ne!(m, a);
    assert_eq!(m as usize % 64, 0);
    for k in 0..8 {
        assert_eq!(unsafe { *m.add(k) }, k as u8 + 1);
    }
    let _c = alloc(1, 1);
    let s = unsafe { Bump.realloc(m, lay(100, 64), 3) };
    for k in 0..3 {
        assert_eq!(unsafe { *s.add(k) }, k as u8 + 1);
    }
}

/// Zeroes the free space above the top so a copy that overruns its block can be seen.
fn wipe_free_space() {
    let z = alloc(4096, 1);
    unsafe { core::ptr::write_bytes(z, 0, 4096) };
    unsafe { Bump.dealloc(z, lay(4096, 1)) };
}

#[test]
fn a_moving_realloc_copies_exactly_the_smaller_of_the_two_sizes() {
    let _g = lock();
    wipe_free_space();
    let a = alloc(24, 8);
    unsafe { core::ptr::write_bytes(a, 0xAA, 24) };
    let b = alloc(24, 8);
    unsafe { core::ptr::write_bytes(b, 0xEE, 24) };
    // shrink of a block that is not the last one: the copy must stop at the new size
    let q = unsafe { Bump.realloc(a, lay(24, 8), 3) };
    assert_ne!(q, a);
    for k in 0..3 {
        assert_eq!(unsafe { *q.add(k) }, 0xAA);
    }
    for k in 3..24 {
        assert_eq!(
            unsafe { *q.add(k) },
            0,
            "the copy ran past the new size at {k}"
        );
    }
    unsafe { reset() };
    wipe_free_space();
    // growth of a block that is not the last one: nothing beyond the old size is copied
    let a = alloc(8, 8);
    unsafe { core::ptr::write_bytes(a, 0xAA, 8) };
    let b = alloc(24, 8);
    unsafe { core::ptr::write_bytes(b, 0xEE, 24) };
    let q = unsafe { Bump.realloc(a, lay(8, 8), 24) };
    assert_ne!(q, a);
    for k in 0..8 {
        assert_eq!(unsafe { *q.add(k) }, 0xAA);
    }
    for k in 8..24 {
        assert_ne!(
            unsafe { *q.add(k) },
            0xEE,
            "the copy read past the old size at {k}"
        );
    }
}

#[test]
fn memcmp_and_bcmp_look_at_the_last_byte() {
    unsafe {
        let a = [1u8, 2, 3, 4];
        let b = [1u8, 2, 3, 9];
        assert_eq!(memcmp(a.as_ptr(), b.as_ptr(), 4), 4 - 9);
        assert_ne!(bcmp(a.as_ptr(), b.as_ptr(), 4), 0);
        assert_eq!(memcmp(a.as_ptr(), b.as_ptr(), 3), 0);
    }
}

#[test]
fn reset_makes_the_whole_arena_available_again() {
    let _g = lock();
    let first = alloc(ARENA / 2, 16);
    let _ = alloc(ARENA / 2, 16);
    assert!(alloc(16, 16).is_null());
    unsafe { reset() };
    assert_eq!(alloc(ARENA / 2, 16), first);
    assert!(!alloc(ARENA / 2, 16).is_null());
}

#[test]
fn memory_functions_follow_the_c_contract() {
    unsafe {
        let a: [u8; 40] = core::array::from_fn(|i| i as u8);
        let mut b = [0u8; 40];
        memcpy(b.as_mut_ptr(), a.as_ptr(), 40);
        assert_eq!(a, b);
        memcpy(b.as_mut_ptr(), b.as_ptr().add(1), 0);
        for (d, s, n) in [
            (3usize, 0usize, 20usize),
            (0, 5, 20),
            (5, 5, 9),
            (20, 0, 20),
            (19, 0, 20),
            (7, 0, 0),
        ] {
            let mut c = a;
            memmove(c.as_mut_ptr().add(d), c.as_ptr().add(s), n);
            let mut e = a;
            e.copy_within(s..s + n, d);
            assert_eq!(c, e, "memmove {d} {s} {n}");
        }
        let mut d = [1u8; 10];
        memset(d.as_mut_ptr(), 0x1ff, 7);
        assert_eq!(&d[..7], &[0xff; 7]);
        assert_eq!(&d[7..], &[1; 3]);
        memset(d.as_mut_ptr(), 0, 0);
        assert_eq!(d[0], 0xff);
        assert_eq!(memcmp(a.as_ptr(), b.as_ptr(), 40), 0);
        b[17] = 200;
        assert_eq!(memcmp(a.as_ptr(), b.as_ptr(), 40), 17 - 200);
        assert!(memcmp(b.as_ptr(), a.as_ptr(), 40) > 0);
        assert_eq!(memcmp(a.as_ptr(), b.as_ptr(), 17), 0);
        assert_ne!(bcmp(a.as_ptr(), b.as_ptr(), 40), 0);
        assert_eq!(bcmp(a.as_ptr(), b.as_ptr(), 17), 0);
        assert_eq!(strlen(c"hello".as_ptr().cast::<u8>()), 5);
        assert_eq!(strlen(c"".as_ptr().cast::<u8>()), 0);
        assert_eq!(
            DllMainCRTStartup(core::ptr::null_mut(), 1, core::ptr::null_mut()),
            1
        );
    }
}
