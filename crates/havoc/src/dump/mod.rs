//! Types and interfaces that handle useful data extraction.

use std::{borrow::Cow, path::Path};

use crate::discord::assets::AssetError;
use crate::{discord::Assets, scrape::ScrapeError};

pub mod modules;
pub use modules::WebpackModules;

pub mod classes;
pub use classes::CSSClasses;
use thiserror::Error;

/// A dump result, returned by [`Dump::dump`](Dump::dump).
pub struct DumpResult {
    pub name: String,
    pub content: DumpContent,
}

impl DumpResult {
    pub fn from_serializable<T: serde::Serialize>(
        value: &T,
        name: &str,
    ) -> Result<DumpResult, serde_json::Error> {
        let value = serde_json::to_value(value)?;

        Ok(DumpResult {
            name: name.to_owned(),
            content: DumpContent::Json(value),
        })
    }

    pub fn filename(&self) -> String {
        match self.content {
            DumpContent::Json(_) => format!("{}.json", self.name),
            DumpContent::Text {
                content: _,
                ref extension,
            } => format!("{}.{}", self.name, extension),
        }
    }

    pub fn writable_content(&self) -> Result<Cow<str>, DumpWriteError> {
        Ok(match self.content {
            DumpContent::Json(ref value) => Cow::Owned(serde_json::to_string(value)?),
            DumpContent::Text { ref content, .. } => Cow::Borrowed(content),
        })
    }

    pub fn write(&self, destination: &Path) -> Result<(), DumpWriteError> {
        std::fs::write(destination, &*self.writable_content()?)?;
        Ok(())
    }
}

/// The content of a dump.
pub enum DumpContent {
    Json(serde_json::Value),
    Text { content: String, extension: String },
}

/// Errors that can occur while writing a dump to disk.
#[derive(Error, Debug)]
pub enum DumpWriteError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("failed to serialize dump content into JSON")]
    SerializationFailed(#[from] serde_json::Error),
}

/// Errors that can occur while dumping from an artifact.
#[derive(Error, Debug)]
pub enum DumpError {
    #[error("failed to scrape")]
    ScrapeFailed(#[from] ScrapeError),

    #[error("failed to resolve asset")]
    Asset(#[from] AssetError),

    #[error("failed to serialize to JSON")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("failed to parse/traverse JS")]
    JSParseError(#[from] crate::parse::ParseError),
}

#[async_trait::async_trait]
pub trait Dump {
    async fn dump(&mut self, assets: &mut Assets) -> Result<DumpResult, DumpError>;
}
