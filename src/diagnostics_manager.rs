use crate::prelude::*;

pub(crate) type LinterName = String;
pub(crate) type DiagnosticsStorage = HashMap<Uri, DocumentDiagnostics>;

pub(crate) struct DiagnosticsManager<'a> {
    client: &'a Client,
    diagnostics_storage: DiagnosticsStorage,
}

impl<'a> DiagnosticsManager<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            diagnostics_storage: Default::default(),
        }
    }

    /// Push new diagnostics for a particular uri, linter, and version. This is called
    /// after a linter has finished running.
    pub(crate) fn update_diagnostics(
        &mut self,
        uri: Uri,
        linter_name: String,
        max_linter_count: usize,
        version: DocumentVersion,
        new_diagnostics: Vec<Diagnostic>,
    ) {
        let mut storage = self.diagnostics_storage;
        if !storage.contains_key(&uri) {
            storage.insert(
                uri.clone(),
                DocumentDiagnostics::new(uri.clone(), max_linter_count, version),
            );
        }

        let document_diagnostics: &mut DocumentDiagnostics = storage.get_mut(&uri).unwrap();
        if document_diagnostics.update_diagnostics_storage(
            &uri,
            &linter_name,
            version,
            new_diagnostics,
        ) {
            // The diagnostics for this (uri, linter program) pair have been
            // updated, publish them along with the appropriate versions of the
            // other linters.
            let (uri, version, diagnostics, progress_messages) =
                document_diagnostics.aggregate_most_recent_diagnostics(uri);
            log::info!(
                "publishing diagnostics [linter={linter_name}, uri={uri}, version={version}, count={count}]",
                count = diagnostics.len()
            );
            self.client
                .publish_diagnostics(uri.clone(), diagnostics, Some(version.0));

            for progress_message in progress_messages.into_iter() {
                self.client.send_notification::<Progress>(progress_message)
            }
        }
    }
}
