use crate::prelude::*;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct JobId(pub Uri);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobId").field("uri", &self.0).finish()
    }
}

impl From<&JobSpec> for JobId {
    fn from(js: &JobSpec) -> JobId {
        JobId(js.uri.clone())
    }
}

#[derive(Clone, Debug)]
pub struct JobSpec {
    pub uri: Uri,
    pub version: DocumentVersion,
    pub language_id: String,
    pub text: String,
}

pub struct Job {
    pub pid: Pid,
}

impl Job {
    pub fn spawn_kill(self) {
        // NOTE: Because we called process_group on the subprocess, its pid == its pgid.
        log::info!("killing job [pgid={pid}]", pid = self.pid);
        let errno =
            Errno::from(unsafe { nix::libc::killpg(self.pid.as_raw(), nix::libc::SIGKILL) });
        if errno.is_error() {
            log::trace!(
                "failed to kill job [pid={pid}, error={errno}]",
                pid = self.pid
            );
        }
    }
}
