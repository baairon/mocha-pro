//! The key file: values of keyed parameters that change from frame to frame.
//!
//! A stage with a keyed parameter takes `--layer keys=PATH`. The file is text in the form
//! `keys-1`:
//!
//! ```text
//! keys-1
//! # a comment
//! start 0
//! key shift_x 0 0.0 linear
//! key shift_x 24 12.5 hold
//! key shift_y 0 1.5 linear
//! ```
//!
//! - The first line that is not empty and not a comment is `keys-1`.
//! - `start T` is the time of the first frame of the stream (default 0). It comes at most once
//!   and before the first key. Frame N of the stream is evaluated at time `T + N`.
//! - `key PARAM TIME VALUE KIND` is one key: a parameter id, a time in frames (fractions
//!   allowed), a value and `hold` or `linear`. The kind governs the segment that starts at the
//!   key: `hold` keeps the value until the next key, `linear` moves on the straight line to it.
//! - Keys may come in any order. Two keys of one parameter at the same time are an error.
//! - Before the first key of a parameter the value is the first key's; from the last key on it
//!   is the last key's. A parameter without a key keeps its command line value.
//!
//! The evaluation is [`evaluate`], in f64, with add, subtract, multiply and divide only.

use crate::params::parse_float;

/// The id of the key file layer.
pub const LAYER_ID: &str = "keys";
/// The form word of the key file, its first line.
pub const FORM: &str = "keys-1";

/// How the value moves from a key to the next one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// The value of the key is kept until the next key.
    Hold,
    /// The value moves on the straight line to the next key.
    Linear,
}

/// One key of a track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    /// The time in frames.
    pub time: f64,
    /// The value at that time.
    pub value: f64,
    /// The segment that starts here.
    pub kind: KeyKind,
}

/// The keys of one parameter, sorted by time.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// The parameter id.
    pub param: String,
    /// The keys, at least one, in rising time.
    pub keys: Vec<Key>,
}

/// A parsed key file.
#[derive(Clone, Debug, PartialEq)]
pub struct KeySet {
    start: f64,
    tracks: Vec<Track>,
}

/// The value of a track at time `t`. `keys` is sorted by time and not empty.
pub fn evaluate(keys: &[Key], t: f64) -> f64 {
    let first = keys[0];
    if t <= first.time {
        return first.value;
    }
    for pair in keys.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t < b.time {
            return match a.kind {
                KeyKind::Hold => a.value,
                KeyKind::Linear => {
                    a.value + (b.value - a.value) * ((t - a.time) / (b.time - a.time))
                }
            };
        }
    }
    keys[keys.len() - 1].value
}

impl KeySet {
    /// A set without tracks: every parameter keeps its command line value.
    pub fn empty() -> KeySet {
        KeySet {
            start: 0.0,
            tracks: Vec::new(),
        }
    }

    /// Parse the text of a key file. The error is one line that names the line of the file.
    pub fn parse(text: &str) -> Result<KeySet, String> {
        let mut set = KeySet::empty();
        let mut seen_form = false;
        let mut seen_start = false;
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let words: Vec<&str> = raw.split_whitespace().collect();
            if words.is_empty() || words[0].starts_with('#') {
                continue;
            }
            if !seen_form {
                if words != [FORM] {
                    return Err(format!("line {line}: the first line is not `{FORM}`"));
                }
                seen_form = true;
                continue;
            }
            match words.as_slice() {
                ["start", time] => {
                    if seen_start || !set.tracks.is_empty() {
                        return Err(format!(
                            "line {line}: `start` comes once and before the first key"
                        ));
                    }
                    set.start = parse_float(time)
                        .ok_or_else(|| format!("line {line}: `{time}` is not a time"))?;
                    seen_start = true;
                }
                ["key", param, time, value, kind] => {
                    let time = parse_float(time)
                        .ok_or_else(|| format!("line {line}: `{time}` is not a time"))?;
                    let value = parse_float(value)
                        .ok_or_else(|| format!("line {line}: `{value}` is not a value"))?;
                    let kind = match *kind {
                        "hold" => KeyKind::Hold,
                        "linear" => KeyKind::Linear,
                        other => {
                            return Err(format!(
                                "line {line}: `{other}` is not a key kind (hold or linear)"
                            ))
                        }
                    };
                    let key = Key { time, value, kind };
                    match set.tracks.iter_mut().find(|t| t.param == *param) {
                        Some(track) => {
                            if track.keys.iter().any(|k| k.time == time) {
                                return Err(format!(
                                    "line {line}: two keys of `{param}` at time {time}"
                                ));
                            }
                            track.keys.push(key);
                        }
                        None => set.tracks.push(Track {
                            param: param.to_string(),
                            keys: vec![key],
                        }),
                    }
                }
                _ => {
                    return Err(format!(
                        "line {line}: expected `start TIME` or `key PARAM TIME VALUE KIND`"
                    ))
                }
            }
        }
        if !seen_form {
            return Err(format!("the file has no `{FORM}` line"));
        }
        for track in &mut set.tracks {
            track.keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        Ok(set)
    }

    /// The time of the first frame of the stream.
    pub fn start(&self) -> f64 {
        self.start
    }

    /// The tracks, in the order their parameters first appear in the file.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// The value of `param` at frame `index` of the stream, or `None` without a track for it.
    pub fn value(&self, param: &str, index: u64) -> Option<f64> {
        let track = self.tracks.iter().find(|t| t.param == param)?;
        Some(evaluate(&track.keys, self.start + index as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "keys-1\n# made up keys\n\nstart 10\nkey gain 34 2.0 hold\nkey gain 10 0.0 linear\nkey gain 20 1.0 linear\nkey bias 12.5 -3 hold\n";

    #[test]
    fn a_file_parses_into_sorted_tracks() {
        let set = KeySet::parse(FILE).unwrap();
        assert_eq!(set.start(), 10.0);
        assert_eq!(set.tracks().len(), 2);
        assert_eq!(set.tracks()[0].param, "gain");
        let times: Vec<f64> = set.tracks()[0].keys.iter().map(|k| k.time).collect();
        assert_eq!(times, [10.0, 20.0, 34.0]);
        assert_eq!(set.tracks()[1].keys[0].kind, KeyKind::Hold);
    }

    #[test]
    fn values_follow_the_kinds() {
        let set = KeySet::parse(FILE).unwrap();
        assert_eq!(set.value("gain", 0), Some(0.0));
        assert_eq!(set.value("gain", 5), Some(0.5));
        assert_eq!(set.value("gain", 10), Some(1.0));
        assert_eq!(set.value("gain", 17), Some(1.5));
        assert_eq!(set.value("gain", 24), Some(2.0));
        assert_eq!(set.value("gain", 500), Some(2.0));
        assert_eq!(set.value("bias", 0), Some(-3.0));
        assert_eq!(set.value("bias", 99), Some(-3.0));
        assert_eq!(set.value("absent", 0), None);
    }

    #[test]
    fn hold_keeps_the_value_until_the_next_key() {
        let keys = [
            Key {
                time: 0.0,
                value: 4.0,
                kind: KeyKind::Hold,
            },
            Key {
                time: 2.0,
                value: 8.0,
                kind: KeyKind::Linear,
            },
        ];
        assert_eq!(evaluate(&keys, -1.0), 4.0);
        assert_eq!(evaluate(&keys, 1.999), 4.0);
        assert_eq!(evaluate(&keys, 2.0), 8.0);
        assert_eq!(evaluate(&keys, 3.0), 8.0);
    }

    #[test]
    fn a_key_time_returns_the_key_value_exactly() {
        let keys = [
            Key {
                time: 0.0,
                value: 0.1,
                kind: KeyKind::Linear,
            },
            Key {
                time: 3.0,
                value: 0.7,
                kind: KeyKind::Linear,
            },
        ];
        assert_eq!(evaluate(&keys, 0.0).to_bits(), 0.1f64.to_bits());
        assert_eq!(evaluate(&keys, 3.0).to_bits(), 0.7f64.to_bits());
    }

    #[test]
    fn the_start_defaults_to_zero() {
        let set = KeySet::parse("keys-1\nkey gain 1 5 linear\nkey gain 3 9 linear\n").unwrap();
        assert_eq!(set.start(), 0.0);
        assert_eq!(set.value("gain", 2), Some(7.0));
    }

    #[test]
    fn a_file_with_the_form_line_alone_has_no_track() {
        assert_eq!(KeySet::parse("keys-1\n").unwrap(), KeySet::empty());
        assert_eq!(
            KeySet::parse("\r\n# x\r\nkeys-1\r\n").unwrap(),
            KeySet::empty()
        );
    }

    #[test]
    fn malformed_files_name_the_line() {
        for (text, needle) in [
            ("", "has no `keys-1` line"),
            ("keys-2\n", "line 1: the first line is not `keys-1`"),
            ("key gain 0 1 linear\n", "line 1: the first line"),
            ("keys-1\nkey gain 0 1\n", "line 2: expected"),
            ("keys-1\nkey gain x 1 hold\n", "line 2: `x` is not a time"),
            (
                "keys-1\nkey gain 0 nan hold\n",
                "line 2: `nan` is not a value",
            ),
            (
                "keys-1\nkey gain 0 1 bezier\n",
                "line 2: `bezier` is not a key kind",
            ),
            (
                "keys-1\nkey gain 0 1 hold\nkey gain 0 2 hold\n",
                "line 3: two keys of `gain`",
            ),
            (
                "keys-1\nkey gain 0 1 hold\nstart 4\n",
                "line 3: `start` comes once",
            ),
            ("keys-1\nstart 1\nstart 2\n", "line 3: `start` comes once"),
            ("keys-1\nstart soon\n", "line 2: `soon` is not a time"),
            ("keys-1\nbogus\n", "line 2: expected"),
        ] {
            let e = KeySet::parse(text).expect_err(text);
            assert!(e.contains(needle), "{text:?}: {e}");
            assert!(!e.contains('\n'));
        }
    }
}
