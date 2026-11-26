//! Projective transfer, rounding and bilinear sampling.

use crate::types::*;

/// Round to nearest, halves away from zero, then to integer.
pub fn round_px(v: f32) -> i32 {
    v.round() as i32
}

/// Transfer of the integer pixel (x, y): back project through the target camera, multiply by the
/// homography, project through the source camera. All f32, no fused ops.
/// Only linear cameras are specified; other kinds are `Unspecified`.
pub fn transfer(tc: &Camera, sc: &Camera, h: &[f32; 9], x: i32, y: i32) -> Res<(f32, f32)> {
    if tc.kind != CamKind::Linear || sc.kind != CamKind::Linear {
        return Err(RemoveError::Unspecified(
            "lens maths of a non linear camera",
        ));
    }
    let bx = (x as f32 - tc.c0[0]) / tc.f;
    let by = (y as f32 - tc.c0[1]) / tc.f;
    let bz = tc.zh;
    let vx = h[0] * bx + h[1] * by + h[2] * bz;
    let vy = h[3] * bx + h[4] * by + h[5] * bz;
    let vz = h[6] * bx + h[7] * by + h[8] * bz;
    if vz == 0.0 {
        return Err(RemoveError::Unspecified(
            "homogeneous coordinate zero after the homography",
        ));
    }
    let px = vx / vz * sc.f + sc.c0[0];
    let py = vy / vz * sc.f + sc.c0[1];
    Ok((px, py))
}

/// Transfer then round to the nearest integer pixel.
pub fn transfer_px(tc: &Camera, sc: &Camera, h: &[f32; 9], x: i32, y: i32) -> Res<(i32, i32)> {
    let (a, b) = transfer(tc, sc, h, x, y)?;
    Ok((round_px(a), round_px(b)))
}

/// Default sampler: bilinear, unnormalised coordinates relative to the image window with pixel
/// centres at integer plus 0.5, clamp to edge, float RGBA, no premultiply.
/// The component arithmetic order (horizontal then vertical) is a platform detail.
pub fn bilinear_clamp(img: &Image, sx: f32, sy: f32) -> [f32; 4] {
    let u = sx - 0.5;
    let v = sy - 0.5;
    let x0 = u.floor();
    let y0 = v.floor();
    let fx = u - x0;
    let fy = v - y0;
    let ix = x0 as i32;
    let iy = y0 as i32;
    let tex = |i: i32, j: i32| -> [f32; 4] {
        let ci = i.clamp(0, img.win.w - 1);
        let cj = j.clamp(0, img.win.h - 1);
        img.data[(cj * img.win.w + ci) as usize]
    };
    let p00 = tex(ix, iy);
    let p10 = tex(ix + 1, iy);
    let p01 = tex(ix, iy + 1);
    let p11 = tex(ix + 1, iy + 1);
    let mut out = [0.0f32; 4];
    for k in 0..4 {
        let top = p00[k] * (1.0 - fx) + p10[k] * fx;
        let bot = p01[k] * (1.0 - fx) + p11[k] * fx;
        out[k] = top * (1.0 - fy) + bot * fy;
    }
    out
}

/// Source sample position: transfer minus the source window left top plus 0.5.
pub fn sample_position(tc: &Camera, src: &Source, x: i32, y: i32) -> Res<(f32, f32)> {
    let (tx, ty) = transfer(tc, &src.cam, &src.h, x, y)?;
    Ok((
        (tx - src.pixels.win.l as f32) + 0.5,
        (ty - src.pixels.win.t as f32) + 0.5,
    ))
}
