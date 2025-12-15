use std::error::Error;
use std::fmt;

#[derive(Clone, Debug)]
pub enum LocaleError {
    InvalidTemplate,
    InvalidLocale,
}

impl fmt::Display for LocaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::InvalidTemplate => "Invalid template string",
            Self::InvalidLocale => "Invalid locale string",
        };
        write!(f, "{msg}")
    }
}

impl Error for LocaleError {}
