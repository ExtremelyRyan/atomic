//! Main Crate Error


#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    /// For starter, to remove as code matures.
    #[error("Generic error: {0}")]
    Generic(String),
    /// For starter, to remove as code matures.
    #[error("Static error: {0}")]
    Static(&'static str),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}
