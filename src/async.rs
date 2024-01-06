use {
    std::{
        path::PathBuf,
        thread::{
            Builder,
            JoinHandle,
        },
    },
    tokio::sync::mpsc::*,
    crate::data::*,
};

fn thread_namer(path: &PathBuf) -> String {
    format!(
        "chase-thread-{}",
        path.to_str().unwrap_or("undisplayable-path")
    )
}

impl Chaser {
    pub fn run_stream(
        mut self,
    ) -> Result<(Receiver<String>, JoinHandle<Result<(), crate::Error>>), crate::Error> {
        let (tx, rx) = channel(1);

        let join_handle = Builder::new()
            .name(thread_namer(&self.path))
            .spawn(move || {
                self.run(|line| {
                    tx.blocking_send(line.to_string())?;
                    Ok(())
                })?;
                Ok(())
            })?;
        Ok((rx, join_handle))
    }
}
