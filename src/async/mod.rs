mod stream;

use std::path::PathBuf;

pub(crate) fn thread_namer(path: &PathBuf) -> String {
    format!(
        "chase-thread-{}",
        path.to_str().unwrap_or("undisplayable-path")
    )
}
