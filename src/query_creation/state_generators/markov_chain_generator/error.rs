use core::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct SyntaxError {
    reason: String,
}

impl SyntaxError {
    pub fn new(reason: String) -> Self {
        SyntaxError { reason }
    }
}

impl Error for SyntaxError {}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.reason)
    }
}
