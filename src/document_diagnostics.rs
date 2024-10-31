use crate::prelude::*;

pub(crate) struct DocumentDiagnostics {
    #[allow(dead_code)]
    pub(crate) uri: Url,
    pub(crate) max_linter_count: usize,
    pub(crate) linter_diagnostics: HashMap<LinterName, Vec<Diagnostic>>,
    pub(crate) versions: BTreeSet<DocumentVersion>,
}

impl DocumentDiagnostics {
    pub(crate) fn new(uri: Url, max_linter_count: usize, version: DocumentVersion) -> Self {
        Self {
            uri,
            max_linter_count,
            linter_diagnostics: Default::default(),
            versions: BTreeSet::from([version]),
        }
    }
}

impl DocumentDiagnostics {
    /// Push new diagnostics for a particular uri, linter, and version. This is called
    /// after a linter has finished running.
    pub(crate) async fn update_diagnostics_storage(
        &mut self,
        uri: &Url,
        linter_name: &str,
        version: DocumentVersion,
        mut new_diagnostics: Vec<Diagnostic>,
    ) -> bool {
        let max_version = self.versions.last().cloned().unwrap_or(version);
        if max_version > version {
            log::info!("ignoring diagnostics for version {version} of {uri} from linter {linter_name} because it is older than the most recent version {max_version}");
            return false;
        }
        if version > max_version {
            // We've got a new version of the file. Forget all prior diagnostics.
            self.linter_diagnostics.clear();
        }

        self.linter_diagnostics
            .entry(linter_name.to_string())
            .and_modify(|e| std::mem::swap(e, &mut new_diagnostics))
            .or_insert_with(|| new_diagnostics);

        // Remember all versions that have started progress messages.
        self.versions.insert(version);
        true
    }

    pub(crate) async fn aggregate_most_recent_diagnostics(
        &mut self,
        uri: Url,
    ) -> (Url, DocumentVersion, Vec<Diagnostic>, Vec<ProgressParams>) {
        let max_version = *self.versions.last().unwrap();
        let available = self.linter_diagnostics.len();
        let mut progress_messages = vec![
            // Always create at least one progress message to denote the current update.
            make_progress_params(uri.clone(), max_version, available, self.max_linter_count),
        ];
        for &version in self.versions.iter().rev().skip(1) {
            progress_messages.push(ProgressParams {
                token: progress_token(&uri, version),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: None,
                })),
            });
        }
        self.versions.retain(|x| *x == max_version);

        (
            uri,
            *self.versions.last().unwrap(),
            self.linter_diagnostics
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<_>>(),
            progress_messages,
        )
    }
}
fn make_progress_params(
    uri: Url,
    version: DocumentVersion,
    available: usize,
    expected: usize,
) -> ProgressParams {
    let percentage = if expected == 0 {
        None
    } else {
        Some((available as f64 / expected as f64 * 100.0) as u32)
    };
    log::info!("publishing progress [uri={uri}, version={version}, available={available}, expected={expected}, percentage={percentage:?}]");

    ProgressParams {
        token: progress_token(&uri, version),
        value: ProgressParamsValue::WorkDone(if expected == available && expected != 0 {
            WorkDoneProgress::End(WorkDoneProgressEnd { message: None })
        } else {
            //
            WorkDoneProgress::Report(WorkDoneProgressReport {
                cancellable: Some(false),
                message: Some("job finished".into()),
                percentage,
            })
        }),
    }
}

fn progress_token(uri: &Url, version: DocumentVersion) -> ProgressToken {
    ProgressToken::String(format!("{}:{}", uri, version))
}
