use std::fmt::Display;

use convert_case::{Case, Casing};

use crate::*;

#[derive(thiserror::Error, Debug)]
pub struct Error {
    source: Repr,
    details: ErrorDetails,
}

#[derive(thiserror::Error, Debug)]
pub struct Repr {
    source: Box<dyn std::error::Error>,
}

#[derive(Debug, Clone)]
pub struct ErrorDetails {
    pub name: String,
    pub message: String,
    pub suggestion: Option<String>,
}

pub trait AsDetails {
    fn as_details(&self) -> ErrorDetails;
    fn get_message_key(&self) -> String;
    fn get_suggestion_key(&self) -> String;
}

impl Error {
    pub fn new(source: impl std::error::Error + 'static, details: ErrorDetails) -> Self {
        Self { 
            source: Repr::new(source), 
            details
        }
    }

    pub fn get_details(&self) -> &ErrorDetails {
        &self.details
    }
    
    pub fn get_source(&self) -> &dyn std::error::Error {
        &*self.source.source
    }
}

impl Repr {
    pub fn new(source: impl std::error::Error + 'static) -> Self {
        Self { source: Box::new(source) }
    }
}

impl ErrorDetails {
    pub fn new(name: &str, message: &str, suggestion: Option<String>) -> Self {
        Self { name: name.to_owned(), message: message.to_string(), suggestion }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.get_details().message, f)
    }
}

impl Display for Repr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.source, f)
    }
}

impl Display for ErrorDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.message, f)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        let details = value.as_details();
        Error::new(value, details)
    }
}

impl AsDetails for std::io::Error {
    fn as_details(&self) -> ErrorDetails {
        let name = "io-error";
        let message_key = t!(&self.get_message_key());
        let suggestion_key = t!(&self.get_suggestion_key());

        ErrorDetails::new(name, &message_key, Some(suggestion_key.to_owned()))
    }

    fn get_message_key(&self) -> String {
        let name = "io-error";
        let kind = self.kind().to_string().to_case(Case::Kebab);
        format!("{}.{}", &name, &kind)
    }

    fn get_suggestion_key(&self) -> String {
        format!("{}.suggestion", self.get_message_key())
    }
}