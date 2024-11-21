use crate::prelude::*;

pub(crate) struct Workspace {
    folders: BTreeSet<PathBuf>,
    unused_folders: BTreeSet<Uri>,
}

impl Workspace {
    pub(crate) fn new() -> Self {
        Self {
            folders: Default::default(),
            unused_folders: Default::default(),
        }
    }
    pub(crate) fn add_folder(&mut self, folder: Uri) {
        if folder.scheme().map_or(false, |x| x.as_str() == "file") {
            let file_path = PathBuf::from(folder.path().as_str());
            self.folders.insert(file_path);
            return;
        }
        self.unused_folders.insert(folder);
    }
    pub(crate) fn folders(&self) -> impl Iterator<Item = &PathBuf> {
        self.folders.iter()
    }
}
