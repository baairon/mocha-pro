//! Cube map faces in a layout chosen by configuration.
//!
//! Each face has coordinates (a, b) in `[-1, 1]`, a to the right and b up in the face image, and a
//! direction (a convention of this crate, see the table below; the face order and the quarter turn of
//! each face are configuration). The face is the one with the largest absolute component of the
//! direction (ties prefer x, then y, then z); the plane projection is the gnomonic cube map of any text
//! on cube mapping (for example Snyder 1987, Gnomonic Projection, the central projection on a plane).
//!
//! | face | direction (a, b) | a from direction | b from direction |
//! |------|------------------|------------------|------------------|
//! | PosX | (1, b, -a)       | -z / m           | y / m            |
//! | NegX | (-1, b, a)       | z / m            | y / m            |
//! | PosY | (a, 1, -b)       | x / m            | -z / m           |
//! | NegY | (a, -1, b)       | x / m            | z / m            |
//! | PosZ | (a, b, 1)        | x / m            | y / m            |
//! | NegZ | (-a, b, -1)      | -x / m           | y / m            |
//!
//! with m the absolute value of the major component. Face image coordinates are `u = (a + 1) / 2`,
//! `v = (1 - b) / 2`, and a face stored with `k` quarter turns maps `(u, v)` to `(1 - v, u)` k times.

use crate::equirect::{PixelPos, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ];
}

/// Arrangement of the six faces in the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeGrid {
    /// Six columns, one row.
    Row6,
    /// One column, six rows.
    Column6,
    /// Three columns, two rows.
    Grid3x2,
    /// Two columns, three rows.
    Grid2x3,
}

impl CubeGrid {
    pub const ALL: [CubeGrid; 4] = [
        CubeGrid::Row6,
        CubeGrid::Column6,
        CubeGrid::Grid3x2,
        CubeGrid::Grid2x3,
    ];

    /// (columns, rows).
    pub fn dims(self) -> (u32, u32) {
        match self {
            CubeGrid::Row6 => (6, 1),
            CubeGrid::Column6 => (1, 6),
            CubeGrid::Grid3x2 => (3, 2),
            CubeGrid::Grid2x3 => (2, 3),
        }
    }

    /// Cell (column, row) of slot `i` in `0..6`, filled row by row.
    pub fn cell(self, i: usize) -> (u32, u32) {
        let (c, _) = self.dims();
        let i = i as u32;
        (i % c, i / c)
    }
}

/// Cube layout configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubeConfig {
    pub grid: CubeGrid,
    /// The face stored in each slot, in slot order.
    pub order: [Face; 6],
    /// Quarter turns of the face image stored in each slot, `0..4`.
    pub turns: [u8; 6],
}

impl Default for CubeConfig {
    fn default() -> Self {
        CubeConfig {
            grid: CubeGrid::Row6,
            order: Face::ALL,
            turns: [0; 6],
        }
    }
}

fn permutations() -> Vec<[Face; 6]> {
    let mut out = Vec::with_capacity(720);
    let mut idx = [0usize, 1, 2, 3, 4, 5];
    loop {
        let mut o = [Face::PosX; 6];
        for (slot, &i) in idx.iter().enumerate() {
            o[slot] = Face::ALL[i];
        }
        out.push(o);
        let mut i = 4isize;
        while i >= 0 && idx[i as usize] > idx[i as usize + 1] {
            i -= 1;
        }
        if i < 0 {
            break;
        }
        let iu = i as usize;
        let mut j = 5;
        while idx[j] < idx[iu] {
            j -= 1;
        }
        idx.swap(iu, j);
        idx[iu + 1..].reverse();
    }
    out
}

impl CubeConfig {
    /// Number of configurations of [`CubeConfig::all`]: 4 grids, 720 face orders, 4 uniform turns.
    pub const COUNT: usize = 11520;

    /// Every grid with every order of the six faces and the same quarter turn on all six faces, in
    /// lexicographic order of the face indices.
    pub fn all() -> Vec<CubeConfig> {
        let perms = permutations();
        let mut out = Vec::with_capacity(Self::COUNT);
        for grid in CubeGrid::ALL {
            for order in &perms {
                for t in 0..4u8 {
                    out.push(CubeConfig {
                        grid,
                        order: *order,
                        turns: [t; 6],
                    });
                }
            }
        }
        out
    }

    /// Side of one face in pixels, or None when the size does not divide into square faces.
    pub fn face_size(&self, size: Size) -> Option<u32> {
        let (c, r) = self.grid.dims();
        if size.w == 0
            || size.h == 0
            || size.w / c * c != size.w
            || size.h / r * r != size.h
            || size.w / c != size.h / r
        {
            None
        } else {
            Some(size.w / c)
        }
    }

    fn slot_of(&self, face: Face) -> usize {
        self.order.iter().position(|&f| f == face).unwrap_or(0)
    }
}

fn quarter(u: f64, v: f64, k: u8) -> (f64, f64) {
    let (mut u, mut v) = (u, v);
    for _ in 0..(k & 3) {
        let nu = 1.0 - v;
        v = u;
        u = nu;
    }
    (u, v)
}

/// Direction (not normalised) of face coordinates (a, b).
pub fn face_to_dir(face: Face, a: f64, b: f64) -> [f64; 3] {
    match face {
        Face::PosX => [1.0, b, 0.0 - a],
        Face::NegX => [-1.0, b, a],
        Face::PosY => [a, 1.0, 0.0 - b],
        Face::NegY => [a, -1.0, b],
        Face::PosZ => [a, b, 1.0],
        Face::NegZ => [0.0 - a, b, -1.0],
    }
}

/// Face and coordinates (a, b) of a direction. The zero vector gives (PosZ, 0, 0).
pub fn dir_to_face(d: [f64; 3]) -> (Face, f64, f64) {
    let (x, y, z) = (d[0], d[1], d[2]);
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let m = ax.max(ay).max(az);
    if m == 0.0 || m.is_nan() {
        return (Face::PosZ, 0.0, 0.0);
    }
    if ax >= ay && ax >= az {
        if x >= 0.0 {
            (Face::PosX, (0.0 - z) / m, y / m)
        } else {
            (Face::NegX, z / m, y / m)
        }
    } else if ay >= az {
        if y >= 0.0 {
            (Face::PosY, x / m, (0.0 - z) / m)
        } else {
            (Face::NegY, x / m, z / m)
        }
    } else if z >= 0.0 {
        (Face::PosZ, x / m, y / m)
    } else {
        (Face::NegZ, (0.0 - x) / m, y / m)
    }
}

/// Direction of the sample at continuous pixel coordinate (x, y) of the layout image, or None when
/// the size has no square faces. Coordinates outside the image are clamped to the nearest face cell.
pub fn pixel_to_dir(cfg: &CubeConfig, size: Size, x: f64, y: f64) -> Option<[f64; 3]> {
    let fs = cfg.face_size(size)?;
    let f = f64::from(fs);
    let (cols, rows) = cfg.grid.dims();
    let cx = ((x + 0.5) / f).floor().clamp(0.0, f64::from(cols - 1));
    let cy = ((y + 0.5) / f).floor().clamp(0.0, f64::from(rows - 1));
    let slot = (0..6)
        .find(|&i| cfg.grid.cell(i) == (cx as u32, cy as u32))
        .unwrap_or(0);
    let lu = ((x + 0.5 - cx * f) / f).clamp(0.0, 1.0);
    let lv = ((y + 0.5 - cy * f) / f).clamp(0.0, 1.0);
    let (u, v) = quarter(lu, lv, (4 - (cfg.turns[slot] & 3)) & 3);
    Some(face_to_dir(cfg.order[slot], 2.0 * u - 1.0, 1.0 - 2.0 * v))
}

/// Pixel coordinate in the layout image of a direction, clamped inside its face cell (so `clamped`
/// is set when the sample lies outside the cell's pixel centres), or None without square faces.
pub fn dir_to_pixel(cfg: &CubeConfig, size: Size, d: [f64; 3]) -> Option<PixelPos> {
    let fs = cfg.face_size(size)?;
    let f = f64::from(fs);
    let (face, a, b) = dir_to_face(d);
    let slot = cfg.slot_of(face);
    let (u, v) = quarter((a + 1.0) * 0.5, (1.0 - b) * 0.5, cfg.turns[slot]);
    let (cx, cy) = cfg.grid.cell(slot);
    let lx = u * f - 0.5;
    let ly = v * f - 0.5;
    let top = f - 1.0;
    let clamped = !(0.0..=top).contains(&lx) || !(0.0..=top).contains(&ly);
    Some(PixelPos {
        x: f64::from(cx) * f + lx.clamp(0.0, top),
        y: f64::from(cy) * f + ly.clamp(0.0, top),
        wrapped: false,
        clamped,
    })
}
