//! The property list serialiser, the parser, the description reader and the COFF object writer,
//! on made up lists.

use effect_shell::pipl::*;

const HEAD: [u8; 10] = [1, 0, 0, 0, 0, 0, 1, 0, 0, 0];

#[test]
fn one_u32_property_exact_bytes() {
    let p = PiplList::new(*b"8BIM").u32(*b"abcd", 0x0102_0304).build();
    let mut want = HEAD.to_vec();
    want.extend_from_slice(b"MIB8"); // signature, characters reversed
    want.extend_from_slice(b"dcba"); // key, characters reversed
    want.extend_from_slice(&[0, 0, 0, 0]); // id
    want.extend_from_slice(&[4, 0, 0, 0]); // length
    want.extend_from_slice(&[4, 3, 2, 1]); // data, little endian
    assert_eq!(serialise(&p), want);
}

#[test]
fn raw_data_is_padded_but_length_is_unpadded() {
    for (n, pad) in [(0usize, 0usize), (1, 3), (2, 2), (3, 1), (4, 0), (5, 3)] {
        let p = PiplList::new(*b"8BIM")
            .raw(*b"wxyz", 9, vec![0xaa; n])
            .build();
        let b = serialise(&p);
        assert_eq!(b.len(), 10 + 16 + n + pad, "n = {n}");
        assert_eq!(&b[26 - 4..26], &(n as u32).to_le_bytes(), "length field");
        assert_eq!(&b[10 + 8..10 + 12], &[9, 0, 0, 0], "id");
        assert!(b[10 + 16 + n..].iter().all(|&x| x == 0), "padding is zero");
        assert_eq!(parse(&b).unwrap(), p);
    }
}

#[test]
fn string_helpers_record_the_padded_length() {
    // "Hi" as a pascal string is 3 bytes, padded to 4; "abc" as a C string is 4 bytes, no padding;
    // "abcd" as a C string is 5 bytes, padded to 8.
    let p = PiplList::new(*b"8BIM")
        .pstring(*b"pstr", "Hi")
        .unwrap()
        .cstring(*b"cst3", "abc")
        .unwrap()
        .cstring(*b"cst4", "abcd")
        .unwrap()
        .build();
    assert_eq!(p[0].data, vec![2, b'H', b'i', 0]);
    assert_eq!(p[1].data, vec![b'a', b'b', b'c', 0]);
    assert_eq!(p[2].data, vec![b'a', b'b', b'c', b'd', 0, 0, 0, 0]);
    let b = serialise(&p);
    assert_eq!(b.len(), 10 + 3 * 16 + 4 + 4 + 8);
    assert_eq!(parse(&b).unwrap(), p);
}

#[test]
fn string_helpers_refuse_bad_input() {
    assert!(PiplList::new(*b"8BIM")
        .pstring(*b"pstr", &"x".repeat(256))
        .is_err());
    assert!(PiplList::new(*b"8BIM")
        .pstring(*b"pstr", &"x".repeat(255))
        .is_ok());
    assert!(PiplList::new(*b"8BIM").cstring(*b"cstr", "a\0b").is_err());
}

#[test]
fn typed_helpers_values() {
    let p = PiplList::new(*b"8BIM")
        .kind(*b"ABCD")
        .pair(*b"vers", 3, 7)
        .global_flags(0x1111_2222)
        .global_flags2(0x33)
        .info()
        .build();
    assert_eq!(find_u32(&p, *b"kind"), Some(0x4142_4344));
    assert_eq!(find_u32(&p, *b"vers"), Some(3 | (7 << 16)));
    assert_eq!(find_u32(&p, *b"eGLO"), Some(0x1111_2222));
    assert_eq!(find_u32(&p, *b"eGL2"), Some(0x33));
    assert_eq!(find_u32(&p, *b"eINF"), None, "two bytes are not a u32");
    assert_eq!(find_u32(&p, *b"none"), None);
    // the kind code is stored reversed in the file
    let b = serialise(&p[..1]);
    assert_eq!(&b[26..30], b"DCBA");
}

#[test]
fn round_trip_of_a_mixed_list() {
    let p = PiplList::new(*b"8BIM")
        .kind(*b"ABCD")
        .name("Made Up")
        .unwrap()
        .category("Made Up Group")
        .unwrap()
        .entry_win64("Main")
        .unwrap()
        .pair(*b"pair", 1, 2)
        .info()
        .global_flags(5)
        .global_flags2(6)
        .match_name("made_up")
        .unwrap()
        .raw(*b"idee", 77, vec![1, 2, 3])
        .build();
    let b = serialise(&p);
    assert_eq!(b.len() % 2, 0);
    assert_eq!(parse(&b).unwrap(), p);
}

#[test]
fn parser_refuses_damage() {
    let p = PiplList::new(*b"8BIM").raw(*b"abcd", 0, vec![1]).build();
    let good = serialise(&p);
    assert!(parse(&good).is_ok());
    assert!(parse(&good[..9]).is_err(), "short header");
    assert!(parse(&good[..good.len() - 1]).is_err(), "cut padding");
    let mut t = good.clone();
    t.push(0);
    assert!(parse(&t).is_err(), "trailing byte");
    let mut v = good.clone();
    v[0] = 2;
    assert!(parse(&v).is_err(), "version");
    let mut r = good.clone();
    r[2] = 1;
    assert!(parse(&r).is_err(), "reserved");
    let mut pad = good.clone();
    let last = pad.len() - 1;
    pad[last] = 1;
    assert!(parse(&pad).is_err(), "padding not zero");
    let mut cnt = good.clone();
    cnt[6] = 2;
    assert!(parse(&cnt).is_err(), "count too large");
    let mut len = good;
    len[22] = 0xff;
    len[23] = 0xff;
    len[24] = 0xff;
    len[25] = 0xff;
    assert!(parse(&len).is_err(), "length past the end");
}

#[test]
fn description_reader_matches_the_helpers() {
    let text = "\
# made up
signature 8BIM
kind ABCD
name \"Two Words\"
category \"G\"
entry_win64 \"Main\"
pair ePVR 2 0
u32 eVER 0x10
info
global_flags 4660
global_flags2 0x56
match_name \"m\"
hex rawb 01 02 03 id=5
";
    let want = PiplList::new(*b"8BIM")
        .kind(*b"ABCD")
        .name("Two Words")
        .unwrap()
        .category("G")
        .unwrap()
        .entry_win64("Main")
        .unwrap()
        .pair(*b"ePVR", 2, 0)
        .u32(*b"eVER", 0x10)
        .info()
        .global_flags(4660)
        .global_flags2(0x56)
        .match_name("m")
        .unwrap()
        .raw(*b"rawb", 5, vec![1, 2, 3])
        .build();
    assert_eq!(parse_description(text).unwrap(), want);
}

#[test]
fn description_reader_refuses_bad_lines() {
    for bad in [
        "",
        "# only a comment\n",
        "kind AB\n",
        "name\n",
        "name \"unterminated\n",
        "u32 abcd 4294967296\n",
        "u32 abcd 1 2\n",
        "pair abcd 70000 1\n",
        "hex abcd 1\n",
        "hex abcd zz\n",
        "bogus x\n",
        "kind ABCD\nsignature 8BIM\n",
        "kind ABCD id=zz\n",
    ] {
        assert!(parse_description(bad).is_err(), "accepted: {bad:?}");
    }
}

#[test]
fn the_shipped_sample_parses_and_round_trips() {
    let text = include_str!("../sample/sample_pipl.txt");
    let p = parse_description(text).unwrap();
    assert_eq!(p.len(), 12);
    assert_eq!(parse(&serialise(&p)).unwrap(), p);
    assert_eq!(find_u32(&p, *b"eGLO"), Some(0x1234));
}

fn u16at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

#[test]
fn coff_object_layout_is_pinned() {
    let data = [1u8, 2, 3, 4, 5];
    let o = coff_resource_object(Machine::X86_64, "Ty", 77, 1033, &data);
    // header
    assert_eq!(u16at(&o, 0), 0x8664);
    assert_eq!(u16at(&o, 2), 2, "sections");
    let symtab = u32at(&o, 8) as usize;
    assert_eq!(u32at(&o, 12), 2, "symbols");
    // section headers
    assert_eq!(&o[20..28], b".rsrc$01");
    assert_eq!(&o[60..68], b".rsrc$02");
    let (s1_size, s1_ptr, s1_rel) = (
        u32at(&o, 36) as usize,
        u32at(&o, 40) as usize,
        u32at(&o, 44) as usize,
    );
    assert_eq!(u16at(&o, 52), 1, "relocations of section 1");
    assert_eq!(u32at(&o, 56), 0x4030_0040);
    let (s2_size, s2_ptr) = (u32at(&o, 76) as usize, u32at(&o, 80) as usize);
    assert_eq!(s2_size, 8, "5 data bytes padded to 8");
    assert_eq!(&o[s2_ptr..s2_ptr + 8], &[1, 2, 3, 4, 5, 0, 0, 0]);
    // tree: root with one named entry, then one id entry, then one language entry
    let s1 = &o[s1_ptr..s1_ptr + s1_size];
    assert_eq!((u16at(s1, 12), u16at(s1, 14)), (1, 0));
    assert_eq!(u32at(s1, 16), 0x8000_0000 | 88);
    let l2 = (u32at(s1, 20) & 0x7fff_ffff) as usize;
    assert_eq!((u16at(s1, l2 + 12), u16at(s1, l2 + 14)), (0, 1));
    assert_eq!(u32at(s1, l2 + 16), 77);
    let l3 = (u32at(s1, l2 + 20) & 0x7fff_ffff) as usize;
    assert_eq!(u32at(s1, l3 + 16), 1033);
    let ent = u32at(s1, l3 + 20) as usize;
    assert_eq!(u32at(s1, ent), 0, "rva is left to the relocation");
    assert_eq!(u32at(s1, ent + 4), 5, "size is the unpadded length");
    // the name string: length 2, "Ty" in UTF-16
    let name = (u32at(s1, 16) & 0x7fff_ffff) as usize;
    assert_eq!(u16at(s1, name), 2);
    assert_eq!(&s1[name + 2..name + 6], &[b'T', 0, b'y', 0]);
    // relocation points at the rva field and at symbol 1, type ADDR32NB
    assert_eq!(u32at(&o, s1_rel) as usize, ent);
    assert_eq!(u32at(&o, s1_rel + 4), 1);
    assert_eq!(u16at(&o, s1_rel + 8), 3);
    // symbols and the empty string table
    assert_eq!(&o[symtab..symtab + 8], b".rsrc$01");
    assert_eq!(&o[symtab + 18..symtab + 26], b".rsrc$02");
    assert_eq!(u16at(&o, symtab + 18 + 12), 2, "section number of symbol 1");
    assert_eq!(o[symtab + 16], 3, "static storage class");
    assert_eq!(u32at(&o, symtab + 36), 4);
    assert_eq!(o.len(), symtab + 40);
}

#[test]
fn machines_pick_their_codes() {
    assert_eq!(Machine::from_arch("x86_64"), Some(Machine::X86_64));
    assert_eq!(Machine::from_arch("x86"), Some(Machine::X86));
    assert_eq!(Machine::from_arch("aarch64"), Some(Machine::Arm64));
    assert_eq!(Machine::from_arch("mips"), None);
    let x = coff_resource_object(Machine::X86, "T", 1, 1, &[]);
    assert_eq!(u16at(&x, 0), 0x14c);
    let a = coff_resource_object(Machine::Arm64, "T", 1, 1, &[]);
    assert_eq!(u16at(&a, 0), 0xaa64);
}
