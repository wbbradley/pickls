#[derive(Clone, Copy, Default, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct DocumentVersion(pub(crate) i32);

impl From<i32> for DocumentVersion {
    fn from(version: i32) -> Self {
        Self(version)
    }
}

impl std::fmt::Display for DocumentVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
