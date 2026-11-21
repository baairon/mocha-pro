//! Hexadecimal float text, canonical form only.

/// Format a double: `0x1.8000000000000p+1`, `-0x0.0p+0`, `inf`, `-inf`, `nan`.
pub fn fmt_hex(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    let sign = if v.is_sign_negative() { "-" } else { "" };
    if v.is_infinite() {
        return format!("{sign}inf");
    }
    let bits = v.to_bits();
    let exp_field = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & ((1u64 << 52) - 1);
    if exp_field == 0 && frac == 0 {
        return format!("{sign}0x0.0p+0");
    }
    let (lead, exp) = if exp_field == 0 {
        (0, -1022)
    } else {
        (1, exp_field - 1023)
    };
    let esign = if exp < 0 { '-' } else { '+' };
    format!("{sign}0x{lead}.{frac:013x}p{esign}{}", exp.abs())
}

/// Parse text produced by `fmt_hex`. Any other spelling is rejected, with one exception: an
/// exponent written `-0` is read as `+0`. NaN keeps no payload.
pub fn parse_hex(s: &str) -> Option<f64> {
    match s {
        "nan" => return Some(f64::NAN),
        "inf" => return Some(f64::INFINITY),
        "-inf" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let rest = rest.strip_prefix("0x")?;
    let (mant, exp) = rest.split_once('p')?;
    let (lead, frac) = mant.split_once('.')?;
    let sign_ok = exp.starts_with('+') || exp.starts_with('-');
    if !sign_ok || exp.len() < 2 || !exp[1..].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if exp.len() > 2 && exp.as_bytes()[1] == b'0' {
        return None;
    }
    let e: i64 = exp.parse().ok()?;
    let signbit = u64::from(neg) << 63;
    if lead == "0" && frac == "0" && e == 0 {
        return Some(f64::from_bits(signbit));
    }
    if frac.len() != 13 || !frac.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let f = u64::from_str_radix(frac, 16).ok()?;
    let bits = match lead {
        "1" if (-1022..=1023).contains(&e) => ((e + 1023) as u64) << 52 | f,
        "0" if e == -1022 && f != 0 => f,
        _ => return None,
    };
    Some(f64::from_bits(signbit | bits))
}
