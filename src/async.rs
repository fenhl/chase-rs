use {
    std::{
        path::PathBuf,
        thread::{
            Builder,
            JoinHandle,
        },
    },
    futures::{
        Future,
        Sink,
        sync::mpsc::*,
    },
    crate::{
        data::*,
        control::*,
    },
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
        let (mut tx, rx) = channel(0);

        let join_handle = Builder::new()
            .name(thread_namer(&self.path))
            .spawn(move || {
                self.run(|line| {
                    let next_tx = tx.clone().send(line.to_string()).wait()?;
                    tx = next_tx;
                    Ok(Control::Continue)
                })?;
                Ok(())
            })?;
        Ok((rx, join_handle))
    }
}
