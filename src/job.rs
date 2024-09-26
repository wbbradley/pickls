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

pub enum JobState {
    Running(JobToolPid),
    Done,
}

#[derive(Clone)]
pub struct JobSpec {
    pub uri: Url,
    pub version: i32,
    pub language_id: Option<String>,
    pub text: String,
}

pub struct Job {
    pub job_spec: JobSpec,
    pub job_state: JobState,
}
