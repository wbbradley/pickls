use crate::prelude::*;

pub fn get_extension_from_url(url: &Url) -> Option<String> {
    if let Some(path) = url.path_segments() {
        if let Some(filename) = path.last() {
            return filename.rsplit('.').next().map(String::from);
        }
    }
    None
}
