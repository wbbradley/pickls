use std::process;
pub use sysinfo::{Pid, System};

pub async fn fetch_parent_process_info() -> String {
    let mut system = System::new_all();
    system.refresh_all();

    let current_pid = process::id();
    if let Some(current_process) = system.process(Pid::from(current_pid as usize)) {
        if let Some(parent_pid) = current_process.parent() {
            if let Some(parent_process) = system.process(parent_pid) {
                return format!(
                    "[name={name:?}, user_id={user_id:?}]",
                    name = parent_process.name().to_string_lossy().into_owned(),
                    user_id = parent_process.user_id(),
                );
            }
        }
    }
    "<unknown parent process>".to_string()
}
