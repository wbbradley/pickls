use crate::document_version::DocumentVersion;

#[derive(Clone, Debug)]
pub(crate) struct DocumentStorage {
    pub(crate) language_id: String,
    pub(crate) file_contents: String,
    pub(crate) version: DocumentVersion,
}
