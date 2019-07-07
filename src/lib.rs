pub mod author;
pub mod commands;
pub mod commit;
pub mod database;
pub mod diff;
pub mod index;
pub mod lockfile;
pub mod refs;
pub mod repository;
pub mod tree;
pub mod utilities;
pub mod workspace;

pub type BoxResult<T> = Result<T, Box<dyn std::error::Error>>;
