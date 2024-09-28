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

pub struct JobToolPid {
    /// A Process ID for the job.
    pid: u32,
    join_handle: JoinHandle<()>,
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
    pub pid: Pid,
}

impl Job {
    pub fn spawn_kill(self) {
        tokio::spawn(async move {
            // NB: Because we called process_group on the subprocess, its pid == its pgid.
            log::warn!("killing job [pgid={pid}]", pid = self.pid);
            unsafe {
                let errno = Errno::from(nix::libc::killpg(
                    self.pid.as_raw() as i32,
                    nix::libc::SIGKILL,
                ));
                if errno.is_error() {
                    log::error!(
                        "failed to kill job [pid={pid}, error={errno}]",
                        pid = self.pid
                    );
                }
            }
        });
    }
}
