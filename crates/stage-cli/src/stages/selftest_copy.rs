//! `selftest-copy`: a self test. It copies frames unchanged and reimplements nothing: it does not
//! compute, imitate or claim any behaviour of any plug-in.

use std::io::{Read, Write};

use crate::error::StageError;
use crate::frame::DEPTHS;
use crate::registry::{Geometry, Kind, Label, Plan, Stage, StageDef};
use crate::run::copy_frames;
use crate::stage::{DESCRIPTION, STAGE_ID};

/// The self test stage.
pub struct SelfTestCopy;

/// What the self test stage declares.
pub static DEF: StageDef = StageDef {
    id: STAGE_ID,
    name: "Self test copy",
    about: DESCRIPTION,
    label: Label::SelfTest,
    kind: Kind::Frame,
    spool: false,
    geometry: Geometry::Preserve,
    depths: &DEPTHS,
    params: &[],
    layers: &[],
};

impl Stage for SelfTestCopy {
    fn def(&self) -> &'static StageDef {
        &DEF
    }

    fn run(
        &self,
        plan: &Plan,
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<u64, StageError> {
        copy_frames(input, output, plan.input.buffer_len()?, plan.frames)
    }
}
