use std::panic::Location;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    message: String,
    location: &'static Location<'static>,
}

impl Error {
    #[allow(dead_code)]
    #[track_caller]
    pub fn new(message: String) -> Self {
        Self {
            message,
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
