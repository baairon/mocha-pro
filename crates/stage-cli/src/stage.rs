//! The one stage of this provider and the documents it prints: the stage manifest and the
//! `--describe` answer.
//!
//! `selftest-copy` is a self test. It copies frames unchanged and reimplements nothing: it does
//! not compute, imitate or claim any behaviour of any plug-in.

use crate::args::Apply;
use crate::error::StageError;
use crate::frame::{frame_bytes, DEPTHS, MAX_FRAME_BYTES};

/// The provider id. The manifest schema requires `^[a-z][a-z0-9_]{0,31}$`, so it has no hyphen.
pub const PROVIDER_ID: &str = "mocha_pro";
/// The program name, the argv prefix of the manifest command.
pub const PROGRAM: &str = "mocha-stage";
/// The contract this program speaks.
pub const CONTRACT: &str = "frames/2";
/// The id of the self test stage.
pub const STAGE_ID: &str = "selftest-copy";
/// What the manifest says about the stage.
pub const DESCRIPTION: &str = "A self test that copies frames unchanged and reimplements nothing.";

/// The text of `--version`, also the provider version of the manifest.
pub fn version_text() -> String {
    format!("{PROGRAM} {}", env!("CARGO_PKG_VERSION"))
}

/// A JSON string literal (quotes, backslash and control characters escaped).
pub fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The stage manifest of `list --json`: one line of JSON.
pub fn manifest_json() -> String {
    let depths: Vec<String> = DEPTHS.iter().map(|d| d.to_string()).collect();
    format!(
        "{{\"contract\":{},\"provider\":{{\"id\":{},\"version\":{},\"command\":[{}]}},\"stages\":[{{\"id\":{},\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[{}],\"params\":[],\"layers\":[],\"name\":\"Self test copy\",\"description\":{}}}]}}",
        json_string(CONTRACT),
        json_string(PROVIDER_ID),
        json_string(&version_text()),
        json_string(PROGRAM),
        json_string(STAGE_ID),
        depths.join(","),
        json_string(DESCRIPTION),
    )
}

/// The plain text of `list`: one stage id per line.
pub fn list_text() -> String {
    format!("{STAGE_ID}\n")
}

/// A request that passed every check: the stage exists, the depth is supported, no parameter or
/// layer was given and the frame fits the 4 GiB rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checked {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bits per channel, 8, 16 or 32.
    pub depth: u32,
    /// Bytes of one frame.
    pub frame_bytes: u64,
}

/// Judge a parsed `apply` request: `unsupported` for an unknown stage, a depth outside 8, 16 and
/// 32, a parameter or a layer; `usage` for a frame above 4 GiB.
pub fn check(request: &Apply) -> Result<Checked, StageError> {
    if request.stage != STAGE_ID {
        return Err(StageError::unsupported(format!(
            "unknown stage `{}`: this provider has {STAGE_ID}",
            request.stage
        )));
    }
    let depth = u32::try_from(request.depth)
        .ok()
        .filter(|d| DEPTHS.contains(d))
        .ok_or_else(|| {
            StageError::unsupported(format!(
                "depth {} is not supported: {STAGE_ID} takes 8, 16 or 32",
                request.depth
            ))
        })?;
    if let Some((key, _)) = request.params.first() {
        return Err(StageError::unsupported(format!(
            "{STAGE_ID} takes no parameters: `{key}` is not one"
        )));
    }
    if let Some((id, _)) = request.layers.first() {
        return Err(StageError::unsupported(format!(
            "{STAGE_ID} takes no layers: `{id}` is not one"
        )));
    }
    let bytes = frame_bytes(request.width, request.height, depth)
        .filter(|b| *b <= MAX_FRAME_BYTES)
        .ok_or_else(|| {
            StageError::usage(format!(
                "a {}x{} frame at depth {depth} is more than 4 GiB",
                request.width, request.height
            ))
        })?;
    Ok(Checked {
        width: request.width,
        height: request.height,
        depth,
        frame_bytes: bytes,
    })
}

/// The `--describe` answer: the geometry of a preserve stage is the request itself. One line of
/// JSON with the keys of `describe.schema.json` in the order of its `required` list.
pub fn describe_json(request: &Apply, checked: &Checked) -> String {
    let frames = match request.frames {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    };
    format!(
        "{{\"ok\":true,\"contract\":{},\"stage\":{},\"width\":{},\"height\":{},\"depth\":{},\"fps\":{},\"frames\":{}}}",
        json_string(CONTRACT),
        json_string(STAGE_ID),
        checked.width,
        checked.height,
        checked.depth,
        json_string(&request.fps.text()),
        frames
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Fps;
    use crate::error::Code;

    fn req(stage: &str, depth: u64) -> Apply {
        Apply {
            stage: stage.to_string(),
            width: 4,
            height: 2,
            depth,
            fps: Fps { num: 24, den: 1 },
            frames: None,
            describe: false,
            params: vec![],
            layers: vec![],
        }
    }

    #[test]
    fn manifest_text_is_exact() {
        let m = manifest_json();
        assert!(m.starts_with("{\"contract\":\"frames/2\",\"provider\":{\"id\":\"mocha_pro\",\"version\":\"mocha-stage "));
        assert!(m.contains("\"command\":[\"mocha-stage\"]}"));
        assert!(m.contains("\"stages\":[{\"id\":\"selftest-copy\",\"kind\":\"frame\",\"spool\":false,\"geometry\":\"preserve\",\"depths\":[8,16,32],\"params\":[],\"layers\":[]"));
        assert!(m.contains("reimplements nothing"));
        assert!(!m.contains('\n'));
        assert_eq!(m.matches("\"id\":").count(), 2);
    }

    #[test]
    fn version_text_names_the_program_and_crate_version() {
        assert_eq!(
            version_text(),
            format!("mocha-stage {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn json_strings_escape() {
        assert_eq!(json_string("a\"b\\c\n\t\r"), "\"a\\\"b\\\\c\\n\\t\\r\"");
        assert_eq!(json_string("\u{1}"), "\"\\u0001\"");
        assert_eq!(json_string("plain"), "\"plain\"");
    }

    #[test]
    fn check_accepts_the_three_depths() {
        for (d, b) in [(8, 32), (16, 64), (32, 128)] {
            let c = check(&req("selftest-copy", d)).unwrap();
            assert_eq!((c.depth as u64, c.frame_bytes), (d, b));
        }
    }

    #[test]
    fn check_refuses_unknown_stage_depth_param_layer() {
        assert_eq!(check(&req("nope", 8)).unwrap_err().code, Code::Unsupported);
        assert_eq!(
            check(&req("selftest-copy", 12)).unwrap_err().code,
            Code::Unsupported
        );
        assert_eq!(
            check(&req("selftest-copy", 1 << 40)).unwrap_err().code,
            Code::Unsupported
        );
        let mut r = req("selftest-copy", 8);
        r.params.push(("k".into(), "v".into()));
        assert_eq!(check(&r).unwrap_err().code, Code::Unsupported);
        let mut r = req("selftest-copy", 8);
        r.layers.push(("k".into(), "v".into()));
        assert_eq!(check(&r).unwrap_err().code, Code::Unsupported);
    }

    #[test]
    fn check_judges_the_stage_before_the_depth() {
        let e = check(&req("nope", 12)).unwrap_err();
        assert!(e.message.contains("unknown stage"), "{}", e.message);
    }

    #[test]
    fn check_refuses_a_frame_above_four_gib() {
        let mut r = req("selftest-copy", 8);
        r.width = 32768;
        r.height = 32768;
        assert!(check(&r).is_ok());
        r.height = 32769;
        assert_eq!(check(&r).unwrap_err().code, Code::Usage);
    }

    #[test]
    fn describe_answer_text() {
        let mut r = req("selftest-copy", 16);
        let c = check(&r).unwrap();
        assert_eq!(
            describe_json(&r, &c),
            "{\"ok\":true,\"contract\":\"frames/2\",\"stage\":\"selftest-copy\",\"width\":4,\"height\":2,\"depth\":16,\"fps\":\"24/1\",\"frames\":null}"
        );
        r.frames = Some(0);
        assert!(describe_json(&r, &c).ends_with("\"frames\":0}"));
        r.frames = Some(12);
        assert!(describe_json(&r, &c).ends_with("\"frames\":12}"));
    }

    #[test]
    fn list_text_is_the_id() {
        assert_eq!(list_text(), "selftest-copy\n");
    }
}
