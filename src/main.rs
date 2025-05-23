#![warn(clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

//! Main execution point
mod cli;
mod command;
mod git;
mod plugin;
mod schema;
mod template;
mod toml;
fn main() {
    cli::start_cli();
}

// Alias Result to be the crate Result.
pub(crate) type Result<T> = core::result::Result<T, AtomicError>;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum AtomicError {
    #[error("Generic error: {0}")]
    Generic(String),
    #[error("Static error: {0}")]
    Static(&'static str),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    GitError(#[from] git2::Error),
}
