//! Reads the PiPL resource back out of a built Windows library with a small PE reader of our own
//! (resource directory walk) and compares it with the bytes the serialiser makes from the same
//! description.
//!
//! The library is `$EFFECT_SHELL_PIPL_DLL` when set (the test then fails when it is unreadable);
//! otherwise `target/x86_64-pc-windows-gnu/{release,debug}/effect_shell.dll` below the workspace
//! target directory is used when present, and the test does nothing when no library was built.
//! The description is `$EFFECT_SHELL_PIPL` or the crate's sample, the same choice build.rs makes.

use effect_shell::pipl::*;
use std::{env, fs, path::PathBuf};

fn u16at(b: &[u8], at: usize) -> usize {
    u16::from_le_bytes([b[at], b[at + 1]]) as usize
}

fn u32at(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}

/// Section table entries: (virtual address, virtual size, raw pointer, raw size).
fn sections(d: &[u8]) -> (Vec<(usize, usize, usize, usize)>, usize) {
    let pe = u32at(d, 0x3c);
    assert_eq!(&d[pe..pe + 4], b"PE\0\0", "PE signature");
    let n = u16at(d, pe + 6);
    let opt = u16at(d, pe + 20);
    let magic = u16at(d, pe + 24);
    let dd = pe + 24 + if magic == 0x20b { 112 } else { 96 };
    let mut o = pe + 24 + opt;
    let mut v = Vec::new();
    for _ in 0..n {
        v.push((
            u32at(d, o + 12),
            u32at(d, o + 8),
            u32at(d, o + 20),
            u32at(d, o + 16),
        ));
        o += 40;
    }
    (v, dd)
}

fn rva_to_off(s: &[(usize, usize, usize, usize)], rva: usize) -> usize {
    for &(va, vs, rp, rs) in s {
        if rva >= va && rva < va + vs.max(rs) {
            return rva - va + rp;
        }
    }
    panic!("rva {rva:#x} is in no section");
}

/// Returns the data of the resource with this named type and numeric id, any language.
fn find_resource(d: &[u8], type_name: &str, id: u32) -> Option<Vec<u8>> {
    let (secs, dd) = sections(d);
    let rrva = u32at(d, dd + 16);
    if rrva == 0 {
        return None;
    }
    let base = rva_to_off(&secs, rrva);
    let entries = |dir: usize| -> Vec<(u32, usize)> {
        let named = u16at(d, dir + 12);
        let ids = u16at(d, dir + 14);
        (0..named + ids)
            .map(|i| {
                (
                    u32at(d, dir + 16 + i * 8) as u32,
                    u32at(d, dir + 20 + i * 8),
                )
            })
            .collect()
    };
    let name_of = |n: u32| -> String {
        let at = base + (n & 0x7fff_ffff) as usize;
        let len = u16at(d, at);
        let units: Vec<u16> = (0..len).map(|i| u16at(d, at + 2 + i * 2) as u16).collect();
        String::from_utf16(&units).unwrap()
    };
    for (tn, toff) in entries(base) {
        if tn & 0x8000_0000 == 0 || name_of(tn) != type_name {
            continue;
        }
        for (idn, ioff) in entries(base + (toff & 0x7fff_ffff)) {
            if idn != id {
                continue;
            }
            let langs = entries(base + (ioff & 0x7fff_ffff));
            let (_lang, loff) = *langs.first()?;
            let ent = base + loff;
            let rva = u32at(d, ent);
            let size = u32at(d, ent + 4);
            let off = rva_to_off(&secs, rva);
            return Some(d[off..off + size].to_vec());
        }
    }
    None
}

fn description() -> String {
    match env::var("EFFECT_SHELL_PIPL") {
        Ok(p) if !p.is_empty() => fs::read_to_string(p).unwrap(),
        _ => include_str!("../sample/sample_pipl.txt").to_string(),
    }
}

fn dll_path() -> Option<PathBuf> {
    if let Ok(p) = env::var("EFFECT_SHELL_PIPL_DLL") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let ws = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let t = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ws.join("target"));
    ["release", "debug"]
        .iter()
        .map(|p| {
            t.join("x86_64-pc-windows-gnu")
                .join(p)
                .join("effect_shell.dll")
        })
        .find(|p| p.is_file())
}

#[test]
fn built_windows_library_carries_the_serialised_property_list() {
    let Some(path) = dll_path() else {
        eprintln!("no built Windows library found, nothing to check");
        return;
    };
    let dll = fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let got = find_resource(&dll, RESOURCE_TYPE, RESOURCE_ID)
        .unwrap_or_else(|| panic!("no PiPL resource 16000 in {}", path.display()));
    let props = parse_description(&description()).unwrap();
    assert_eq!(got, serialise(&props), "resource bytes");
    assert_eq!(parse(&got).unwrap(), props);
}

#[test]
fn the_reader_finds_nothing_in_a_library_without_the_resource() {
    // a hand made minimal PE image with an empty resource directory pointer
    let mut d = vec![0u8; 0x200];
    d[0x3c] = 0x40;
    d[0x40..0x44].copy_from_slice(b"PE\0\0");
    d[0x54] = 0xf0; // optional header size
    d[0x58] = 0x0b;
    d[0x59] = 0x02; // PE32+
    assert_eq!(find_resource(&d, "PiPL", 16000), None);
}
