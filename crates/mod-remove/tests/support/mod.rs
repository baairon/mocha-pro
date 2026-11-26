#![allow(dead_code)]
use mod_remove::fill::SynthResult;
use mod_remove::*;

pub const IDENT: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

pub fn translate(tx: f32, ty: f32) -> [f32; 9] {
    [1.0, 0.0, tx, 0.0, 1.0, ty, 0.0, 0.0, 1.0]
}
pub fn img_fn(win: Win, f: impl Fn(i32, i32) -> [f32; 4]) -> Image {
    let mut img = Image::new(win, [0.0; 4]);
    for y in win.t..win.t + win.h {
        for x in win.l..win.l + win.w {
            img.set(x, y, f(x, y));
        }
    }
    img
}
pub fn const_img(win: Win, c: [f32; 4]) -> Image {
    Image::new(win, c)
}
pub fn src_of(image: Image) -> Source {
    let w = image.win;
    Source {
        pixels: image,
        cover: Matte::new(w, 255),
        h: IDENT,
        cam: Camera::identity(),
        layer_id: 1,
        weight: 1.0,
    }
}
pub fn rect_matte(win: Win, x0: i32, y0: i32, x1: i32, y1: i32) -> Matte {
    let mut m = Matte::new(win, 0);
    for y in y0..=y1 {
        for x in x0..=x1 {
            m.set(x, y, 255);
        }
    }
    m
}
pub fn bits(c: [f32; 4]) -> [u32; 4] {
    [
        c[0].to_bits(),
        c[1].to_bits(),
        c[2].to_bits(),
        c[3].to_bits(),
    ]
}
pub fn run(fill: &Matte, sources: &[Source], spread: bool, r: &FillConfig) -> SynthResult {
    fill::run_fill(
        &fill::FillInputs {
            fill,
            tc: Camera::identity(),
            sources,
            illum: &illum::IllumSet::None,
            layer_filter: None,
            separate_weight: false,
            spread,
        },
        r,
    )
    .unwrap()
}
pub fn sidx_with(
    win: Win,
    channels: usize,
    vals: &[(i32, i32, usize, u16)],
) -> source_index::SourceIndex {
    let mut p = source_index::SourceIndex::new(win, channels);
    for &(x, y, c, v) in vals {
        p.set(c, x, y, v);
    }
    p
}
