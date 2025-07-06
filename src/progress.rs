use std::sync::atomic::{AtomicUsize, Ordering};

use crate::prelude::*;

pub struct ProgressNotifier {
    pub counter: AtomicUsize,
    pub uri: Uri,
    pub version: DocumentVersion,
    pub total: usize,
    pub client: Client,
}

impl ProgressNotifier {
    pub fn new(client: Client, uri: Uri, version: DocumentVersion, total: usize) -> Self {
        Self {
            counter: AtomicUsize::new(0),
            uri,
            version,
            total,
            client,
        }
    }
    pub fn notify(&self) {
        let counter = self.counter.fetch_add(1, Ordering::SeqCst);
        let progress = make_progress_params(
            "running inline-assist",
            self.uri.clone(),
            self.version,
            counter + 1,
            // The extra 1 is for the final completion message.
            self.total,
        );
        let r = self.client.send_notification::<Progress, _>(progress);
        if let Err(e) = r {
            log::error!("error sending progress notification: {e:?}");
        }
    }
}
