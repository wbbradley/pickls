use crossbeam_channel::{bounded, Sender};
use std::thread;
use tokio::runtime::{Handle, Runtime};

#[derive(Debug)]
pub enum AsyncJobRequest {
    Prompt(String),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AsyncJobResponse {}

/// Start a work queue that processes async job requests.
pub fn start_work_queue() -> (tokio::runtime::Runtime, Sender<AsyncJobRequest>) {
    // Create a channel for sending async job requests.
    let (tx, rx) = bounded::<AsyncJobRequest>(1);

    // Create a new multi-threaded runtime for tokio to run on background threads.
    let rt = Runtime::new().unwrap();

    // Pass the runtime handle to the control thread.
    let handle: Handle = rt.handle().clone();
    thread::spawn(move || {
        handle.block_on(async {
            while let Ok(job_request) = rx.recv() {
                tokio::spawn(async move {
                    match job_request {
                        AsyncJobRequest::Prompt(prompt) => {
                            log::error!("Prompt: {prompt}");
                        }
                    }
                });
            }
        });
    });
    (rt, tx)
}
