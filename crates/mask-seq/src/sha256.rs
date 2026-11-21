//! SHA-256 as FIPS 180-4 defines it (https://csrc.nist.gov/pubs/fips/180-4/upd1/final), written
//! from the standard alone. The round constants `K` (section 4.2.2) and the initial hash value
//! `H0` (section 5.3.3) are computed at compile time from their definitions: the fractional parts
//! of the cube roots of the first 64 primes and of the square roots of the first 8 primes, each
//! taken to 32 bits. No table of those 72 values is typed in this file.

const fn first_primes() -> [u64; 64] {
    let mut primes = [0u64; 64];
    let mut found = 0;
    let mut n = 2u64;
    while found < 64 {
        let mut is_prime = true;
        let mut d = 2u64;
        while d * d <= n {
            if n.is_multiple_of(d) {
                is_prime = false;
                break;
            }
            d += 1;
        }
        if is_prime {
            primes[found] = n;
            found += 1;
        }
        n += 1;
    }
    primes
}

/// The largest `x` below 2^36 with `x^power <= n`, by binary search.
const fn int_root(n: u128, power: u32) -> u128 {
    let mut lo = 0u128;
    let mut hi = 1u128 << 36;
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let mut p = 1u128;
        let mut i = 0;
        while i < power {
            p *= mid;
            i += 1;
        }
        if p <= n {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

const fn round_constants() -> [u32; 64] {
    let primes = first_primes();
    let mut k = [0u32; 64];
    let mut i = 0;
    while i < 64 {
        let root = int_root((primes[i] as u128) << 96, 3);
        k[i] = (root & 0xffff_ffff) as u32;
        i += 1;
    }
    k
}

const fn initial_hash() -> [u32; 8] {
    let primes = first_primes();
    let mut h = [0u32; 8];
    let mut i = 0;
    while i < 8 {
        let root = int_root((primes[i] as u128) << 64, 2);
        h[i] = (root & 0xffff_ffff) as u32;
        i += 1;
    }
    h
}

/// The 64 round constants of FIPS 180-4, section 4.2.2.
pub const K: [u32; 64] = round_constants();

/// The initial hash value of FIPS 180-4, section 5.3.3.
pub const H0: [u32; 8] = initial_hash();

/// An incremental SHA-256 computation.
#[derive(Clone, Debug)]
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    block_len: usize,
    total: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for (t, word) in w.iter_mut().take(16).enumerate() {
        *word = u32::from_be_bytes([
            block[4 * t],
            block[4 * t + 1],
            block[4 * t + 2],
            block[4 * t + 3],
        ]);
    }
    for t in 16..64 {
        let s0 = w[t - 15].rotate_right(7) ^ w[t - 15].rotate_right(18) ^ (w[t - 15] >> 3);
        let s1 = w[t - 2].rotate_right(17) ^ w[t - 2].rotate_right(19) ^ (w[t - 2] >> 10);
        w[t] = s1
            .wrapping_add(w[t - 7])
            .wrapping_add(s0)
            .wrapping_add(w[t - 16]);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for t in 0..64 {
        let big_s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(big_s1)
            .wrapping_add(ch)
            .wrapping_add(K[t])
            .wrapping_add(w[t]);
        let big_s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = big_s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (s, v) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *s = s.wrapping_add(v);
    }
}

impl Sha256 {
    /// A computation over the empty message so far.
    pub fn new() -> Sha256 {
        Sha256 {
            state: H0,
            block: [0; 64],
            block_len: 0,
            total: 0,
        }
    }

    /// Adds bytes to the message.
    pub fn update(&mut self, data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        let mut rest = data;
        if self.block_len > 0 {
            let take = (64 - self.block_len).min(rest.len());
            self.block[self.block_len..self.block_len + take].copy_from_slice(&rest[..take]);
            self.block_len += take;
            rest = &rest[take..];
            if self.block_len < 64 {
                return;
            }
            let block = self.block;
            compress(&mut self.state, &block);
            self.block_len = 0;
        }
        while rest.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&rest[..64]);
            compress(&mut self.state, &block);
            rest = &rest[64..];
        }
        self.block[..rest.len()].copy_from_slice(rest);
        self.block_len = rest.len();
    }

    /// Pads the message (FIPS 180-4, section 5.1.1) and returns the 32 byte digest.
    pub fn finish(mut self) -> [u8; 32] {
        let bit_len = self.total.wrapping_mul(8);
        let len = self.block_len;
        self.block[len] = 0x80;
        for b in self.block[len + 1..].iter_mut() {
            *b = 0;
        }
        if len >= 56 {
            let block = self.block;
            compress(&mut self.state, &block);
            self.block = [0; 64];
        }
        self.block[56..].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.block;
        compress(&mut self.state, &block);
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

/// The SHA-256 digest of `data`.
pub fn digest(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finish()
}

/// A digest as 64 lower-case hexadecimal digits.
pub fn to_hex(d: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for b in d {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

/// A digest from exactly 64 characters of `0-9a-f`; anything else gives `None`.
pub fn from_hex(s: &str) -> Option<[u8; 32]> {
    let bytes = s.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let nibble = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, pair) in bytes.chunks(2).enumerate() {
        out[i] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(out)
}
