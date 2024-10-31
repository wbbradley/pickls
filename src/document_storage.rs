#[allow(unused)]
use crate::prelude::*;

#[derive(Clone, Debug)]
pub(crate) struct DocumentStorage {
    pub(crate) language_id: String,
    pub(crate) file_contents: Arc<String>,
}
