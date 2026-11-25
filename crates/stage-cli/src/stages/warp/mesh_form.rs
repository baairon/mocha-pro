//! The mesh file: the text form `mesh-1` of a triangle mesh with a source and a destination
//! position per vertex, given to a stage as `--layer mesh=PATH`.
//!
//! The form, line by line:
//!
//! ```text
//! mesh-1
//! # a comment
//! vertex SX SY DX DY
//! triangle A B C
//! ```
//!
//! - The first line that is not empty and not a comment is the word `mesh-1`.
//! - An empty line and a line whose first word starts with `#` are skipped. Words are separated
//!   by spaces or tabs.
//! - `vertex SX SY DX DY` adds a vertex: its position in the source frame and its position in
//!   the output frame, four finite decimal numbers. Vertices are numbered from 0 in the order of
//!   the file.
//! - `triangle A B C` adds a triangle of three vertex numbers. A vertex may be named before the
//!   line that gives it.
//! - The unit is the pixel: x grows to the right and y grows down, a frame of w by h pixels
//!   spans 0 to w and 0 to h, and pixel i has its centre at i + 0.5.
//! - The file holds one mesh, which is used for every frame.
//!
//! A file that does not follow the form, a file without a triangle, and a triangle that names
//! a vertex the file does not have are errors of the file ([`parse`] answers one line that
//! names the line of the file). Whether triangles of no area and of mixed winding are taken is
//! a choice of the stage, judged by `mesh_warp::Mesh::validate`.

use mesh_warp::{Mesh, Vec2, Vertex};

use crate::params::parse_float;

/// The id of the mesh layer.
pub const LAYER_ID: &str = "mesh";
/// The form word of the mesh file, its first line.
pub const FORM: &str = "mesh-1";

fn numbers<const N: usize>(words: &[&str], line: usize) -> Result<[f64; N], String> {
    if words.len() != N + 1 {
        return Err(format!(
            "line {line}: `{}` takes {N} values, found {}",
            words[0],
            words.len() - 1
        ));
    }
    let mut out = [0.0; N];
    for (slot, word) in out.iter_mut().zip(&words[1..]) {
        *slot = parse_float(word)
            .ok_or_else(|| format!("line {line}: cannot parse `{word}` as a number"))?;
    }
    Ok(out)
}

fn index(word: &str, line: usize) -> Result<usize, String> {
    if word.is_empty() || !word.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("line {line}: `{word}` is not a vertex number"));
    }
    word.parse()
        .map_err(|_| format!("line {line}: `{word}` is not a vertex number"))
}

/// Parse the text of a mesh file. The error is one line that names the line of the file.
pub fn parse(text: &str) -> Result<Mesh, String> {
    let mut mesh = Mesh {
        vertices: Vec::new(),
        triangles: Vec::new(),
    };
    let mut lines_of_triangles = Vec::new();
    let mut seen_form = false;
    for (at, raw) in text.lines().enumerate() {
        let line = at + 1;
        let words: Vec<&str> = raw.split_whitespace().collect();
        if words.is_empty() || words[0].starts_with('#') {
            continue;
        }
        if !seen_form {
            if words != [FORM] {
                return Err(format!("line {line}: the first line is not `{FORM}`"));
            }
            seen_form = true;
            continue;
        }
        match words[0] {
            "vertex" => {
                let [sx, sy, dx, dy] = numbers::<4>(&words, line)?;
                mesh.vertices.push(Vertex {
                    src: Vec2::new(sx, sy),
                    dst: Vec2::new(dx, dy),
                });
            }
            "triangle" => {
                if words.len() != 4 {
                    return Err(format!(
                        "line {line}: `triangle` takes 3 vertex numbers, found {}",
                        words.len() - 1
                    ));
                }
                mesh.triangles.push([
                    index(words[1], line)?,
                    index(words[2], line)?,
                    index(words[3], line)?,
                ]);
                lines_of_triangles.push(line);
            }
            other => {
                return Err(format!(
                    "line {line}: `{other}` is not `vertex` or `triangle`"
                ))
            }
        }
    }
    if !seen_form {
        return Err(format!("the file is empty: the first line is `{FORM}`"));
    }
    if mesh.triangles.is_empty() {
        return Err("the file has no triangle".to_string());
    }
    for (triangle, line) in mesh.triangles.iter().zip(&lines_of_triangles) {
        if let Some(bad) = triangle.iter().find(|i| **i >= mesh.vertices.len()) {
            return Err(format!(
                "line {line}: vertex {bad} is not in the file, which has {} vertices",
                mesh.vertices.len()
            ));
        }
    }
    Ok(mesh)
}

/// Write a mesh in the form [`parse`] reads. Every number is written as the shortest text that
/// reads back as the same value.
pub fn write(mesh: &Mesh) -> String {
    let mut text = format!("{FORM}\n");
    for v in &mesh.vertices {
        text.push_str(&format!(
            "vertex {} {} {} {}\n",
            v.src.x, v.src.y, v.dst.x, v.dst.y
        ));
    }
    for [a, b, c] in &mesh.triangles {
        text.push_str(&format!("triangle {a} {b} {c}\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = "# a square of two triangles\n\nmesh-1\nvertex 0 0 1 0\nvertex 4 0 5 0\n\tvertex 4 2 5 2.5\nvertex 0 2 1 2\ntriangle 0 1 2\ntriangle 0 2 3\n";

    #[test]
    fn a_file_is_read_line_by_line() {
        let mesh = parse(SQUARE).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.vertices[2].src, Vec2::new(4.0, 2.0));
        assert_eq!(mesh.vertices[2].dst, Vec2::new(5.0, 2.5));
        assert_eq!(mesh.triangles, [[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn a_written_mesh_reads_back_equal() {
        let mesh =
            mesh_warp::grid_mesh(3, 2, 0.5, -1.25, 7.0, 3.0, mesh_warp::Diagonal::Alternating);
        let text = write(&mesh);
        assert!(text.starts_with("mesh-1\nvertex 0.5 -1.25 0.5 -1.25\n"));
        assert_eq!(parse(&text).unwrap(), mesh);
        assert_eq!(write(&parse(SQUARE).unwrap()).lines().count(), 7);
    }

    #[test]
    fn a_triangle_may_come_before_its_vertices() {
        let text = "mesh-1\ntriangle 0 1 2\nvertex 0 0 0 0\nvertex 1 0 1 0\nvertex 0 1 0 1\n";
        assert_eq!(parse(text).unwrap().triangles, [[0, 1, 2]]);
    }

    #[test]
    fn every_error_names_its_line() {
        for (text, want) in [
            ("", "the file is empty: the first line is `mesh-1`"),
            (
                "# only a comment\n",
                "the file is empty: the first line is `mesh-1`",
            ),
            ("mesh-2\n", "line 1: the first line is not `mesh-1`"),
            ("\nmesh-1 extra\n", "line 2: the first line is not `mesh-1`"),
            ("vertex 0 0 0 0\n", "line 1: the first line is not `mesh-1`"),
            (
                "mesh-1\nvertex 0 0 0\n",
                "line 2: `vertex` takes 4 values, found 3",
            ),
            (
                "mesh-1\nvertex 0 0 0 0 0\n",
                "line 2: `vertex` takes 4 values, found 5",
            ),
            (
                "mesh-1\nvertex 0 x 0 0\n",
                "line 2: cannot parse `x` as a number",
            ),
            (
                "mesh-1\nvertex 0 inf 0 0\n",
                "line 2: cannot parse `inf` as a number",
            ),
            (
                "mesh-1\nvertex 0 1e999 0 0\n",
                "line 2: cannot parse `1e999` as a number",
            ),
            (
                "mesh-1\ntriangle 0 1\n",
                "line 2: `triangle` takes 3 vertex numbers, found 2",
            ),
            (
                "mesh-1\ntriangle 0 1 -2\n",
                "line 2: `-2` is not a vertex number",
            ),
            (
                "mesh-1\ntriangle 0 1 2.0\n",
                "line 2: `2.0` is not a vertex number",
            ),
            (
                "mesh-1\ntriangle 0 1 99999999999999999999999\n",
                "line 2: `99999999999999999999999` is not a vertex number",
            ),
            (
                "mesh-1\nquad 0 1 2 3\n",
                "line 2: `quad` is not `vertex` or `triangle`",
            ),
            ("mesh-1\nvertex 0 0 0 0\n", "the file has no triangle"),
            (
                "mesh-1\nvertex 0 0 0 0\nvertex 1 0 1 0\n\ntriangle 0 1 2\n",
                "line 5: vertex 2 is not in the file, which has 2 vertices",
            ),
        ] {
            assert_eq!(parse(text).unwrap_err(), want, "{text:?}");
        }
    }
}
