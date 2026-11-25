//! The sink of a data stage: a text file beside the frames, written whole or not at all.
//!
//! A data stage passes its frames through and writes its result to the file named by a text
//! parameter. The lines go to a temporary file in the folder of the target, named
//! `.NAME.PID.part`. [`Sink::commit`] renames it to the target once the input has ended. A sink
//! that is dropped before that removes the temporary file and leaves the target path as it was,
//! so a reader never sees half a file. The rename stays inside one folder, so it never crosses a
//! file system.
//!
//! Every failure of a sink is `stage_failed` (exit 1): the result could not be moved into place.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::{Code, StageError};

/// A text file that appears at its target path only when [`Sink::commit`] succeeds.
pub struct Sink {
    target: PathBuf,
    temp: PathBuf,
    file: Option<BufWriter<File>>,
}

fn failed(what: &str, path: &Path, why: &dyn std::fmt::Display) -> StageError {
    StageError::new(
        Code::StageFailed,
        format!("cannot {what} `{}`: {why}", path.display()),
    )
}

/// The temporary path of a target: `.NAME.PID.part` in the folder of the target, or `None` when
/// the target has no file name.
pub fn temp_path(target: &Path) -> Option<PathBuf> {
    let name = target.file_name()?.to_str()?;
    Some(target.with_file_name(format!(".{name}.{}.part", std::process::id())))
}

impl Sink {
    /// Open the temporary file of `target`. Nothing exists at the target path until
    /// [`Sink::commit`]. A target without a file name, a target that is a folder and a folder
    /// that cannot be written are `stage_failed`.
    pub fn create(target: &str) -> Result<Sink, StageError> {
        let target = PathBuf::from(target);
        let temp = temp_path(&target)
            .ok_or_else(|| failed("write the data file", &target, &"the path names no file"))?;
        if target.is_dir() {
            return Err(failed(
                "write the data file",
                &target,
                &"the path is a folder",
            ));
        }
        let file = File::create(&temp).map_err(|e| failed("create", &temp, &e))?;
        Ok(Sink {
            target,
            temp,
            file: Some(BufWriter::new(file)),
        })
    }

    /// Write one line; the line break is added here.
    pub fn line(&mut self, text: &str) -> Result<(), StageError> {
        let Some(file) = self.file.as_mut() else {
            return Err(failed("write", &self.temp, &"the file is closed"));
        };
        file.write_all(text.as_bytes())
            .and_then(|()| file.write_all(b"\n"))
            .map_err(|e| failed("write", &self.temp, &e))
    }

    /// Close the temporary file and rename it to the target. On a failure the temporary file is
    /// removed and the target path is left as it was.
    pub fn commit(mut self) -> Result<(), StageError> {
        let Some(mut file) = self.file.take() else {
            return Err(failed("write", &self.temp, &"the file is closed"));
        };
        let closed = file.flush().and_then(|()| file.get_ref().sync_all());
        drop(file);
        let moved = closed
            .map_err(|e| failed("write", &self.temp, &e))
            .and_then(|()| {
                std::fs::rename(&self.temp, &self.target)
                    .map_err(|e| failed("move the data file to", &self.target, &e))
            });
        if moved.is_err() {
            let _ = std::fs::remove_file(&self.temp);
        }
        moved
    }
}

impl Drop for Sink {
    /// A sink that was not committed removes its temporary file.
    fn drop(&mut self) {
        if self.file.take().is_some() {
            let _ = std::fs::remove_file(&self.temp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own in the temporary folder, removed when dropped.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Folder {
            let path =
                std::env::temp_dir().join(format!("stage-cli-sink-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create the scratch folder");
            Folder(path)
        }

        fn path(&self, name: &str) -> String {
            self.0.join(name).to_str().expect("a text path").to_string()
        }

        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .expect("list the scratch folder")
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_committed_sink_holds_its_lines_and_leaves_no_temporary_file() {
        let folder = Folder::new("commit");
        let target = folder.path("poses.keys");
        let mut sink = Sink::create(&target).unwrap();
        sink.line("keys-1").unwrap();
        sink.line("key m00 0 1 linear").unwrap();
        assert_eq!(folder.names().len(), 1, "only the temporary file exists");
        assert!(!Path::new(&target).exists());
        sink.commit().unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "keys-1\nkey m00 0 1 linear\n"
        );
        assert_eq!(folder.names(), ["poses.keys"]);
    }

    #[test]
    fn a_dropped_sink_leaves_nothing() {
        let folder = Folder::new("drop");
        let mut sink = Sink::create(&folder.path("poses.keys")).unwrap();
        sink.line("keys-1").unwrap();
        drop(sink);
        assert!(folder.names().is_empty());
    }

    #[test]
    fn a_dropped_sink_keeps_an_earlier_target() {
        let folder = Folder::new("keep");
        let target = folder.path("poses.keys");
        std::fs::write(&target, "earlier\n").unwrap();
        let mut sink = Sink::create(&target).unwrap();
        sink.line("later").unwrap();
        drop(sink);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "earlier\n");
        assert_eq!(folder.names(), ["poses.keys"]);
    }

    #[test]
    fn a_commit_replaces_an_earlier_target() {
        let folder = Folder::new("replace");
        let target = folder.path("poses.keys");
        std::fs::write(&target, "earlier\n").unwrap();
        let mut sink = Sink::create(&target).unwrap();
        sink.line("later").unwrap();
        sink.commit().unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "later\n");
    }

    #[test]
    fn the_temporary_file_is_in_the_folder_of_the_target() {
        let temp = temp_path(Path::new("some/folder/poses.keys")).unwrap();
        assert_eq!(temp.parent(), Some(Path::new("some/folder")));
        let name = temp.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".poses.keys.") && name.ends_with(".part"));
        assert_eq!(temp_path(Path::new("..")), None);
        assert_eq!(temp_path(Path::new("/")), None);
    }

    #[test]
    fn a_folder_that_does_not_exist_is_stage_failed() {
        let folder = Folder::new("missing");
        let e = Sink::create(&folder.path("no-such-folder/poses.keys"))
            .err()
            .unwrap();
        assert_eq!((e.code, e.exit()), (Code::StageFailed, 1));
        assert!(e.message.starts_with("cannot create `"), "{}", e.message);
    }

    #[test]
    fn a_target_that_is_a_folder_or_names_no_file_is_stage_failed() {
        let folder = Folder::new("folder");
        let e = Sink::create(folder.0.to_str().unwrap()).err().unwrap();
        assert_eq!(e.code, Code::StageFailed);
        assert!(e.message.ends_with("the path is a folder"), "{}", e.message);
        assert!(folder.names().is_empty());
        let e = Sink::create("..").err().unwrap();
        assert_eq!(e.code, Code::StageFailed);
        assert!(
            e.message.ends_with("the path names no file"),
            "{}",
            e.message
        );
    }
}
