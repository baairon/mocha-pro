//! SHA-256 checks: NIST CAVP byte-oriented test vectors, split updates against one shot, the
//! derived constants and the hexadecimal text. The vectors come from the files `SHA256ShortMsg.rsp`
//! and `SHA256LongMsg.rsp` of the NIST Cryptographic Algorithm Validation Program set
//! `shabytetestvectors` (https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program/secure-hashing);
//! every other value is invented.

use mask_seq::sha256::{digest, from_hex, to_hex, Sha256, H0, K};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex digit"))
        .collect()
}

/// SHA256ShortMsg.rsp, the entries of Len 8, 440, 448, 504 and 512 bits (1, 55, 56, 63 and 64
/// bytes: the padding boundaries). Each row is (Len, Msg, MD).
const SHORT: [(usize, &str, &str); 5] = [
    (8, "d3", "28969cdfa74a12c82f3bad960b0b000aca2ac329deea5c2328ebc6f2ba9802c1"),
    (440, "3ebfb06db8c38d5ba037f1363e118550aad94606e26835a01af05078533cc25f2f39573c04b632f62f68c294ab31f2a3e2a1a0d8c2be51", "6595a2ef537a69ba8583dfbf7f5bec0ab1f93ce4c8ee1916eff44a93af5749c4"),
    (448, "2d52447d1244d2ebc28650e7b05654bad35b3a68eedc7f8515306b496d75f3e73385dd1b002625024b81a02f2fd6dffb6e6d561cb7d0bd7a", "cfb88d6faf2de3a69d36195acec2e255e2af2b7d933997f348e09f6ce5758360"),
    (504, "e2f76e97606a872e317439f1a03fcd92e632e5bd4e7cbc4e97f1afc19a16fde92d77cbe546416b51640cddb92af996534dfd81edb17c4424cf1ac4d75aceeb", "18041bd4665083001fba8c5411d2d748e8abbfdcdfd9218cb02b68a78e7d4c23"),
    (512, "5a86b737eaea8ee976a0a24da63e7ed7eefad18a101c1211e2b3650c5187c2a8a650547208251f6d4237e661c7bf4c77f335390394c37fa1a9f9be836ac28509", "42e61e174fbb3897d6dd6cef3dd2802fe67b331953b06114a65c772859dfc1aa"),
];

#[test]
fn cavp_short_messages() {
    for (len, msg, md) in SHORT {
        let m = unhex(msg);
        assert_eq!(m.len() * 8, len, "Len {len}");
        assert_eq!(to_hex(&digest(&m)), md, "Len {len}");
    }
}

/// SHA256LongMsg.rsp, the first entry, Len 1304 bits (163 bytes, three blocks).
#[test]
fn cavp_long_message() {
    let msg = "451101250ec6f26652249d59dc974b7361d571a8101cdfd36aba3b5854d3ae086b5fdd4597721b66e3c0dc5d8c606d9657d0e323283a5217d1f53f2f284f57b85c8a61ac8924711f895c5ed90ef17745ed2d728abd22a5f7a13479a462d71b56c19a74a40b655c58edfe0a188ad2cf46cbf30524f65d423c837dd1ff2bf462ac4198007345bb44dbb7b1c861298cdf61982a833afc728fae1eda2f87aa2c9480858bec";
    let md = "3c593aa539fdcdae516cdf2f15000f6634185c88f505b39775fb9ab137a10aa2";
    let m = unhex(msg);
    assert_eq!(m.len() * 8, 1304);
    assert_eq!(to_hex(&digest(&m)), md);
}

fn invented(n: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * 31 + 7) % 251) as u8).collect()
}

#[test]
fn split_updates_equal_one_shot() {
    for n in 0..=130 {
        let m = invented(n);
        let whole = digest(&m);
        for s in 0..=n {
            let mut h = Sha256::new();
            h.update(&m[..s]);
            h.update(&m[s..]);
            assert_eq!(h.finish(), whole, "length {n}, split {s}");
        }
    }
    let big = invented(200_000);
    let whole = digest(&big);
    for chunk in [1usize, 63, 64, 65, 4096] {
        let mut h = Sha256::default();
        for part in big.chunks(chunk) {
            h.update(part);
        }
        assert_eq!(h.finish(), whole, "chunk {chunk}");
    }
}

#[test]
fn derived_constants_spot_values() {
    assert_eq!(K[0], 0x428a2f98);
    assert_eq!(K[63], 0xc67178f2);
    assert_eq!(H0[0], 0x6a09e667);
    assert_eq!(H0[7], 0x5be0cd19);
}

#[test]
fn hex_text() {
    let d = digest(&invented(10));
    let t = to_hex(&d);
    assert_eq!(t.len(), 64);
    assert!(t
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    assert_eq!(from_hex(&t), Some(d));
    assert_eq!(from_hex(&t[..63]), None);
    assert_eq!(from_hex(&format!("{t}0")), None);
    assert!(t.bytes().any(|b| b.is_ascii_lowercase()));
    assert_eq!(from_hex(&t.to_uppercase()), None);
    assert_eq!(from_hex(&format!("g{}", &t[1..])), None);
    assert_eq!(from_hex(&format!(" {}", &t[1..])), None);
}
