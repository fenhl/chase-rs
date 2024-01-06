use std::io::BufReader;
use std::fs::File;
use std::time::Duration;

use std::path::PathBuf;

#[cfg(windows)] use windows::Win32::Storage::FileSystem::FILE_ID_INFO;

pub(crate) const DEFAULT_ROTATION_CHECK_WAIT: Duration = Duration::from_millis(100);
pub(crate) const DEFAULT_NOT_ROTATED_WAIT: Duration = Duration::from_millis(50);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct Line(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct Pos(pub(crate) u64);

#[cfg(unix)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct FileId(pub(crate) u64);

#[cfg(windows)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct FileId(pub(crate) FILE_ID_INFO);

#[derive(Debug, Clone)]
pub struct Chaser {
    pub line: Line,
    pub(crate) path: PathBuf,
    pub(crate) initial_no_file_wait: Duration,
    pub(crate) initial_no_file_attempts: Option<usize>,
    pub(crate) rotation_check_wait: Duration,
    pub(crate) rotation_check_attempts: Option<usize>,
    pub(crate) not_rotated_wait: Duration,
}

#[derive(Debug)]
pub(crate) struct Chasing<'a> {
    pub(crate) chaser: &'a mut Chaser,
    pub(crate) file_id: FileId,
    pub(crate) reader: BufReader<File>,
    pub(crate) buffer: String,
    pub(crate) line: Line,
    pub(crate) pos: Pos,
}

impl Chaser {
    pub fn new(path: impl Into<PathBuf>) -> Chaser {
        Chaser {
            line: Line(0),
            path: path.into(),
            initial_no_file_attempts: None,
            initial_no_file_wait: DEFAULT_ROTATION_CHECK_WAIT,
            rotation_check_attempts: None,
            rotation_check_wait: DEFAULT_ROTATION_CHECK_WAIT,
            not_rotated_wait: DEFAULT_NOT_ROTATED_WAIT,
        }
    }
}
