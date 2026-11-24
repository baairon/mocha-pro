//! The error rows of the handoff convention that a stage process can meet, with their exits.

use std::fmt;

/// A row of the convention's error table, restricted to the ones a stage reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    /// Any other failure (exit 1).
    StageFailed,
    /// The command line was rejected (exit 2).
    Usage,
    /// A valid request that is refused (exit 3).
    Unsupported,
    /// The input could not be read or is malformed (exit 4).
    BadInput,
    /// The reader of the output went away (exit 4).
    BrokenPipe,
    /// The input ended inside a frame (exit 4).
    PartialFrame,
}

impl Code {
    /// The code word printed after `error: `.
    pub fn name(self) -> &'static str {
        match self {
            Code::StageFailed => "stage_failed",
            Code::Usage => "usage",
            Code::Unsupported => "unsupported",
            Code::BadInput => "bad_input",
            Code::BrokenPipe => "broken_pipe",
            Code::PartialFrame => "partial_frame",
        }
    }

    /// The process exit status of a stage that fails with this code.
    pub fn exit(self) -> i32 {
        match self {
            Code::StageFailed => 1,
            Code::Usage => 2,
            Code::Unsupported => 3,
            Code::BadInput | Code::BrokenPipe | Code::PartialFrame => 4,
        }
    }
}

/// A failure of the stage: a code and one line of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageError {
    /// The error row.
    pub code: Code,
    /// One line of text without a line break.
    pub message: String,
}

impl StageError {
    /// A new error; line breaks and other control characters in `message` become spaces so the
    /// text form stays one line even when it quotes user input.
    pub fn new(code: Code, message: impl Into<String>) -> StageError {
        let message: String = message
            .into()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        StageError { code, message }
    }

    /// A `usage` error.
    pub fn usage(message: impl Into<String>) -> StageError {
        StageError::new(Code::Usage, message)
    }

    /// An `unsupported` error.
    pub fn unsupported(message: impl Into<String>) -> StageError {
        StageError::new(Code::Unsupported, message)
    }

    /// A `bad_input` error.
    pub fn bad_input(message: impl Into<String>) -> StageError {
        StageError::new(Code::BadInput, message)
    }

    /// The process exit status.
    pub fn exit(&self) -> i32 {
        self.code.exit()
    }
}

impl fmt::Display for StageError {
    /// The text form of the convention: `error: <code>: <message>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}: {}", self.code.name(), self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exits_follow_the_stage_table() {
        assert_eq!(Code::StageFailed.exit(), 1);
        assert_eq!(Code::Usage.exit(), 2);
        assert_eq!(Code::Unsupported.exit(), 3);
        assert_eq!(Code::BadInput.exit(), 4);
        assert_eq!(Code::BrokenPipe.exit(), 4);
        assert_eq!(Code::PartialFrame.exit(), 4);
    }

    #[test]
    fn names_are_the_table_words() {
        let names: Vec<&str> = [
            Code::StageFailed,
            Code::Usage,
            Code::Unsupported,
            Code::BadInput,
            Code::BrokenPipe,
            Code::PartialFrame,
        ]
        .iter()
        .map(|c| c.name())
        .collect();
        assert_eq!(
            names,
            [
                "stage_failed",
                "usage",
                "unsupported",
                "bad_input",
                "broken_pipe",
                "partial_frame"
            ]
        );
    }

    #[test]
    fn text_form_is_one_line() {
        let e = StageError::usage("bad\nthing\r\there");
        assert_eq!(e.to_string(), "error: usage: bad thing  here");
        assert!(!e.to_string().contains('\n'));
    }

    #[test]
    fn constructors_set_the_code() {
        assert_eq!(StageError::usage("x").code, Code::Usage);
        assert_eq!(StageError::unsupported("x").code, Code::Unsupported);
        assert_eq!(StageError::bad_input("x").code, Code::BadInput);
        assert_eq!(StageError::unsupported("x").exit(), 3);
    }
}
