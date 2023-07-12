
#[derive(thiserror::Error, Debug)]
pub enum IJSError {
    #[error("{0}")]
    IOError(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}