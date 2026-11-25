//! Mesh, point location and the two maps.

use crate::{cross, Vec2, Vertex};

/// Which of the two vertex position sets is meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Src,
    Dst,
}

/// Who owns a point on a shared edge or vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieRule {
    /// The containing triangle (closed triangle, edges included) of lowest index.
    LowestIndex,
    /// The top left rule (Pineda 1988): the first triangle that owns the point, where a
    /// triangle owns its interior, its top and left edges, and its vertices only through those
    /// edges. Each triangle is first oriented to positive area; an edge `s -> e` with
    /// `d = e - s` is owned when `d.y < 0` or (`d.y == 0` and `d.x > 0`), which is the top and
    /// left edge for a y axis pointing down. A point on the mesh border that no triangle owns
    /// falls back to the lowest index closed containment.
    TopLeft,
}

/// Behaviour for a point that lies in no triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outside {
    /// No result.
    None,
    /// The point of the nearest triangle boundary is mapped (barycentrics of that boundary
    /// point). Nearest by squared distance, ties to the lowest triangle index.
    ClampNearest,
    /// The affine map of the nearest triangle (as for `ClampNearest`) is extended unclamped.
    ExtendAffine,
}

/// Winding validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winding {
    /// Any orientation of each triangle.
    Any,
    /// All non degenerate triangles of a space have the same area sign.
    Consistent,
}

/// The formula and operation order of the barycentric coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaryForm {
    /// `area = (b-a) x (c-a)`; `la = ((c-b) x (p-b)) / area`, `lb = ((a-c) x (p-c)) / area`,
    /// `lc = ((b-a) x (p-a)) / area` (Ericson 2005, section 3.4, edge function form).
    EdgeFunctions,
    /// `lb = ((p-a) x (c-a)) / area`, `lc = ((b-a) x (p-a)) / area`, `la = (1 - lb) - lc`
    /// (Cramer's rule on the two edge vectors, Wolberg 1990).
    Cramer,
}

/// Grid acceleration structure sizing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridRes {
    /// `n x n` cells with `n = max(1, floor(sqrt(count)))` (integer square root) for `count`
    /// non degenerate triangles.
    PerTriangleSqrt,
    /// `nx x ny` cells, each at least 1.
    Fixed(usize, usize),
}

/// All choices. The default claims nothing: lowest index ties, no outside result, any winding,
/// edge function barycentrics, degenerate triangles refused, square root grid.
///
/// Cell rule: the grid covers the bounding box of the non degenerate triangles of the space, with
/// cell size `(max - min) / n` per axis; the cell of a coordinate is
/// `clamp(floor((v - min) / size), 0, n - 1)` (0 when the size is 0). A triangle is listed, in
/// index order, in every cell from the cell of its box minimum to the cell of its box maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub tie: TieRule,
    pub outside: Outside,
    pub winding: Winding,
    pub bary: BaryForm,
    /// Accept triangles of exactly zero area (they are ignored by location).
    pub allow_degenerate: bool,
    pub grid: GridRes,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tie: TieRule::LowestIndex,
            outside: Outside::None,
            winding: Winding::Any,
            bary: BaryForm::EdgeFunctions,
            allow_degenerate: false,
            grid: GridRes::PerTriangleSqrt,
        }
    }
}

/// Every combination of the discrete choices (96 configurations; grid is `PerTriangleSqrt` or
/// `Fixed(1, 1)`).
pub fn config_space() -> Vec<Config> {
    let mut v = Vec::new();
    for tie in [TieRule::LowestIndex, TieRule::TopLeft] {
        for outside in [Outside::None, Outside::ClampNearest, Outside::ExtendAffine] {
            for winding in [Winding::Any, Winding::Consistent] {
                for bary in [BaryForm::EdgeFunctions, BaryForm::Cramer] {
                    for allow_degenerate in [false, true] {
                        for grid in [GridRes::PerTriangleSqrt, GridRes::Fixed(1, 1)] {
                            v.push(Config {
                                tie,
                                outside,
                                winding,
                                bary,
                                allow_degenerate,
                                grid,
                            });
                        }
                    }
                }
            }
        }
    }
    v
}

/// Reasons a mesh is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshError {
    NoTriangles,
    IndexOutOfRange { triangle: usize, index: usize },
    NonFinite { vertex: usize },
    Degenerate { triangle: usize },
    InconsistentWinding { triangle: usize },
}

/// Vertices and triangles (indices into the vertex list).
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[usize; 3]>,
}

fn pos(v: &Vertex, s: Space) -> Vec2 {
    match s {
        Space::Src => v.src,
        Space::Dst => v.dst,
    }
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

/// Signed area times two: `(b-a) x (c-a)`.
fn area2(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    cross(sub(b, a), sub(c, a))
}

impl Mesh {
    /// Corner positions of triangle `t` in a space.
    pub fn corners(&self, t: usize, s: Space) -> [Vec2; 3] {
        let i = self.triangles[t];
        [
            pos(&self.vertices[i[0]], s),
            pos(&self.vertices[i[1]], s),
            pos(&self.vertices[i[2]], s),
        ]
    }

    /// Twice the signed area of triangle `t` in a space.
    pub fn area2(&self, t: usize, s: Space) -> f64 {
        let [a, b, c] = self.corners(t, s);
        area2(a, b, c)
    }

    /// Adds an offset to every destination position.
    pub fn translate_dst(&mut self, dx: f64, dy: f64) {
        for v in &mut self.vertices {
            v.dst = Vec2::new(v.dst.x + dx, v.dst.y + dy);
        }
    }

    /// Checks finiteness of the vertices, indices, degenerate triangles and winding, in that order.
    pub fn validate(&self, cfg: &Config) -> Result<(), MeshError> {
        if self.triangles.is_empty() {
            return Err(MeshError::NoTriangles);
        }
        for (i, v) in self.vertices.iter().enumerate() {
            if ![v.src.x, v.src.y, v.dst.x, v.dst.y]
                .iter()
                .all(|f| f.is_finite())
            {
                return Err(MeshError::NonFinite { vertex: i });
            }
        }
        for (t, tri) in self.triangles.iter().enumerate() {
            for &index in tri {
                if index >= self.vertices.len() {
                    return Err(MeshError::IndexOutOfRange { triangle: t, index });
                }
            }
        }
        for s in [Space::Src, Space::Dst] {
            let mut sign = 0.0f64;
            for t in 0..self.triangles.len() {
                let a = self.area2(t, s);
                if a == 0.0 {
                    if !cfg.allow_degenerate {
                        return Err(MeshError::Degenerate { triangle: t });
                    }
                    continue;
                }
                if cfg.winding == Winding::Consistent {
                    if sign == 0.0 {
                        sign = a.signum();
                    } else if a.signum() != sign {
                        return Err(MeshError::InconsistentWinding { triangle: t });
                    }
                }
            }
        }
        Ok(())
    }
}

/// Which way each grid cell is cut into two triangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diagonal {
    /// From the cell's top left vertex to its bottom right vertex.
    Forward,
    /// From the top right vertex to the bottom left vertex.
    Back,
    /// `Forward` where `(i + j)` is even, else `Back`.
    Alternating,
}

/// A regular `cols x rows` grid over the box `(x0, y0)` to `(x0 + w, y0 + h)`, destination equal
/// to source. Vertex `(i, j)` has index `j * (cols + 1) + i` and position
/// `(x0 + (w * i) / cols, y0 + (h * j) / rows)`. Each cell gives two triangles, positive area
/// for positive `w` and `h`: `Forward` is (v00, v10, v11) then (v00, v11, v01); `Back` is
/// (v00, v10, v01) then (v10, v11, v01). Cells run row by row.
pub fn grid_mesh(cols: usize, rows: usize, x0: f64, y0: f64, w: f64, h: f64, d: Diagonal) -> Mesh {
    let mut vertices = Vec::new();
    for j in 0..=rows {
        for i in 0..=cols {
            let p = Vec2::new(
                x0 + (w * i as f64) / cols as f64,
                y0 + (h * j as f64) / rows as f64,
            );
            vertices.push(Vertex { src: p, dst: p });
        }
    }
    let mut triangles = Vec::new();
    for j in 0..rows {
        for i in 0..cols {
            let v00 = j * (cols + 1) + i;
            let v10 = v00 + 1;
            let v01 = v00 + cols + 1;
            let v11 = v01 + 1;
            let fwd = match d {
                Diagonal::Forward => true,
                Diagonal::Back => false,
                Diagonal::Alternating => (i + j) % 2 == 0,
            };
            if fwd {
                triangles.push([v00, v10, v11]);
                triangles.push([v00, v11, v01]);
            } else {
                triangles.push([v00, v10, v01]);
                triangles.push([v10, v11, v01]);
            }
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}

/// Barycentric coordinates `[la, lb, lc]` of `p` in triangle `a, b, c`, or `None` for zero area.
pub fn barycentric(a: Vec2, b: Vec2, c: Vec2, p: Vec2, form: BaryForm) -> Option<[f64; 3]> {
    let area = area2(a, b, c);
    if area == 0.0 {
        return None;
    }
    Some(match form {
        BaryForm::EdgeFunctions => [
            cross(sub(c, b), sub(p, b)) / area,
            cross(sub(a, c), sub(p, c)) / area,
            cross(sub(b, a), sub(p, a)) / area,
        ],
        BaryForm::Cramer => {
            let lb = cross(sub(p, a), sub(c, a)) / area;
            let lc = cross(sub(b, a), sub(p, a)) / area;
            [(1.0 - lb) - lc, lb, lc]
        }
    })
}

fn interp(a: Vec2, b: Vec2, c: Vec2, l: [f64; 3]) -> Vec2 {
    Vec2::new(
        (l[0] * a.x + l[1] * b.x) + l[2] * c.x,
        (l[0] * a.y + l[1] * b.y) + l[2] * c.y,
    )
}

/// `(e - s) x (q - s)`, evaluated from the lexicographically smaller end point so that the
/// reversed segment gives exactly the negated value.
fn edge_fn(s: Vec2, e: Vec2, q: Vec2) -> f64 {
    if (s.x, s.y) <= (e.x, e.y) {
        cross(sub(e, s), sub(q, s))
    } else {
        -cross(sub(s, e), sub(q, e))
    }
}

#[derive(Clone, Debug)]
struct Prep {
    index: usize,
    p: [Vec2; 3],
    sign: f64,
}

impl Prep {
    /// Edge functions, edge `i` opposite vertex `i`, oriented to be non negative inside. A
    /// segment shared by two triangles gives bit identical values of opposite sign in both, so a
    /// point is never refused by both neighbours through rounding.
    fn edges(&self, q: Vec2) -> [f64; 3] {
        let [a, b, c] = self.p;
        [
            edge_fn(b, c, q) * self.sign,
            edge_fn(c, a, q) * self.sign,
            edge_fn(a, b, q) * self.sign,
        ]
    }

    fn owns_edge(&self, i: usize) -> bool {
        let mut s = self.p[(i + 1) % 3];
        let mut e = self.p[(i + 2) % 3];
        if self.sign < 0.0 {
            std::mem::swap(&mut s, &mut e);
        }
        let d = sub(e, s);
        d.y < 0.0 || (d.y == 0.0 && d.x > 0.0)
    }

    fn contains(&self, q: Vec2) -> bool {
        self.edges(q).iter().all(|&e| e >= 0.0)
    }

    fn owns(&self, q: Vec2) -> bool {
        let e = self.edges(q);
        (0..3).all(|i| e[i] > 0.0 || (e[i] == 0.0 && self.owns_edge(i)))
    }

    /// Squared distance to the boundary and the barycentrics of the nearest boundary point.
    fn nearest_boundary(&self, q: Vec2) -> (f64, [f64; 3]) {
        let mut best = (f64::INFINITY, [0.0; 3]);
        for i in 0..3 {
            let s = self.p[i];
            let e = self.p[(i + 1) % 3];
            let d = sub(e, s);
            let len2 = d.x * d.x + d.y * d.y;
            let w = sub(q, s);
            let t = ((w.x * d.x + w.y * d.y) / len2).clamp(0.0, 1.0);
            let r = Vec2::new(s.x + t * d.x, s.y + t * d.y);
            let dd = sub(q, r);
            let d2 = dd.x * dd.x + dd.y * dd.y;
            if d2 < best.0 {
                let mut l = [0.0; 3];
                l[i] = 1.0 - t;
                l[(i + 1) % 3] = t;
                best = (d2, l);
            }
        }
        best
    }
}

/// Where a point was found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Located {
    /// Index of the triangle in the mesh.
    pub triangle: usize,
    /// Barycentric coordinates used for the map.
    pub bary: [f64; 3],
    /// True when the point lies in no triangle and an outside rule produced this result.
    pub outside: bool,
}

/// Point location in one space of a mesh.
#[derive(Clone, Debug)]
pub struct Locator {
    space: Space,
    cfg: Config,
    tris: Vec<Prep>,
    min: Vec2,
    size: Vec2,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<usize>>,
}

fn isqrt(n: usize) -> usize {
    let mut r = 0usize;
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r
}

fn cell_coord(v: f64, min: f64, size: f64, n: usize) -> usize {
    if size <= 0.0 {
        return 0;
    }
    let f = ((v - min) / size).floor();
    if f <= 0.0 {
        0
    } else if f >= n as f64 {
        n - 1
    } else {
        f as usize
    }
}

impl Locator {
    /// Builds the grid over the non degenerate triangles of `space`. The mesh must be valid.
    pub fn build(mesh: &Mesh, space: Space, cfg: &Config) -> Locator {
        let mut tris = Vec::new();
        for t in 0..mesh.triangles.len() {
            let p = mesh.corners(t, space);
            let a = area2(p[0], p[1], p[2]);
            if a != 0.0 {
                let sign = if a < 0.0 { -1.0 } else { 1.0 };
                tris.push(Prep { index: t, p, sign });
            }
        }
        let (nx, ny) = match cfg.grid {
            GridRes::PerTriangleSqrt => {
                let n = isqrt(tris.len()).max(1);
                (n, n)
            }
            GridRes::Fixed(x, y) => (x.max(1), y.max(1)),
        };
        let mut lo = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for t in &tris {
            for q in &t.p {
                lo = Vec2::new(lo.x.min(q.x), lo.y.min(q.y));
                hi = Vec2::new(hi.x.max(q.x), hi.y.max(q.y));
            }
        }
        if tris.is_empty() {
            lo = Vec2::new(0.0, 0.0);
            hi = lo;
        }
        let size = Vec2::new((hi.x - lo.x) / nx as f64, (hi.y - lo.y) / ny as f64);
        let mut cells = vec![Vec::new(); nx * ny];
        for (k, t) in tris.iter().enumerate() {
            let (mut bl, mut bh) = (t.p[0], t.p[0]);
            for q in &t.p[1..] {
                bl = Vec2::new(bl.x.min(q.x), bl.y.min(q.y));
                bh = Vec2::new(bh.x.max(q.x), bh.y.max(q.y));
            }
            let (x0, x1) = (
                cell_coord(bl.x, lo.x, size.x, nx),
                cell_coord(bh.x, lo.x, size.x, nx),
            );
            let (y0, y1) = (
                cell_coord(bl.y, lo.y, size.y, ny),
                cell_coord(bh.y, lo.y, size.y, ny),
            );
            for cy in y0..=y1 {
                for cx in x0..=x1 {
                    cells[cy * nx + cx].push(k);
                }
            }
        }
        Locator {
            space,
            cfg: *cfg,
            tris,
            min: lo,
            size,
            nx,
            ny,
            cells,
        }
    }

    /// Grid dimensions `(nx, ny)`.
    pub fn grid_dims(&self) -> (usize, usize) {
        (self.nx, self.ny)
    }

    /// Number of triangles listed in the cell holding `p`.
    pub fn cell_len(&self, p: Vec2) -> usize {
        self.cells[self.cell_of(p)].len()
    }

    fn cell_of(&self, p: Vec2) -> usize {
        cell_coord(p.y, self.min.y, self.size.y, self.ny) * self.nx
            + cell_coord(p.x, self.min.x, self.size.x, self.nx)
    }

    /// The triangle holding `p` under the tie rule, with no outside rule applied.
    pub fn find(&self, p: Vec2) -> Option<usize> {
        let cell = &self.cells[self.cell_of(p)];
        let first_closed = cell.iter().copied().find(|&k| self.tris[k].contains(p));
        let k = match self.cfg.tie {
            TieRule::LowestIndex => first_closed,
            TieRule::TopLeft => cell
                .iter()
                .copied()
                .find(|&k| self.tris[k].owns(p))
                .or(first_closed),
        };
        k.map(|k| self.tris[k].index)
    }

    fn prep_of(&self, index: usize) -> &Prep {
        self.tris.iter().find(|t| t.index == index).expect("listed")
    }

    /// Location with the outside rule applied.
    pub fn locate(&self, p: Vec2) -> Option<Located> {
        if let Some(t) = self.find(p) {
            let [a, b, c] = self.prep_of(t).p;
            let bary = barycentric(a, b, c, p, self.cfg.bary)?;
            return Some(Located {
                triangle: t,
                bary,
                outside: false,
            });
        }
        if self.cfg.outside == Outside::None {
            return None;
        }
        let mut best: Option<(f64, usize, [f64; 3])> = None;
        for t in &self.tris {
            let (d2, l) = t.nearest_boundary(p);
            if best.is_none_or(|b| d2 < b.0) {
                best = Some((d2, t.index, l));
            }
        }
        let (_, index, l) = best?;
        let bary = match self.cfg.outside {
            Outside::ClampNearest => l,
            _ => {
                let [a, b, c] = self.prep_of(index).p;
                barycentric(a, b, c, p, self.cfg.bary)?
            }
        };
        Some(Located {
            triangle: index,
            bary,
            outside: true,
        })
    }

    /// The space this locator indexes.
    pub fn space(&self) -> Space {
        self.space
    }
}

/// A validated mesh with a locator in each space.
#[derive(Clone, Debug)]
pub struct Warp {
    mesh: Mesh,
    dst: Locator,
    src: Locator,
}

impl Warp {
    pub fn new(mesh: Mesh, cfg: &Config) -> Result<Warp, MeshError> {
        mesh.validate(cfg)?;
        let dst = Locator::build(&mesh, Space::Dst, cfg);
        let src = Locator::build(&mesh, Space::Src, cfg);
        Ok(Warp { mesh, dst, src })
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    fn map(&self, loc: &Locator, from: Space, to: Space, p: Vec2) -> Option<Vec2> {
        let l = loc.locate(p)?;
        let [a, b, c] = self.mesh.corners(l.triangle, to);
        debug_assert_eq!(loc.space(), from);
        Some(interp(a, b, c, l.bary))
    }

    /// Inverse map: a destination point to a source position.
    pub fn inverse(&self, p: Vec2) -> Option<Vec2> {
        self.map(&self.dst, Space::Dst, Space::Src, p)
    }

    /// Forward map: a source point to a destination position.
    pub fn forward(&self, p: Vec2) -> Option<Vec2> {
        self.map(&self.src, Space::Src, Space::Dst, p)
    }

    /// Location of a destination point.
    pub fn locate_dst(&self, p: Vec2) -> Option<Located> {
        self.dst.locate(p)
    }

    /// Location of a source point.
    pub fn locate_src(&self, p: Vec2) -> Option<Located> {
        self.src.locate(p)
    }

    /// The inverse mapping a resampler needs: output `(x, y)` to source `(x, y)`.
    pub fn inverse_closure(&self) -> impl Fn(f64, f64) -> Option<(f64, f64)> + '_ {
        move |x, y| self.inverse(Vec2::new(x, y)).map(|q| (q.x, q.y))
    }
}
