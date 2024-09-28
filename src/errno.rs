use crate::error::Result;
use libc::strerror;
use std::ffi::CStr;
use std::fs::File;
use std::io::BufRead;
use std::io::{BufReader, Lines};
use std::path::Path;

#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub mod errors {
    use libc::__errno_location;
    pub fn get_errno() -> c_int {
        unsafe { *__errno_location() }
    }
}

#[allow(dead_code)]
#[cfg(target_os = "macos")]
pub mod errors {
    use libc::__error;
    pub fn get_errno() -> libc::c_int {
        unsafe { *__error() }
    }
}

#[allow(unused_macros)]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}
#[allow(unused_imports)]
pub(crate) use function;

#[allow(unused_macros)]
macro_rules! trace_fn {
    ($($args:expr),+) => {{
        log::trace!("[{} called] {}", function!(),
            format!($($args),+));
    }};
}
#[allow(unused_imports)]
pub(crate) use trace_fn;

#[derive(Copy, Clone)]
pub struct Errno {
    errno: libc::c_int,
}

impl Errno {
    pub fn latest() -> Self {
        Self {
            errno: errors::get_errno(),
        }
    }
    pub fn is_enoent(self) -> bool {
        self.errno == libc::ENOENT
    }
    pub fn is_eagain(self) -> bool {
        self.errno == libc::EAGAIN
    }
    pub fn is_error(self) -> bool {
        self.errno != 0
    }
}

impl std::fmt::Display for Errno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        write!(f, "{}", s)
    }
}

impl From<i32> for Errno {
    fn from(errno: libc::c_int) -> Self {
        Self { errno }
    }
}

impl From<Errno> for String {
    fn from(errno: Errno) -> Self {
        String::from_utf8_lossy(unsafe { CStr::from_ptr(strerror(errno.errno)) }.to_bytes())
            .to_string()
    }
}

pub fn read_lines<P>(filename: P) -> Result<Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}

#[cfg(test)]
pub fn open_dev_null() -> libc::c_int {
    unsafe {
        match std::ffi::CString::new("/dev/null") {
            Ok(name) => libc::open(name.as_ptr(), libc::O_RDWR, 0o644),
            Err(_) => panic!("failed to make cstring"),
        }
    }
}
