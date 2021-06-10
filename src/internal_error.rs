use rusqlite;
use std::error::Error;
use std::io;

use rocket::response::Responder;

use std::fmt;
use std::sync::PoisonError;

#[derive(Debug, Responder)]
#[response(status = 500, content_type = "json")]
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

impl From<io::Error> for InternalError {
    fn from(e: io::Error) -> InternalError {
        InternalError {
            what: e.to_string(),
        }
    }
}

impl From<&str> for InternalError {
    fn from(s: &str) -> InternalError {
        InternalError {
            what: s.to_string(),
        }
    }
}

pub type InternalResult<T> = Result<T, InternalError>;
