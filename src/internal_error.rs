use rusqlite;
use std::error::Error;

use std::fmt;
use std::sync::PoisonError;

#[derive(Debug)]
pub struct InternalError {
    what: String,
}

impl Error for InternalError {}
impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Generic internal error: {}", self.what)
    }
}

impl<T> From<PoisonError<T>> for InternalError {
    fn from(e: PoisonError<T>) -> InternalError {
        InternalError {
            what: e.to_string(),
        }
    }
}

impl From<rusqlite::Error> for InternalError {
    fn from(e: rusqlite::Error) -> InternalError {
        InternalError {
            what: e.to_string(),
        }
    }
}

pub type InternalResult<T> = Result<T, InternalError>;
