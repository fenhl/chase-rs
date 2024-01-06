use crate::data::*;
use crate::control::*;

use super::thread_namer;

use std::thread::{Builder, JoinHandle};
use futures::{Future, Sink};
use futures::sync::mpsc::*;

use crate::errors::ChaseError;

impl Chaser {
    pub fn run_stream(
        mut self,
    ) -> Result<(Receiver<String>, JoinHandle<Result<(), ChaseError>>), ChaseError> {
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
