//! Types that handle extracting data from [`Artifact`](crate::artifact::Artifact)s ("dumping").

use std::path::Path;

use thiserror::Error;

use crate::scrape::ScrapeError;

/// Something that can potentially be dumped from an artifact.
///
/// Each artifact declares what dump item it supports through
/// [`Artifact::supports_dump_item`](crate::artifact::Artifact::supports_dump_item).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DumpItem {
    /// Dump a representation of the artifact itself.
    Itself,

    /// Dump the mapping of module IDs to class names mappings.
    CssClasses,

    /// Dump a representation of Webpack modules within the artifact.
    WebpackModules,
}

impl std::str::FromStr for DumpItem {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DumpItem::*;

        match s {
            "classes" => Ok(CssClasses),
            "modules" => Ok(WebpackModules),
            "self" => Ok(Itself),
            _ => Err(()),
        }
    }
}

/// Types of dump results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DumpResultType {
    Js,
    Json,
}

impl DumpResultType {
    pub fn ext(&self) -> &'static str {
        use DumpResultType::*;

        match self {
            Js => "js",
            Json => "json",
        }
    }
}

/// A dump result, returned by [`Artifact::dump`](crate::artifact::Artifact::dump).
pub struct DumpResult {
    pub name: String,
    pub typ: DumpResultType,
    pub content: String,
}

/// Errors that can occur while writing a dump to disk.
#[derive(Error, Debug)]
pub enum DumpWriteError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("specified destination was invalid")]
    InvalidDestination,
}

/// Errors that can occur while dumping from an artifact.
#[derive(Error, Debug)]
pub enum DumpError {
    #[error("failed to scrape")]
    ScrapeFailed(#[from] ScrapeError),

    #[error("failed to serialize to JSON")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("failed to parse/traverse JS")]
    JSParseError(#[from] crate::parse::ParseError),
}

impl DumpResult {
    pub fn from_serializable<T: serde::Serialize>(
        value: &T,
        name: &str,
    ) -> Result<DumpResult, serde_json::Error> {
        let content = serde_json::to_string(value)?;

        Ok(DumpResult {
            name: name.to_owned(),
            typ: DumpResultType::Json,
            content,
        })
    }

    pub fn filename(&self) -> String {
        format!("{}.{}", self.name, self.typ.ext())
    }

    pub fn write(&self, destination: &Path) -> Result<(), DumpWriteError> {
        std::fs::write(destination, &self.content)?;
        Ok(())
    }
}
