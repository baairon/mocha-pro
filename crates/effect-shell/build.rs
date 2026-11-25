//! Picks the parameter table. The environment variable EFFECT_SHELL_TABLE names a text file in the
//! effect-params format (with optional `#@` directive lines, see src/plan.rs). When it is not set
//! the crate's made up sample table is used. The text is copied into OUT_DIR and compiled in.

use std::{env, fs, path::PathBuf};

#[allow(dead_code)]
#[path = "src/pipl.rs"]
mod pipl;

fn main() {
    println!("cargo:rerun-if-env-changed=EFFECT_SHELL_TABLE");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let src = match env::var("EFFECT_SHELL_TABLE") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
            .join("sample")
            .join("sample_table.txt"),
    };
    println!("cargo:rerun-if-changed={}", src.display());
    let text = fs::read_to_string(&src)
        .unwrap_or_else(|e| panic!("cannot read table {}: {e}", src.display()));
    fs::write(out.join("table.txt"), text).expect("write table copy");

    // The freestanding image (cargo feature `freestanding`, Windows cdylib only): no C runtime
    // start up files and no default libraries, our own entry point. Build it with
    // `--no-default-features --features freestanding` and RUSTFLAGS="-C panic=abort"
    // (see README.md, "Freestanding build").
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_FREESTANDING");
    if env::var_os("CARGO_FEATURE_FREESTANDING").is_some()
        && env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    {
        for a in [
            "-nostartfiles",
            "-nodefaultlibs",
            "-Wl,--entry=DllMainCRTStartup",
        ] {
            println!("cargo:rustc-link-arg-cdylib={a}");
        }
    }

    embed_pipl(&out);
}

/// Windows targets only: serialise the property list named by EFFECT_SHELL_PIPL (default: the
/// crate's made up sample), write it as a COFF object holding a `PiPL` resource (id 16000) and hand
/// the object to the linker of the cdylib. No resource compiler is involved. The description is
/// checked on every target so a broken file fails early everywhere.
fn embed_pipl(out: &std::path::Path) {
    println!("cargo:rerun-if-env-changed=EFFECT_SHELL_PIPL");
    let src = match env::var("EFFECT_SHELL_PIPL") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
            .join("sample")
            .join("sample_pipl.txt"),
    };
    println!("cargo:rerun-if-changed={}", src.display());
    let text = fs::read_to_string(&src)
        .unwrap_or_else(|e| panic!("cannot read property list {}: {e}", src.display()));
    let props = pipl::parse_description(&text)
        .unwrap_or_else(|e| panic!("bad property list {}: {e}", src.display()));
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let machine = pipl::Machine::from_arch(&arch)
        .unwrap_or_else(|| panic!("no PiPL resource object for the architecture {arch}"));
    let ext = if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        "obj"
    } else {
        "o"
    };
    let obj = out.join(format!("pipl_resource.{ext}"));
    fs::write(&obj, pipl::pipl_object(machine, &props)).expect("write resource object");
    println!("cargo:rustc-link-arg-cdylib={}", obj.display());
}
