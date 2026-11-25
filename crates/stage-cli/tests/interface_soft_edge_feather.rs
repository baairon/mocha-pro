//! Checks `docs/data/stage_soft_edge_feather_interface.json` against the built `mocha-stage`: each exit code
//! row of the file is reproduced by a child process, and its parameter and layer rows are those
//! of the manifest, so the file cannot drift from the program.

mod image_support;

use image_support::interface;

const STAGE: &str = "soft-edge-feather";

#[test]
fn exit_code_rows_equal_the_program() {
    interface::rows_equal_the_program(STAGE, 27);
}

#[test]
fn rows_cover_the_exit_codes_of_the_table() {
    interface::rows_cover_the_exit_codes(STAGE);
}

#[test]
fn parameters_and_layers_equal_the_manifest() {
    interface::parameters_and_layers_equal_the_manifest(STAGE);
}
