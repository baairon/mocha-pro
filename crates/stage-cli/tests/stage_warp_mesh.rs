//! `warp-mesh` on the built program: its manifest entry, its arguments, the mesh file, every
//! refusal, and what it writes for small made up frames, against values worked out by hand and
//! against the `mesh-warp` and `resample` crates called directly.

mod stage_warp_common;

use mesh_warp::{BaryForm, Config, Diagonal, GridRes, Outside, TieRule, Vec2, Warp};
use stage_cli::stages::warp::mesh_form;
use stage_warp_common::*;

const STAGE: &str = "warp-mesh";

/// A 4 by 2 frame: the first row holds 1 to 4, the second 5 to 8.
fn small() -> Vec<u8> {
    grey(&[1, 2, 3, 4, 5, 6, 7, 8])
}

const SMALL: (u32, u32, u32) = (4, 2, 8);

/// The whole 4 by 2 frame as two triangles, moved one pixel to the right in the output.
const SHIFT: &str = "mesh-1\nvertex 0 0 1 0\nvertex 4 0 5 0\nvertex 4 2 5 2\nvertex 0 2 1 2\ntriangle 0 1 2\ntriangle 0 2 3\n";
/// The whole frame with its left and right edges exchanged in the output.
const MIRROR: &str = "mesh-1\nvertex 0 0 4 0\nvertex 4 0 0 0\nvertex 4 2 0 2\nvertex 0 2 4 2\ntriangle 0 1 2\ntriangle 0 2 3\n";
/// [`SHIFT`] with the second triangle named the other way round.
const MIXED: &str = "mesh-1\nvertex 0 0 1 0\nvertex 4 0 5 0\nvertex 4 2 5 2\nvertex 0 2 1 2\ntriangle 0 1 2\ntriangle 0 3 2\n";
/// [`SHIFT`] with a third triangle of no area.
const FLAT: &str = "mesh-1\nvertex 0 0 1 0\nvertex 4 0 5 0\nvertex 4 2 5 2\nvertex 0 2 1 2\ntriangle 0 1 2\ntriangle 0 2 3\ntriangle 0 1 1\n";

fn run_mesh(name: &str, mesh: &str, params: &[(&str, &str)]) -> Run {
    let file = Scratch::new(&format!("mesh-{name}"), mesh);
    let layer = file.layer("mesh");
    let all: Vec<(&str, &str)> = [("filter", "nearest")]
        .iter()
        .chain(params.iter())
        .copied()
        .collect();
    apply(STAGE, SMALL, &all, &["--layer", layer.as_str()], &small())
}

fn out_mesh(name: &str, mesh: &str, params: &[(&str, &str)]) -> Vec<u8> {
    let run = run_mesh(name, mesh, params);
    assert_eq!((run.code, run.err.as_str()), (0, ""), "{name} {params:?}");
    run.out
}

#[test]
fn the_manifest_entry_declares_the_stage() {
    check_the_entry_with_layer(
        STAGE,
        "Mesh warp",
        &[
            "mesh_tie",
            "outside",
            "winding",
            "bary",
            "allow_degenerate",
            "grid",
            "grid_cols",
            "grid_rows",
        ],
        "{\"id\":\"mesh\",\"optional\":true,\"form\":\"mesh-1\",",
    );
    let entry = manifest_entry(STAGE);
    assert!(entry.contains("{\"id\":\"mesh_tie\",\"name\":\"Shared edge rule\",\"kind\":\"popup\",\"default\":\"lowest-index\",\"choices\":[\"lowest-index\",\"pineda\"],\"keyed\":false,"));
    assert!(entry.contains("{\"id\":\"allow_degenerate\",\"name\":\"Allow degenerate\",\"kind\":\"checkbox\",\"default\":false,\"keyed\":false,"));
    assert!(entry.contains("{\"id\":\"grid_cols\",\"name\":\"Grid columns\",\"kind\":\"int\",\"default\":1,\"min\":1,\"max\":4096,\"keyed\":false,"));
    assert_eq!(entry.matches("\"keyed\":true").count(), 0);
}

#[test]
fn describe_answers_the_declared_size() {
    check_describe(STAGE);
}

#[test]
fn without_a_mesh_the_input_is_returned() {
    for depth in [8, 16, 32] {
        let size = (7, 5, depth);
        let input = clip(size, 3, 11);
        let run = apply(STAGE, size, &[], &[], &input);
        assert_eq!((run.code, run.err.as_str()), (0, ""));
        assert!(run.out == input, "depth {depth}");
    }
}

#[test]
fn a_mesh_moved_one_pixel_moves_the_frame() {
    // The output positions of the mesh span x from 1 to 5. Output column 0 lies outside it and
    // is zero; column X reads source column X - 1.
    assert_eq!(
        out_mesh("shift", SHIFT, &[]),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
    // the other formula and the other owner of the shared edge give the same pixels here
    assert_eq!(
        out_mesh(
            "shift-cramer",
            SHIFT,
            &[("bary", "cramer"), ("mesh_tie", "pineda")]
        ),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
    // outside the mesh: the nearest point of the mesh, or the map of the nearest triangle
    // carried on; both read at or before the first column, which the clamp border answers
    for outside in ["clamp-nearest", "extend-affine"] {
        assert_eq!(
            out_mesh("shift-outside", SHIFT, &[("outside", outside)]),
            grey(&[1, 1, 2, 3, 5, 5, 6, 7]),
            "{outside}"
        );
    }
    // with a zero border the carried on map reads before the frame
    assert_eq!(
        out_mesh(
            "shift-extend",
            SHIFT,
            &[("outside", "extend-affine"), ("border", "zero")]
        ),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
    // a wider canvas shows the whole moved frame
    let wide = out_mesh("shift-wide", SHIFT, &[("width", "6")]);
    assert_eq!(wide, grey_or_zero(&[0, 1, 2, 3, 4, 0, 0, 5, 6, 7, 8, 0]));
    // a fixed location grid finds the same triangles
    assert_eq!(
        out_mesh(
            "shift-grid",
            SHIFT,
            &[("grid", "fixed"), ("grid_cols", "3"), ("grid_rows", "2")]
        ),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
}

#[test]
fn a_mesh_with_exchanged_edges_mirrors_the_frame() {
    assert_eq!(
        out_mesh("mirror", MIRROR, &[]),
        grey(&[4, 3, 2, 1, 8, 7, 6, 5])
    );
    // the triangles turn the other way in the output than in the source, each space within
    // itself the same way: winding consistent takes it
    assert_eq!(
        out_mesh("mirror-winding", MIRROR, &[("winding", "consistent")]),
        grey(&[4, 3, 2, 1, 8, 7, 6, 5])
    );
}

#[test]
fn the_resampler_centre_rule_does_not_move_the_mesh() {
    assert_eq!(
        out_mesh("shift-corner", SHIFT, &[("center", "corner")]),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
}

/// One case of the comparison with the crates: the parameters, the mesh configuration they
/// name and the resampler configuration they name.
type Case<'a> = (&'a [(&'a str, &'a str)], Config, resample::Config);

#[test]
fn a_displaced_grid_equals_the_crates_called_directly() {
    // a 3 by 2 grid over a 7 by 5 frame with the inner vertices moved in the output
    let mut mesh = mesh_warp::grid_mesh(3, 2, 0.0, 0.0, 7.0, 5.0, Diagonal::Alternating);
    for (index, (dx, dy)) in [(5, (0.6, -0.4)), (6, (-0.5, 0.3)), (1, (0.25, 0.0))] {
        let v = &mut mesh.vertices[index];
        v.dst = Vec2::new(v.dst.x + dx, v.dst.y + dy);
    }
    let file = Scratch::new("mesh-grid", &mesh_form::write(&mesh));
    let layer = file.layer("mesh");
    let cases: [Case<'_>; 3] = [
        (&[], Config::default(), resample::Config::default()),
        (
            &[
                ("mesh_tie", "pineda"),
                ("bary", "cramer"),
                ("outside", "extend-affine"),
                ("grid", "fixed"),
                ("grid_cols", "4"),
                ("grid_rows", "3"),
                ("filter", "keys"),
                ("border", "mirror"),
            ],
            Config {
                tie: TieRule::TopLeft,
                bary: BaryForm::Cramer,
                outside: Outside::ExtendAffine,
                grid: GridRes::Fixed(4, 3),
                ..Config::default()
            },
            resample::Config {
                filter: resample::Filter::Keys { a: -0.5 },
                border: resample::Border::Mirror,
                ..resample::Config::default()
            },
        ),
        (
            &[
                ("outside", "clamp-nearest"),
                ("width", "9"),
                ("height", "6"),
            ],
            Config {
                outside: Outside::ClampNearest,
                ..Config::default()
            },
            resample::Config::default(),
        ),
    ];
    for (n, (params, cfg, sampler)) in cases.iter().enumerate() {
        let warp = Warp::new(mesh.clone(), cfg).unwrap();
        let dst = if n == 2 { (9, 6) } else { (7, 5) };
        for depth in [8, 16, 32] {
            let size = (7, 5, depth);
            let input = clip(size, 2, 8);
            let want = reference(&input, size, dst, sampler, |_, x, y| {
                warp.inverse(Vec2::new(x, y)).map(|q| (q.x, q.y))
            });
            let run = apply(STAGE, size, params, &["--layer", layer.as_str()], &input);
            assert_eq!((run.code, run.err.as_str()), (0, ""), "case {n}");
            assert!(run.out == want, "case {n}, depth {depth}");
            assert!(
                run.out != input,
                "case {n}, depth {depth}: the mesh moves pixels"
            );
        }
    }
}

#[test]
fn a_mesh_the_configuration_refuses_is_unsupported() {
    // a triangle of no area
    let flat = run_mesh("flat", FLAT, &[]);
    assert_eq!(flat.code, 3);
    assert!(
        flat.err.starts_with("error: unsupported: mesh file `"),
        "{}",
        flat.err
    );
    assert!(
        flat.err
            .ends_with("`: triangle 2 has no area and allow_degenerate is off\n"),
        "{}",
        flat.err
    );
    assert!(flat.out.is_empty());
    assert_eq!(
        out_mesh("flat-allowed", FLAT, &[("allow_degenerate", "1")]),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
    // triangles that turn both ways
    assert_eq!(
        out_mesh("mixed", MIXED, &[]),
        grey_or_zero(&[0, 1, 2, 3, 0, 5, 6, 7])
    );
    let mixed = run_mesh("mixed-consistent", MIXED, &[("winding", "consistent")]);
    assert_eq!(mixed.code, 3);
    assert!(
        mixed
            .err
            .ends_with("`: triangle 1 turns the other way and winding is consistent\n"),
        "{}",
        mixed.err
    );
    assert!(mixed.out.is_empty());
    // the refusal comes before any frame is read: an empty input is refused as well
    let file = Scratch::new("mesh-flat-empty", FLAT);
    let layer = file.layer("mesh");
    let empty = apply(STAGE, SMALL, &[], &["--layer", layer.as_str()], b"");
    assert_eq!(empty.code, 3);
    // --describe opens no layer
    let described = apply(
        STAGE,
        SMALL,
        &[],
        &["--describe", "--layer", layer.as_str()],
        b"",
    );
    assert_eq!(described.code, 0);
}

#[test]
fn a_mesh_file_that_cannot_be_read_or_parsed_is_bad_input() {
    let frame = small();
    let missing = apply(
        STAGE,
        SMALL,
        &[],
        &["--layer", "mesh=no/such/mesh/file"],
        &frame,
    );
    assert_eq!(missing.code, 4);
    assert!(
        missing
            .err
            .starts_with("error: bad_input: cannot read the mesh file `no/such/mesh/file`: "),
        "{}",
        missing.err
    );
    assert_eq!(missing.err.matches('\n').count(), 1);
    assert!(missing.out.is_empty());
    for (name, content, needle) in [
        ("empty", "", "the file is empty: the first line is `mesh-1`"),
        ("form", "mesh-2\n", "line 1: the first line is not `mesh-1`"),
        (
            "keys",
            "keys-1\nkey width 0 1 hold\n",
            "line 1: the first line is not `mesh-1`",
        ),
        (
            "word",
            "mesh-1\nquad 0 1 2 3\n",
            "line 2: `quad` is not `vertex` or `triangle`",
        ),
        (
            "count",
            "mesh-1\nvertex 0 0 1\n",
            "line 2: `vertex` takes 4 values, found 3",
        ),
        (
            "number",
            "mesh-1\nvertex 0 0 1 nan\n",
            "line 2: cannot parse `nan` as a number",
        ),
        (
            "index",
            "mesh-1\nvertex 0 0 0 0\ntriangle 0 0 x\n",
            "line 3: `x` is not a vertex number",
        ),
        (
            "short",
            "mesh-1\ntriangle 0 1\n",
            "line 2: `triangle` takes 3 vertex numbers, found 2",
        ),
        (
            "none",
            "mesh-1\nvertex 0 0 0 0\n",
            "the file has no triangle",
        ),
        (
            "range",
            "mesh-1\nvertex 0 0 0 0\ntriangle 0 0 1\n",
            "line 3: vertex 1 is not in the file, which has 1 vertices",
        ),
    ] {
        for input in [&frame[..], &b""[..]] {
            let file = Scratch::new(&format!("mesh-bad-{name}"), content);
            let layer = file.layer("mesh");
            let run = apply(STAGE, SMALL, &[], &["--layer", layer.as_str()], input);
            assert_eq!(run.code, 4, "{name}: {}", run.err);
            assert!(
                run.err.starts_with("error: bad_input: mesh file `"),
                "{name}: {}",
                run.err
            );
            assert!(
                run.err.ends_with(&format!("`: {needle}\n")),
                "{name}: {}",
                run.err
            );
            assert_eq!(run.err.matches('\n').count(), 1, "{name}");
            assert!(run.out.is_empty(), "{name}");
        }
    }
    let twice = apply(
        STAGE,
        SMALL,
        &[],
        &["--layer", "mesh=a", "--layer", "mesh=b"],
        &frame,
    );
    assert_eq!(twice.code, 2, "{}", twice.err);
    // the stage has no key file layer
    let keys = apply(STAGE, SMALL, &[], &["--layer", "keys=x"], &frame);
    assert_eq!(keys.code, 3, "{}", keys.err);
}

#[test]
fn every_parameter_refuses_a_bad_value() {
    check_refusals(
        STAGE,
        &[
            (
                &[("mesh_tie", "first")],
                3,
                "`first` is not one of lowest-index, pineda",
            ),
            (
                &[("outside", "zero")],
                3,
                "`zero` is not one of none, clamp-nearest, extend-affine",
            ),
            (
                &[("winding", "clockwise")],
                3,
                "`clockwise` is not one of any, consistent",
            ),
            (
                &[("bary", "areas")],
                3,
                "`areas` is not one of edge-functions, cramer",
            ),
            (
                &[("allow_degenerate", "yes")],
                3,
                "bad value for `allow_degenerate`: `yes` is not 1 or 0",
            ),
            (
                &[("grid", "auto")],
                3,
                "`auto` is not one of per-triangle-sqrt, fixed",
            ),
            (
                &[("grid_cols", "0")],
                3,
                "bad value for `grid_cols`: 0 is outside 1 to 4096",
            ),
            (&[("grid_rows", "4097")], 3, "bad value for `grid_rows`"),
            (
                &[("grid_rows", "1.5")],
                3,
                "cannot parse `1.5` as a whole number",
            ),
            (
                &[("bary", "cramer"), ("bary", "cramer")],
                2,
                "parameter `bary` given more than once",
            ),
            (&[("m00", "1")], 3, "warp-mesh has no parameter `m00`"),
            (&[("grid_cols", "1@0;2@5")], 3, "--layer keys=PATH"),
        ],
    );
}

#[test]
fn the_shared_refusals_hold() {
    check_the_stream_refusals(STAGE);
}

#[test]
fn frames_are_written_as_they_are_read() {
    // a mesh over the 64 by 48 frame of the check, with one corner pulled inward
    let mesh = "mesh-1\nvertex 0 0 6 4\nvertex 64 0 64 0\nvertex 64 48 64 48\nvertex 0 48 0 48\ntriangle 0 1 2\ntriangle 0 2 3\n";
    let file = Scratch::new("mesh-streams", mesh);
    let layer = file.layer("mesh");
    check_it_streams_with(
        STAGE,
        &[("outside", "clamp-nearest")],
        &["--layer", layer.as_str()],
        (64, 48),
    );
    check_it_streams(STAGE, &[], (64, 48));
}
