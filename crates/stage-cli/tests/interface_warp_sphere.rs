//! Checks `docs/data/stage_warp_sphere_interface.json` against the built `mocha-stage`: each exit
//! code row of the file is reproduced by a child process, and its parameter and layer rows are
//! those of the manifest, so the file cannot drift from the program.

mod stage_warp_common;

use stage_warp_common::interface;

const ROWS: usize = 35;
const STAGE: &str = "warp-sphere";

#[test]
fn exit_code_rows_equal_the_program() {
    interface::rows_equal_the_program(STAGE, ROWS);
}

#[test]
fn rows_cover_the_exit_codes_of_the_table() {
    interface::rows_cover_the_exit_codes(STAGE);
}

#[test]
fn parameters_and_layers_equal_the_manifest() {
    interface::parameters_and_layers_equal_the_manifest(STAGE);
}
