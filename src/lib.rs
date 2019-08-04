use std::error::Error;
use std::fmt;
use std::fmt::Formatter;

pub mod author;
pub mod commands;
pub mod commit;
pub mod database;
pub mod diff;
pub mod index;
pub mod lockfile;
pub mod refs;
pub mod repository;
pub mod revision;
pub mod tree;
pub mod utilities;
pub mod workspace;

pub type BoxResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct CliError {
    details: String,
}

impl CliError {
    pub fn new(msg: &str) -> Self {
        Self {
            details: msg.to_string(),
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for CliError {
    fn description(&self) -> &str {
        &self.details
    }
}
