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
    pub(crate) async fn files(&self) -> impl Iterator<Item = PathBuf> {
        let futures = self.folders.iter().map(|folder| {
            get_command_output(
                [
                    "git".to_string(),
                    "-C".to_string(),
                    folder.to_str().unwrap().to_string(),
                    "ls-files".to_string(),
                ]
                .into_iter()
                .collect(),
            )
        });
        join_all(futures)
            .await
            .into_iter()
            .inspect(|res| {
                if let Err(e) = res {
                    log::error!("Failed to list files in git repository: {}", e);
                }
            })
            .flatten()
            .map(PathBuf::from)
    }
}
