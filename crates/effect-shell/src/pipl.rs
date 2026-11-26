//! Plug-in property list (PiPL) resource: a serialiser, a parser, a text description format and a
//! writer for a COFF object that carries the bytes as a Windows resource of type `PiPL`.
//!
//! The module needs no external tool and no dependency: it is `core` plus `alloc` only, and build.rs
//! includes this very file by path to embed the resource in the Windows library.
//!
//! Byte layout (little endian), as used by this crate:
//! - header: u16 version (1), u32 reserved (0), u32 property count;
//! - each property: 4 byte signature, 4 byte key, u32 id, u32 data length, the data, then zero
//!   padding up to a multiple of 4 bytes (the padding is not part of the length field);
//! - a four character code such as `kind` is stored as the little endian bytes of the big endian
//!   number it spells, i.e. the characters in reverse order in the file;
//! - the string helpers store the already padded length in the length field.
//!
//! All names, codes and values in tests and in the sample description are made up.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

/// Resource type name of the property list.
pub const RESOURCE_TYPE: &str = "PiPL";
/// Resource id of the property list.
pub const RESOURCE_ID: u32 = 16000;
/// Language id written into the resource directory (US English).
pub const RESOURCE_LANG: u32 = 1033;
/// The signature every property record carries in practice.
pub const DEFAULT_SIGNATURE: [u8; 4] = *b"8BIM";
/// Format version written in the header.
pub const FORMAT_VERSION: u16 = 1;
const HEADER_LEN: usize = 10;
const RECORD_HEAD: usize = 16;

/// One property record. `signature` and `key` are four character codes in reading order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub signature: [u8; 4],
    pub key: [u8; 4],
    pub id: u32,
    pub data: Vec<u8>,
}

/// Errors of the parser, the description reader and the helpers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiplError(pub String);

impl fmt::Display for PiplError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(m: impl Into<String>) -> Result<T, PiplError> {
    Err(PiplError(m.into()))
}

/// Rounds up to a multiple of 4.
pub fn pad4(n: usize) -> usize {
    (n + 3) & !3
}

fn code_bytes(c: [u8; 4]) -> [u8; 4] {
    [c[3], c[2], c[1], c[0]]
}

/// The u32 a four character code spells (first character most significant).
pub fn code_value(c: [u8; 4]) -> u32 {
    u32::from_be_bytes(c)
}

/// Serialises the property list into the exact resource bytes.
pub fn serialise(props: &[Property]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(props.len() as u32).to_le_bytes());
    for p in props {
        out.extend_from_slice(&code_bytes(p.signature));
        out.extend_from_slice(&code_bytes(p.key));
        out.extend_from_slice(&p.id.to_le_bytes());
        out.extend_from_slice(&(p.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&p.data);
        out.resize(out.len() + pad4(p.data.len()) - p.data.len(), 0);
    }
    out
}

fn rd32(b: &[u8], at: usize) -> Result<u32, PiplError> {
    match b.get(at..at + 4) {
        Some(s) => Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]])),
        None => err(format!("truncated at byte {at}")),
    }
}

/// Parses resource bytes back into properties (the inverse of [`serialise`]). Padding bytes must be
/// zero and the data must end exactly at the end of the last record.
pub fn parse(b: &[u8]) -> Result<Vec<Property>, PiplError> {
    if b.len() < HEADER_LEN {
        return err("shorter than the header");
    }
    let ver = u16::from_le_bytes([b[0], b[1]]);
    if ver != FORMAT_VERSION {
        return err(format!("unsupported format version {ver}"));
    }
    if rd32(b, 2)? != 0 {
        return err("reserved word is not zero");
    }
    let count = rd32(b, 6)? as usize;
    let mut at = HEADER_LEN;
    let mut props = Vec::new();
    for i in 0..count {
        let sig = code_bytes(rd32(b, at)?.to_le_bytes());
        let key = code_bytes(rd32(b, at + 4)?.to_le_bytes());
        let id = rd32(b, at + 8)?;
        let len = rd32(b, at + 12)? as usize;
        let start = at + RECORD_HEAD;
        let padded = pad4(len);
        let Some(end) = start.checked_add(padded) else {
            return err(format!("property {i}: length overflow"));
        };
        let Some(raw) = b.get(start..end) else {
            return err(format!("property {i}: data runs past the end"));
        };
        if raw[len..].iter().any(|&x| x != 0) {
            return err(format!("property {i}: padding is not zero"));
        }
        props.push(Property {
            signature: sig,
            key,
            id,
            data: raw[..len].to_vec(),
        });
        at = end;
    }
    if at != b.len() {
        return err("trailing bytes after the last property");
    }
    Ok(props)
}

/// Builds the common properties with a fixed signature and id 0.
#[derive(Clone, Debug)]
pub struct PiplList {
    signature: [u8; 4],
    props: Vec<Property>,
}

impl PiplList {
    pub fn new(signature: [u8; 4]) -> Self {
        PiplList {
            signature,
            props: Vec::new(),
        }
    }

    /// A property with arbitrary data and id.
    pub fn raw(mut self, key: [u8; 4], id: u32, data: Vec<u8>) -> Self {
        self.props.push(Property {
            signature: self.signature,
            key,
            id,
            data,
        });
        self
    }

    /// A 32 bit value.
    pub fn u32(self, key: [u8; 4], v: u32) -> Self {
        self.raw(key, 0, v.to_le_bytes().to_vec())
    }

    /// Two 16 bit fields (first in the low half), the form of the version pair properties.
    pub fn pair(self, key: [u8; 4], first: u16, second: u16) -> Self {
        self.u32(key, u32::from(first) | (u32::from(second) << 16))
    }

    /// A four character code stored as a value (the `kind` property).
    pub fn code(self, key: [u8; 4], c: [u8; 4]) -> Self {
        self.u32(key, code_value(c))
    }

    /// Pascal string (length byte, text), zero padded to a multiple of 4; the padded size is the
    /// recorded length. At most 255 bytes of text.
    pub fn pstring(self, key: [u8; 4], s: &str) -> Result<Self, PiplError> {
        if s.len() > 255 {
            return err("pascal string longer than 255 bytes");
        }
        let mut d = vec![s.len() as u8];
        d.extend_from_slice(s.as_bytes());
        d.resize(pad4(d.len()), 0);
        Ok(self.raw(key, 0, d))
    }

    /// C string (text, NUL), zero padded to a multiple of 4; the padded size is the recorded length.
    pub fn cstring(self, key: [u8; 4], s: &str) -> Result<Self, PiplError> {
        if s.as_bytes().contains(&0) {
            return err("NUL inside a C string");
        }
        let mut d = s.as_bytes().to_vec();
        d.push(0);
        d.resize(pad4(d.len()), 0);
        Ok(self.raw(key, 0, d))
    }

    /// Plug-in kind (key `kind`).
    pub fn kind(self, c: [u8; 4]) -> Self {
        self.code(*b"kind", c)
    }

    /// Display name (key `name`).
    pub fn name(self, s: &str) -> Result<Self, PiplError> {
        self.pstring(*b"name", s)
    }

    /// Category (key `catg`).
    pub fn category(self, s: &str) -> Result<Self, PiplError> {
        self.pstring(*b"catg", s)
    }

    /// Entry point of Windows 64 bit (key `8664`).
    pub fn entry_win64(self, s: &str) -> Result<Self, PiplError> {
        self.cstring(*b"8664", s)
    }

    /// Global out flags (key `eGLO`); must equal what the plug-in reports in its global setup.
    pub fn global_flags(self, v: u32) -> Self {
        self.u32(*b"eGLO", v)
    }

    /// Global out flags 2 (key `eGL2`); must equal what the plug-in reports in its global setup.
    pub fn global_flags2(self, v: u32) -> Self {
        self.u32(*b"eGL2", v)
    }

    /// Match name (key `eMNA`).
    pub fn match_name(self, s: &str) -> Result<Self, PiplError> {
        self.cstring(*b"eMNA", s)
    }

    /// Two zero bytes of info (key `eINF`), as a property of length 2.
    pub fn info(self) -> Self {
        self.raw(*b"eINF", 0, vec![0, 0])
    }

    pub fn build(self) -> Vec<Property> {
        self.props
    }
}

fn parse_u32(t: &str) -> Result<u32, PiplError> {
    let r = if let Some(h) = t.strip_prefix("0x") {
        u32::from_str_radix(h, 16)
    } else {
        t.parse::<u32>()
    };
    match r {
        Ok(v) => Ok(v),
        Err(_) => err(format!("not a 32 bit number: {t}")),
    }
}

fn parse_u16(t: &str) -> Result<u16, PiplError> {
    let v = parse_u32(t)?;
    u16::try_from(v).or_else(|_| err(format!("not a 16 bit number: {t}")))
}

fn parse_code(t: &str) -> Result<[u8; 4], PiplError> {
    let b = t.as_bytes();
    if b.len() == 4 && b.iter().all(|c| c.is_ascii_graphic()) {
        Ok([b[0], b[1], b[2], b[3]])
    } else {
        err(format!("not a four character code: {t}"))
    }
}

/// Splits a line into words; a word in double quotes may hold spaces (no escapes).
fn words(line: &str) -> Result<Vec<(String, bool)>, PiplError> {
    let mut out = Vec::new();
    let mut it = line.chars().peekable();
    while let Some(&c) = it.peek() {
        if c.is_whitespace() {
            it.next();
        } else if c == '"' {
            it.next();
            let mut s = String::new();
            loop {
                match it.next() {
                    Some('"') => break,
                    Some(ch) => s.push(ch),
                    None => return err("unterminated quote"),
                }
            }
            out.push((s, true));
        } else {
            let mut s = String::new();
            while let Some(&ch) = it.peek() {
                if ch.is_whitespace() {
                    break;
                }
                s.push(ch);
                it.next();
            }
            out.push((s, false));
        }
    }
    Ok(out)
}

fn hex_bytes(ws: &[(String, bool)]) -> Result<Vec<u8>, PiplError> {
    let mut v = Vec::new();
    for (w, _) in ws {
        if w.len() != 2 {
            return err(format!("hex byte expected, got {w}"));
        }
        match u8::from_str_radix(w, 16) {
            Ok(b) => v.push(b),
            Err(_) => return err(format!("hex byte expected, got {w}")),
        }
    }
    Ok(v)
}

/// Reads a description text into properties. One property per line, `#` starts a comment line,
/// an optional trailing `id=N` sets the id. Lines:
///
/// ```text
/// signature 8BIM            (optional, before the first property; default 8BIM)
/// kind CODE                 name "TEXT"          category "TEXT"
/// entry_win64 "SYMBOL"      match_name "TEXT"    info
/// global_flags NUM          global_flags2 NUM
/// u32 KEY NUM               pair KEY NUM NUM     code KEY CODE
/// pstring KEY "TEXT"        cstring KEY "TEXT"   hex KEY BB BB ...
/// ```
pub fn parse_description(text: &str) -> Result<Vec<Property>, PiplError> {
    let mut sig = DEFAULT_SIGNATURE;
    let mut b: Option<PiplList> = None;
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut ws = words(line).map_err(|e| PiplError(format!("line {}: {e}", n + 1)))?;
        let mut id = 0u32;
        if let Some((last, false)) = ws.last() {
            if let Some(v) = last.strip_prefix("id=") {
                id = parse_u32(v).map_err(|e| PiplError(format!("line {}: {e}", n + 1)))?;
                ws.pop();
            }
        }
        let at = |e: PiplError| PiplError(format!("line {}: {e}", n + 1));
        if ws.is_empty() {
            return err(format!("line {}: an id with no property line", n + 1));
        }
        let cmd = ws[0].0.clone();
        let args = &ws[1..];
        if cmd == "signature" {
            if b.as_ref().is_some_and(|x| !x.props.is_empty()) {
                return err(format!("line {}: signature after a property", n + 1));
            }
            sig = parse_code(args.first().map_or("", |a| a.0.as_str())).map_err(at)?;
            b = None;
            continue;
        }
        let cur = b.take().unwrap_or_else(|| PiplList::new(sig));
        let a = |i: usize| -> Result<&str, PiplError> {
            match args.get(i) {
                Some(w) => Ok(w.0.as_str()),
                None => err("missing argument"),
            }
        };
        let want = |k: usize| -> Result<(), PiplError> {
            if args.len() == k {
                Ok(())
            } else {
                err(format!("{cmd} takes {k} argument(s)"))
            }
        };
        let res: Result<PiplList, PiplError> = (|| {
            Ok(match cmd.as_str() {
                "kind" => {
                    want(1)?;
                    cur.kind(parse_code(a(0)?)?)
                }
                "name" => {
                    want(1)?;
                    cur.name(a(0)?)?
                }
                "category" => {
                    want(1)?;
                    cur.category(a(0)?)?
                }
                "entry_win64" => {
                    want(1)?;
                    cur.entry_win64(a(0)?)?
                }
                "match_name" => {
                    want(1)?;
                    cur.match_name(a(0)?)?
                }
                "info" => {
                    want(0)?;
                    cur.info()
                }
                "global_flags" => {
                    want(1)?;
                    cur.global_flags(parse_u32(a(0)?)?)
                }
                "global_flags2" => {
                    want(1)?;
                    cur.global_flags2(parse_u32(a(0)?)?)
                }
                "u32" => {
                    want(2)?;
                    cur.raw(
                        parse_code(a(0)?)?,
                        0,
                        parse_u32(a(1)?)?.to_le_bytes().to_vec(),
                    )
                }
                "pair" => {
                    want(3)?;
                    cur.pair(parse_code(a(0)?)?, parse_u16(a(1)?)?, parse_u16(a(2)?)?)
                }
                "code" => {
                    want(2)?;
                    cur.code(parse_code(a(0)?)?, parse_code(a(1)?)?)
                }
                "pstring" => {
                    want(2)?;
                    cur.pstring(parse_code(a(0)?)?, a(1)?)?
                }
                "cstring" => {
                    want(2)?;
                    cur.cstring(parse_code(a(0)?)?, a(1)?)?
                }
                "hex" => {
                    if args.is_empty() {
                        return err("hex takes a key");
                    }
                    cur.raw(parse_code(a(0)?)?, 0, hex_bytes(&args[1..])?)
                }
                _ => return err(format!("unknown line type {cmd}")),
            })
        })();
        let mut nb = res.map_err(at)?;
        if id != 0 {
            if let Some(last) = nb.props.last_mut() {
                last.id = id;
            }
        }
        b = Some(nb);
    }
    match b {
        Some(x) if !x.props.is_empty() => Ok(x.build()),
        _ => err("the description holds no property"),
    }
}

/// Reads the 32 bit value of the first property with this key, if it has exactly 4 bytes.
pub fn find_u32(props: &[Property], key: [u8; 4]) -> Option<u32> {
    let p = props.iter().find(|p| p.key == key)?;
    let d: [u8; 4] = p.data.as_slice().try_into().ok()?;
    Some(u32::from_le_bytes(d))
}

/// Target machine of a COFF object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Machine {
    X86_64,
    X86,
    Arm64,
}

impl Machine {
    /// From the value of CARGO_CFG_TARGET_ARCH.
    pub fn from_arch(a: &str) -> Option<Machine> {
        match a {
            "x86_64" => Some(Machine::X86_64),
            "x86" => Some(Machine::X86),
            "aarch64" => Some(Machine::Arm64),
            _ => None,
        }
    }

    fn code(self) -> u16 {
        match self {
            Machine::X86_64 => 0x8664,
            Machine::X86 => 0x014c,
            Machine::Arm64 => 0xaa64,
        }
    }

    /// The "address without image base" relocation type.
    fn addr32nb(self) -> u16 {
        match self {
            Machine::X86_64 => 3,
            Machine::X86 => 7,
            Machine::Arm64 => 2,
        }
    }
}

fn w16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn w32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn dir(v: &mut Vec<u8>, named: u16, ids: u16) {
    w32(v, 0); // characteristics
    w32(v, 0); // time stamp
    w16(v, 0); // major
    w16(v, 0); // minor
    w16(v, named);
    w16(v, ids);
}

/// Builds a COFF object file with the two sections `.rsrc$01` (directory tree) and `.rsrc$02`
/// (the data), the layout a resource compiler emits, holding one resource of the given named type,
/// id and language. The linker merges the sections into the image's `.rsrc`.
pub fn coff_resource_object(
    machine: Machine,
    type_name: &str,
    id: u32,
    lang: u32,
    data: &[u8],
) -> Vec<u8> {
    // Section 1 layout: root dir, id dir, language dir, data entry, then the type name string.
    let root = 0u32;
    let l2 = root + 16 + 8;
    let l3 = l2 + 16 + 8;
    let entry = l3 + 16 + 8;
    let name_at = entry + 16;
    let mut s1 = Vec::new();
    dir(&mut s1, 1, 0);
    w32(&mut s1, 0x8000_0000 | name_at);
    w32(&mut s1, 0x8000_0000 | l2);
    dir(&mut s1, 0, 1);
    w32(&mut s1, id);
    w32(&mut s1, 0x8000_0000 | l3);
    dir(&mut s1, 0, 1);
    w32(&mut s1, lang);
    w32(&mut s1, entry);
    let reloc_at = s1.len() as u32;
    w32(&mut s1, 0); // data RVA, filled by the relocation
    w32(&mut s1, data.len() as u32);
    w32(&mut s1, 0); // code page
    w32(&mut s1, 0); // reserved
    let units: Vec<u16> = type_name.encode_utf16().collect();
    w16(&mut s1, units.len() as u16);
    for u in &units {
        w16(&mut s1, *u);
    }
    s1.resize(pad4(s1.len()), 0);

    let mut s2 = data.to_vec();
    s2.resize(pad4(s2.len()), 0);

    let nsec = 2u32;
    let raw1 = 20 + 40 * nsec;
    let reloc1 = raw1 + s1.len() as u32;
    let raw2 = reloc1 + 10;
    let symtab = raw2 + s2.len() as u32;
    let flags = 0x4030_0040u32; // initialised data, 4 byte alignment, read

    let mut o = Vec::new();
    w16(&mut o, machine.code());
    w16(&mut o, nsec as u16);
    w32(&mut o, 0);
    w32(&mut o, symtab);
    w32(&mut o, 2);
    w16(&mut o, 0);
    w16(&mut o, 0);
    for (name, size, ptr, rel, nrel) in [
        (b".rsrc$01", s1.len() as u32, raw1, reloc1, 1u16),
        (b".rsrc$02", s2.len() as u32, raw2, 0, 0u16),
    ] {
        o.extend_from_slice(name);
        w32(&mut o, 0);
        w32(&mut o, 0);
        w32(&mut o, size);
        w32(&mut o, ptr);
        w32(&mut o, rel);
        w32(&mut o, 0);
        w16(&mut o, nrel);
        w16(&mut o, 0);
        w32(&mut o, flags);
    }
    o.extend_from_slice(&s1);
    w32(&mut o, reloc_at);
    w32(&mut o, 1); // symbol index of .rsrc$02
    w16(&mut o, machine.addr32nb());
    o.extend_from_slice(&s2);
    for (i, name) in [b".rsrc$01", b".rsrc$02"].iter().enumerate() {
        o.extend_from_slice(*name);
        w32(&mut o, 0); // value
        w16(&mut o, (i + 1) as u16); // section number
        w16(&mut o, 0); // type
        o.push(3); // static
        o.push(0); // no aux
    }
    w32(&mut o, 4); // empty string table
    o
}

/// The object file for a serialised property list, as build.rs links it.
pub fn pipl_object(machine: Machine, props: &[Property]) -> Vec<u8> {
    coff_resource_object(
        machine,
        RESOURCE_TYPE,
        RESOURCE_ID,
        RESOURCE_LANG,
        &serialise(props),
    )
}
