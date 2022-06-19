use std::{str::FromStr, char::ParseCharError};

#[derive(Debug, PartialEq, Eq)]
pub enum Format {
    Raw,
    Text,
    Html,
}

impl Default for Format {
    fn default() -> Self {
        Self::Raw
    }
}

impl FromStr for Format {
    type Err = ParseCharError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Text" => Ok(Format::Text),
            "Html" => Ok(Format::Html),
            _ => Ok(Format::Raw)
        }
    }
}