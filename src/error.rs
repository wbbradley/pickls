use handlebars::RenderError;
use std::{num::ParseIntError, panic::Location};

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait Context<T> {
    fn context(self, context: &str) -> Result<T>;
    fn ok_or_log(self, context: &str) -> Option<T>
    where
        Self: Sized,
    {
        match self.context(context) {
            Ok(value) => Some(value),
            Err(error) => {
                log::error!("{error}");
                None
            }
        }
    }
}

impl<T, E: std::fmt::Display> Context<T> for std::result::Result<T, E> {
    #[track_caller]
    #[inline]
    fn context(self, context: &str) -> Result<T> {
        self.map_err(|e| Error::new(format!("{context}: {e}")))
    }
}

#[derive(Debug)]
pub struct Error {
    message: String,
    location: &'static Location<'static>,
}

impl Error {
    #[allow(dead_code)]
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: Location::caller(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [location={}]", self.message, self.location)
    }
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    #[track_caller]
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Self {
            message: format!("dyn error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<regex::Error> for Error {
    #[track_caller]
    fn from(error: regex::Error) -> Self {
        Self {
            message: format!("regex error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<serde_yml::Error> for Error {
    fn from(error: serde_yml::Error) -> Self {
        Self {
            message: format!("yaml error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self {
            message: format!("json error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        Self {
            message: format!("io error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<toml::de::Error> for Error {
    #[track_caller]
    fn from(error: toml::de::Error) -> Self {
        Self {
            message: format!("toml error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for Error {
    #[track_caller]
    fn from(error: std::sync::mpsc::SendError<T>) -> Self {
        Self {
            message: format!("mpsc send error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<RenderError> for Error {
    #[track_caller]
    fn from(error: RenderError) -> Self {
        Self {
            message: format!("render error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for Error {
    #[track_caller]
    fn from(error: crossbeam_channel::SendError<T>) -> Self {
        Self {
            message: format!("crossbeam channel send error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<String> for Error {
    #[track_caller]
    fn from(error: String) -> Self {
        Self {
            message: format!("error: {error}"),
            location: Location::caller(),
        }
    }
}

impl From<&str> for Error {
    #[track_caller]
    fn from(error: &str) -> Self {
        Self {
            message: format!("error: {error}"),
            location: Location::caller(),
        }
    }
}

impl From<ParseIntError> for Error {
    #[track_caller]
    fn from(error: ParseIntError) -> Self {
        Self {
            message: format!("parse int error: {error:?}"),
            location: Location::caller(),
        }
    }
}

impl From<anyhow::Error> for Error {
    #[track_caller]
    fn from(error: anyhow::Error) -> Self {
        Self {
            message: format!("anyhow error: {error:?}"),
            location: Location::caller(),
        }
    }
}
