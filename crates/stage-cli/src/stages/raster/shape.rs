//! The shape layer of the rasterisation stages: the text forms `rings-1` and `contours-1`.
//!
//! A rasterisation stage takes its outlines from `--layer shape=PATH`. The file is UTF-8 text,
//! one record per line, words separated by spaces or tabs. Empty lines are skipped, and so is a
//! line whose first word starts with `#`. The first record is the name of the form. Every number
//! is a finite decimal (digits, an optional leading `-`, an optional fraction, an optional
//! exponent) of at most [`MAX_COORD`] in absolute value. Coordinates are in pixels of the frame;
//! the parameters `centre` and `y_down` of the stage say where a pixel lies in that plane.
//!
//! `rings-1` holds polygons as rings of points, for `raster-polygon`:
//!
//! ```text
//! rings-1
//! # a square with a square hole
//! polygon
//! ring
//! point 1 1
//! point 7 1
//! point 7 7
//! point 1 7
//! ring
//! point 3 3
//! point 3 5
//! point 5 5
//! point 5 3
//! ```
//!
//! - `polygon` starts a polygon. The rings of one polygon are filled together under the fill
//!   rule, so a hole is a ring inside another. Polygons are combined in file order.
//! - `ring` starts a ring of the last polygon. A ring is closed: its last point joins its first.
//! - `point X Y` adds a point to the last ring.
//! - A polygon holds at least one ring and a ring at least three points. A file without a
//!   polygon is valid and covers nothing.
//!
//! `contours-1` holds polygons as contours of control points, for `raster-contour`:
//!
//! ```text
//! contours-1
//! polygon
//! contour bezier closed
//! point 2 2 0 0 0 0
//! point 6 2 0 0 0 0
//! point 6 6 0 0 0 0
//! contour xspline-basic open
//! point 1 1 0
//! point 2 5 1
//! point 3 1 0
//! ```
//!
//! - `polygon` starts a polygon, as above; its contours are filled together.
//! - `contour KIND CLOSURE` starts a contour of the last polygon. `KIND` is `bezier`,
//!   `xspline-basic` or `xspline-general`; `CLOSURE` is `closed` or `open`. An open contour is
//!   filled as the ring of its polyline, so a straight edge joins its last point to its first.
//! - `point X Y IN_X IN_Y OUT_X OUT_Y` adds a point to a Bezier contour: the anchor, the incoming
//!   tangent handle and the outgoing one. The parameter `tangents` says whether a handle is an
//!   offset from its anchor or a position.
//! - `point X Y S` adds a point to an X-spline contour: the position and the stored weight. The
//!   parameter `weight_map` maps the weight to the shape parameter.
//! - A polygon holds at least one contour and a contour at least two points.
//!
//! A file that cannot be read or does not follow its form is `bad_input`. A file that follows
//! its form and holds a number above [`MAX_COORD`] in absolute value is `unsupported`.

use contour_eval::{Contour, Kind, Point, XVariant};
use coverage_raster::Polygon;

use crate::error::StageError;
use crate::params::parse_float;
use crate::registry::{LayerDef, Plan};

/// The id of the shape layer.
pub const LAYER_ID: &str = "shape";
/// The form word of a file of rings, its first record.
pub const RINGS_FORM: &str = "rings-1";
/// The form word of a file of contours, its first record.
pub const CONTOURS_FORM: &str = "contours-1";
/// The largest absolute value of a number in a shape file.
pub const MAX_COORD: f64 = 1.0e9;

/// The shape layer of `raster-polygon`.
pub const RINGS_LAYER: LayerDef = LayerDef {
    id: LAYER_ID,
    optional: true,
    form: RINGS_FORM,
    about: "The polygons to rasterise: rings of points in pixels of the frame. Without it nothing is covered.",
};

/// The shape layer of `raster-contour`.
pub const CONTOURS_LAYER: LayerDef = LayerDef {
    id: LAYER_ID,
    optional: true,
    form: CONTOURS_FORM,
    about: "The contours to rasterise: cubic Bezier or X-spline control points in pixels of the frame. Without it nothing is covered.",
};

/// Why a shape text is not taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapeError {
    /// The text does not follow its form.
    Malformed(String),
    /// The text follows its form and holds a value the stages do not take.
    Refused(String),
}

fn malformed<T>(line: usize, why: &str) -> Result<T, ShapeError> {
    Err(ShapeError::Malformed(format!("line {line}: {why}")))
}

/// The records of a text: the line number, counted from 1, and the words of every line that is
/// neither empty nor a comment.
fn records(text: &str) -> impl Iterator<Item = (usize, Vec<&str>)> {
    text.lines().enumerate().filter_map(|(index, raw)| {
        let words: Vec<&str> = raw.split_whitespace().collect();
        if words.is_empty() || words[0].starts_with('#') {
            None
        } else {
            Some((index + 1, words))
        }
    })
}

fn number(word: &str, line: usize) -> Result<f64, ShapeError> {
    let Some(v) = parse_float(word) else {
        return malformed(line, &format!("`{word}` is not a number"));
    };
    if v.abs() > MAX_COORD {
        return Err(ShapeError::Refused(format!(
            "line {line}: {v} is outside -{MAX_COORD} to {MAX_COORD}"
        )));
    }
    Ok(v)
}

fn numbers<const N: usize>(words: &[&str], line: usize) -> Result<[f64; N], ShapeError> {
    let mut out = [0.0; N];
    for (slot, word) in out.iter_mut().zip(words) {
        *slot = number(word, line)?;
    }
    Ok(out)
}

/// Parse the text of a `rings-1` file into polygons.
pub fn parse_rings(text: &str) -> Result<Vec<Polygon>, ShapeError> {
    let mut polygons: Vec<Polygon> = Vec::new();
    let mut seen_form = false;
    for (line, words) in records(text) {
        if !seen_form {
            if words != [RINGS_FORM] {
                return malformed(line, &format!("the first line is not `{RINGS_FORM}`"));
            }
            seen_form = true;
            continue;
        }
        match words.as_slice() {
            ["polygon"] => polygons.push(Polygon(Vec::new())),
            ["ring"] => match polygons.last_mut() {
                Some(polygon) => polygon.0.push(Vec::new()),
                None => return malformed(line, "`ring` comes after a `polygon` line"),
            },
            ["point", x, y] => {
                let Some(ring) = polygons.last_mut().and_then(|p| p.0.last_mut()) else {
                    return malformed(line, "`point` comes after a `ring` line");
                };
                let [x, y] = numbers(&[*x, *y], line)?;
                ring.push((x, y));
            }
            _ => return malformed(line, "expected `polygon`, `ring` or `point X Y`"),
        }
    }
    if !seen_form {
        return Err(ShapeError::Malformed(format!(
            "the file has no `{RINGS_FORM}` line"
        )));
    }
    for (p, polygon) in polygons.iter().enumerate() {
        if polygon.0.is_empty() {
            return Err(ShapeError::Malformed(format!("polygon {p} has no ring")));
        }
        for (r, ring) in polygon.0.iter().enumerate() {
            if ring.len() < 3 {
                return Err(ShapeError::Malformed(format!(
                    "ring {r} of polygon {p} has {} points: a ring has at least 3",
                    ring.len()
                )));
            }
        }
    }
    Ok(polygons)
}

fn contour_kind(word: &str) -> Option<Kind> {
    match word {
        "bezier" => Some(Kind::Bezier),
        "xspline-basic" => Some(Kind::XSpline {
            variant: XVariant::Basic,
        }),
        "xspline-general" => Some(Kind::XSpline {
            variant: XVariant::General,
        }),
        _ => None,
    }
}

/// Parse the text of a `contours-1` file into polygons, each a list of contours.
pub fn parse_contours(text: &str) -> Result<Vec<Vec<Contour>>, ShapeError> {
    let mut polygons: Vec<Vec<Contour>> = Vec::new();
    let mut seen_form = false;
    for (line, words) in records(text) {
        if !seen_form {
            if words != [CONTOURS_FORM] {
                return malformed(line, &format!("the first line is not `{CONTOURS_FORM}`"));
            }
            seen_form = true;
            continue;
        }
        match words.as_slice() {
            ["polygon"] => polygons.push(Vec::new()),
            ["contour", kind, closure] => {
                let Some(polygon) = polygons.last_mut() else {
                    return malformed(line, "`contour` comes after a `polygon` line");
                };
                let Some(kind) = contour_kind(kind) else {
                    return malformed(
                        line,
                        &format!(
                            "`{kind}` is not a contour kind (bezier, xspline-basic or xspline-general)"
                        ),
                    );
                };
                let closed = match *closure {
                    "closed" => true,
                    "open" => false,
                    other => {
                        return malformed(line, &format!("`{other}` is not `closed` or `open`"))
                    }
                };
                polygon.push(Contour {
                    kind,
                    closed,
                    points: Vec::new(),
                });
            }
            ["point", fields @ ..] => {
                let Some(contour) = polygons.last_mut().and_then(|p| p.last_mut()) else {
                    return malformed(line, "`point` comes after a `contour` line");
                };
                let point = match (contour.kind, fields.len()) {
                    (Kind::Bezier, 6) => {
                        let [x, y, in_x, in_y, out_x, out_y] = numbers(fields, line)?;
                        Point {
                            x,
                            y,
                            tan_in: (in_x, in_y),
                            tan_out: (out_x, out_y),
                            s: 0.0,
                        }
                    }
                    (Kind::XSpline { .. }, 3) => {
                        let [x, y, s] = numbers(fields, line)?;
                        Point {
                            s,
                            ..Point::new(x, y)
                        }
                    }
                    (Kind::Bezier, _) => {
                        return malformed(
                            line,
                            "a point of a bezier contour is `point X Y IN_X IN_Y OUT_X OUT_Y`",
                        )
                    }
                    (Kind::XSpline { .. }, _) => {
                        return malformed(line, "a point of an xspline contour is `point X Y S`")
                    }
                };
                contour.points.push(point);
            }
            _ => {
                return malformed(
                    line,
                    "expected `polygon`, `contour KIND CLOSURE` or a `point` line",
                )
            }
        }
    }
    if !seen_form {
        return Err(ShapeError::Malformed(format!(
            "the file has no `{CONTOURS_FORM}` line"
        )));
    }
    for (p, polygon) in polygons.iter().enumerate() {
        if polygon.is_empty() {
            return Err(ShapeError::Malformed(format!("polygon {p} has no contour")));
        }
        for (c, contour) in polygon.iter().enumerate() {
            if contour.points.len() < 2 {
                return Err(ShapeError::Malformed(format!(
                    "contour {c} of polygon {p} has {} points: a contour has at least 2",
                    contour.points.len()
                )));
            }
        }
    }
    Ok(polygons)
}

/// Read the shape layer of a request and parse it with `parse`. Without the layer the result is
/// empty: no polygon, so nothing is covered.
fn read<T>(
    plan: &Plan,
    parse: fn(&str) -> Result<Vec<T>, ShapeError>,
) -> Result<Vec<T>, StageError> {
    let Some(path) = plan.layers.path(LAYER_ID) else {
        return Ok(Vec::new());
    };
    let text = std::fs::read_to_string(path)
        .map_err(|e| StageError::bad_input(format!("cannot read the shape file `{path}`: {e}")))?;
    parse(&text).map_err(|e| match e {
        ShapeError::Malformed(why) => StageError::bad_input(format!("shape file `{path}`: {why}")),
        ShapeError::Refused(why) => StageError::unsupported(format!("shape file `{path}`: {why}")),
    })
}

/// The polygons of the `rings-1` shape layer of a request.
pub fn rings(plan: &Plan) -> Result<Vec<Polygon>, StageError> {
    read(plan, parse_rings)
}

/// The polygons of the `contours-1` shape layer of a request, each a list of contours.
pub fn contours(plan: &Plan) -> Result<Vec<Vec<Contour>>, StageError> {
    read(plan, parse_contours)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn malformed_text<T: std::fmt::Debug>(result: Result<T, ShapeError>) -> String {
        match result {
            Err(ShapeError::Malformed(why)) => why,
            other => panic!("expected a malformed text, got {other:?}"),
        }
    }

    #[test]
    fn rings_parse_into_polygons() {
        let text = "\n# made up\nrings-1\npolygon\nring\npoint 1 1\npoint 7 1\n\tpoint 7 7\nring\npoint 3 3\npoint 3 5\npoint 5.5 -2e0\npolygon\nring\npoint 0 0\npoint 1 0\npoint 0 1\n";
        let polygons = parse_rings(text).unwrap();
        assert_eq!(polygons.len(), 2);
        assert_eq!(
            polygons[0],
            Polygon(vec![
                vec![(1.0, 1.0), (7.0, 1.0), (7.0, 7.0)],
                vec![(3.0, 3.0), (3.0, 5.0), (5.5, -2.0)],
            ])
        );
        assert_eq!(
            polygons[1],
            Polygon::from_ring(vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)])
        );
    }

    #[test]
    fn a_file_of_rings_without_a_polygon_is_empty() {
        assert_eq!(parse_rings("rings-1\n").unwrap(), Vec::new());
    }

    #[test]
    fn malformed_rings_name_their_line() {
        let cases = [
            ("", "the file has no `rings-1` line"),
            ("contours-1\n", "line 1: the first line is not `rings-1`"),
            ("rings-1 extra\n", "line 1: the first line is not `rings-1`"),
            (
                "rings-1\nring\n",
                "line 2: `ring` comes after a `polygon` line",
            ),
            (
                "rings-1\npolygon\npoint 1 1\n",
                "line 3: `point` comes after a `ring` line",
            ),
            (
                "rings-1\npolygon\nring\npoint 1\n",
                "line 4: expected `polygon`, `ring` or `point X Y`",
            ),
            (
                "rings-1\npolygon\nring\npoint 1 2 3\n",
                "line 4: expected `polygon`, `ring` or `point X Y`",
            ),
            (
                "rings-1\nshape\n",
                "line 2: expected `polygon`, `ring` or `point X Y`",
            ),
            (
                "rings-1\npolygon\nring\npoint 1 x\n",
                "line 4: `x` is not a number",
            ),
            (
                "rings-1\npolygon\nring\npoint inf 1\n",
                "line 4: `inf` is not a number",
            ),
            ("rings-1\npolygon\n", "polygon 0 has no ring"),
            (
                "rings-1\npolygon\nring\npoint 0 0\npoint 1 1\n",
                "ring 0 of polygon 0 has 2 points: a ring has at least 3",
            ),
        ];
        for (text, want) in cases {
            assert_eq!(malformed_text(parse_rings(text)), want, "{text:?}");
        }
    }

    #[test]
    fn a_number_above_the_bound_is_refused_not_malformed() {
        let text = "rings-1\npolygon\nring\npoint 0 0\npoint 1e9 0\npoint 0 1.5e9\n";
        assert_eq!(
            parse_rings(text),
            Err(ShapeError::Refused(
                "line 6: 1500000000 is outside -1000000000 to 1000000000".to_string()
            ))
        );
    }

    #[test]
    fn contours_parse_into_polygons() {
        let text = "contours-1\npolygon\ncontour bezier closed\npoint 2 2 0 -1 1 0\npoint 6 2 0 0 0 0\ncontour xspline-general open\npoint 1 1 -0.5\npoint 2 5 1\npolygon\ncontour xspline-basic closed\npoint 0 0 0\npoint 4 0 0.25\n";
        let polygons = parse_contours(text).unwrap();
        assert_eq!(polygons.len(), 2);
        assert_eq!(polygons[0].len(), 2);
        assert_eq!(polygons[0][0].kind, Kind::Bezier);
        assert!(polygons[0][0].closed);
        assert_eq!(
            polygons[0][0].points[0],
            Point {
                x: 2.0,
                y: 2.0,
                tan_in: (0.0, -1.0),
                tan_out: (1.0, 0.0),
                s: 0.0,
            }
        );
        assert_eq!(
            polygons[0][1].kind,
            Kind::XSpline {
                variant: XVariant::General
            }
        );
        assert!(!polygons[0][1].closed);
        assert_eq!(polygons[0][1].points[0].s, -0.5);
        assert_eq!(
            polygons[1][0].kind,
            Kind::XSpline {
                variant: XVariant::Basic
            }
        );
        assert_eq!(
            polygons[1][0].points[1],
            Point {
                s: 0.25,
                ..Point::new(4.0, 0.0)
            }
        );
    }

    #[test]
    fn malformed_contours_name_their_line() {
        let cases = [
            ("", "the file has no `contours-1` line"),
            ("rings-1\n", "line 1: the first line is not `contours-1`"),
            (
                "contours-1\ncontour bezier closed\n",
                "line 2: `contour` comes after a `polygon` line",
            ),
            (
                "contours-1\npolygon\ncontour arc closed\n",
                "line 3: `arc` is not a contour kind (bezier, xspline-basic or xspline-general)",
            ),
            (
                "contours-1\npolygon\ncontour bezier shut\n",
                "line 3: `shut` is not `closed` or `open`",
            ),
            (
                "contours-1\npolygon\ncontour bezier\n",
                "line 3: expected `polygon`, `contour KIND CLOSURE` or a `point` line",
            ),
            (
                "contours-1\npolygon\npoint 1 1 0\n",
                "line 3: `point` comes after a `contour` line",
            ),
            (
                "contours-1\npolygon\ncontour bezier closed\npoint 1 1 0\n",
                "line 4: a point of a bezier contour is `point X Y IN_X IN_Y OUT_X OUT_Y`",
            ),
            (
                "contours-1\npolygon\ncontour xspline-basic closed\npoint 1 1\n",
                "line 4: a point of an xspline contour is `point X Y S`",
            ),
            (
                "contours-1\npolygon\ncontour xspline-basic closed\npoint 1 1 nan\n",
                "line 4: `nan` is not a number",
            ),
            ("contours-1\npolygon\n", "polygon 0 has no contour"),
            (
                "contours-1\npolygon\ncontour bezier open\npoint 1 1 0 0 0 0\n",
                "contour 0 of polygon 0 has 1 points: a contour has at least 2",
            ),
        ];
        for (text, want) in cases {
            assert_eq!(malformed_text(parse_contours(text)), want, "{text:?}");
        }
    }

    #[test]
    fn a_contour_number_above_the_bound_is_refused() {
        let text = "contours-1\npolygon\ncontour xspline-basic open\npoint 0 0 0\npoint 0 0 -2e9\n";
        assert_eq!(
            parse_contours(text),
            Err(ShapeError::Refused(
                "line 5: -2000000000 is outside -1000000000 to 1000000000".to_string()
            ))
        );
    }
}
