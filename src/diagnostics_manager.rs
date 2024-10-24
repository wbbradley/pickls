use crate::prelude::*;
use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::{Diagnostic, Url};

pub(crate) type LinterName = String;
pub(crate) type DiagnosticsStorage = HashMap<Url, DocumentDiagnostics>;

#[derive(Clone)]
pub(crate) struct DiagnosticsManager {
    client: Client,
    diagnostics_storage: Arc<Mutex<DiagnosticsStorage>>,
}

impl DiagnosticsManager {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            diagnostics_storage: Arc::new(Mutex::new(Default::default())),
        }
    }

    /// Push new diagnostics for a particular uri, linter, and version. This is called
    /// after a linter has finished running.
    pub(crate) async fn update_diagnostics(
        &mut self,
        uri: Url,
        linter_name: String,
        max_linter_count: usize,
        version: DocumentVersion,
        new_diagnostics: Vec<Diagnostic>,
    ) {
        let mut storage = self.diagnostics_storage.lock().await;
        if !storage.contains_key(&uri) {
            storage.insert(
                uri.clone(),
                DocumentDiagnostics::new(uri.clone(), max_linter_count, version),
            );
        }

        let document_diagnostics: &mut DocumentDiagnostics = storage.get_mut(&uri).unwrap();
        if document_diagnostics
            .update_diagnostics_storage(&uri, &linter_name, version, new_diagnostics)
            .await
        {
            // The diagnostics for this (uri, linter program) pair have been
            // updated, publish them along with the appropriate versions of the
            // other linters.
            let (uri, version, diagnostics, progress_messages) = document_diagnostics
                .aggregate_most_recent_diagnostics(uri)
                .await;
            log::info!(
                "publishing diagnostics [linter={linter_name}, uri={uri}, version={version}, count={count}]",
                count = diagnostics.len()
            );
            self.client
                .publish_diagnostics(uri.clone(), diagnostics, Some(version.0))
                .await;

            let futures = progress_messages.into_iter().map(|progress_message| {
                self.client.send_notification::<Progress>(progress_message)
            });

            join_all(futures).await;
        }
    }
}
