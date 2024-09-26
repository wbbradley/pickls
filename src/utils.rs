use crate::prelude::*;
use std::process;
use sysinfo::{Pid, System};

pub fn get_extension_from_url(url: &Url) -> Option<String> {
    if let Some(path) = url.path_segments() {
        if let Some(filename) = path.last() {
            return filename.rsplit('.').next().map(String::from);
        }
    }
    None
}

pub async fn fetch_parent_process_info() -> Option<String> {
    let mut system = System::new_all();
    system.refresh_all();

    let current_pid = process::id();
    if let Some(current_process) = system.process(Pid::from(current_pid as usize)) {
        if let Some(parent_pid) = current_process.parent() {
            if let Some(parent_process) = system.process(parent_pid) {
                return Some(format!(
                    "[name={name:?}, user_id={user_id:?}]",
                    name = parent_process.name().to_string_lossy().into_owned(),
                    user_id = parent_process.user_id().unwrap(),
                ));
            }
        }
    }
    None
}
