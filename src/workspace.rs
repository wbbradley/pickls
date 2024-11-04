use crate::prelude::*;
use tower_lsp::lsp_types::Url;

pub(crate) struct Workspace {
    folders: BTreeSet<PathBuf>,
    unused_folders: BTreeSet<Url>,
}

impl Workspace {
    pub(crate) fn new() -> Self {
        Self {
            folders: Default::default(),
            unused_folders: Default::default(),
        }
    }
    pub(crate) fn add_folder(&mut self, folder: Url) {
        if folder.scheme() == "file" {
            if let Ok(file_path) = folder.to_file_path() {
                self.folders.insert(file_path);
                return;
            }
        }
        self.unused_folders.insert(folder);
    }
    pub(crate) fn folders(&self) -> impl Iterator<Item = &PathBuf> {
        self.folders.iter()
    }
}