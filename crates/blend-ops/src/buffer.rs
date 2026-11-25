use crate::config::Config;
use crate::modes::{BlendMode, PorterDuff};
use crate::real::Real;
use crate::{composite_pixel, Rgba};

/// A buffer length problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    /// A colour buffer length is not a multiple of 4.
    NotWholePixels,
    /// The source, backdrop, output or matte do not describe the same number of pixels.
    LengthMismatch,
}

/// Composites the interleaved RGBA buffer `src` over `dst` into `out` (all `4 * n` values), with
/// an optional one channel `matte` of `n` values. The matte value `m` multiplies the source
/// alpha, `as = as * m`, in the straight domain before the opacity point of the configuration;
/// a premultiplied source has its colour scaled by `m` as well, so its straight colour is unchanged.
#[allow(clippy::too_many_arguments)]
pub fn composite_buffers<T: Real>(
    src: &[T],
    dst: &[T],
    out: &mut [T],
    matte: Option<&[T]>,
    opacity: T,
    blend: BlendMode,
    pd: PorterDuff,
    cfg: &Config,
) -> Result<(), BufferError> {
    if !src.len().is_multiple_of(4) || !dst.len().is_multiple_of(4) || !out.len().is_multiple_of(4)
    {
        return Err(BufferError::NotWholePixels);
    }
    let n = src.len() / 4;
    if dst.len() != src.len() || out.len() != src.len() || matte.is_some_and(|m| m.len() != n) {
        return Err(BufferError::LengthMismatch);
    }
    for i in 0..n {
        let k = 4 * i;
        let mut s = Rgba::new(src[k], src[k + 1], src[k + 2], src[k + 3]);
        if let Some(m) = matte {
            s = match cfg.input_form {
                crate::Form::Straight => Rgba::new(s.r, s.g, s.b, s.a * m[i]),
                crate::Form::Premultiplied => {
                    Rgba::new(s.r * m[i], s.g * m[i], s.b * m[i], s.a * m[i])
                }
            };
        }
        let d = Rgba::new(dst[k], dst[k + 1], dst[k + 2], dst[k + 3]);
        let r = composite_pixel(s, d, blend, pd, opacity, cfg);
        out[k] = r.r;
        out[k + 1] = r.g;
        out[k + 2] = r.b;
        out[k + 3] = r.a;
    }
    Ok(())
}
