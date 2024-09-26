use crate::prelude::*;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct JobId {
    uri: Url,
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{uri}", uri = self.uri)
    }
}

impl From<&JobSpec> for JobId {
    fn from(js: &JobSpec) -> JobId {
        JobId {
            uri: js.uri.clone(),
        }
    }
}

enum JobState {
    Running { pid: u32 },
    Done,
}
#[derive(Clone)]
pub struct JobSpec {
    uri: Url,
    version: i32,
    language_id: Option<String>,
    text: String,
}

pub struct Job {
    job_spec: JobSpec,
    job_state: JobState,
}
