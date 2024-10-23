use crate::prelude::*;

pub(crate) struct DocumentDiagnostics {
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
        let max_version = self.versions.last().map(|x| *x).unwrap_or(version);
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
        let linter_diagnostics = self
            .linter_diagnostics
            .get(&uri)
            .expect("uri should have diagnostics");
        (
            uri,
            self.max_version,
            linter_diagnostics
                .values()
                .filter_map(|(version, diagnostics)| {
                    if *version == self.max_version {
                        Some(diagnostics.clone())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<_>>(),
        )
    }
}
