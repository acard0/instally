
use std::fmt::Display;

/// Represents a error that can be shown to end user
/// 
/// The motivation behind this is to provide a way to translate errors to user's language
/// 
/// This paradigm requires creating bunch of error types but it makes errors implicit so that
/// caller function can keep track of whats happening and can provide a better error message
#[derive(thiserror::Error, Debug)]
pub struct Error {
    source: ErrorRepr,
    translation: String
}

#[derive(thiserror::Error, Debug)]
pub struct ErrorRepr {
    source: Box<dyn std::error::Error>,
}

impl Error {
    pub fn new(source: impl std::error::Error + 'static, message: &str) -> Self {
        Self { source: ErrorRepr::new(source), translation: message.to_owned() }
    }
}

impl ErrorRepr {
    pub fn new(source: impl std::error::Error + 'static) -> Self {
        Self { source: Box::new(source) }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.translation, f)
    }
}

impl Display for ErrorRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.source, f)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        let translation = format!("io-error.{}", value.kind().to_string());

        Error {
            source: ErrorRepr::new(value),
            translation
        }
    }
}
