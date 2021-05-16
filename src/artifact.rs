use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::rc::Rc;

use thiserror::Error;

use super::discord::FeAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DumpItem {
    CssClasses,
    WebpackChunks,
}

impl std::str::FromStr for DumpItem {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DumpItem::*;

        match s {
            "classes" => Ok(CssClasses),
            "chunks" => Ok(WebpackChunks),
            _ => Err(()),
        }
    }
}

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

pub struct DumpResult {
    pub name: String,
    pub typ: DumpResultType,
    pub content: String,
}

#[derive(Error, Debug)]
pub enum DumpError {
    #[error("i/o error")]
    Io(#[from] std::io::Error),

    #[error("specified destination was invalid")]
    InvalidDestination,
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

    pub fn dump_to(&self, destination: &Path) -> Result<(), DumpError> {
        std::fs::write(destination, &self.content)?;
        Ok(())
    }
}

pub type AssetContentMap = HashMap<Rc<FeAsset>, String>;

pub trait Artifact: Display {
    /// Returns whether a dump item is supported or not.
    fn supports_dump_item(&self, _item: DumpItem) -> bool {
        false
    }

    /// Returns the dump prefix for this artifact, which is intended to be
    /// prepended to [`DumpResult`] filenames when dumping.
    fn dump_prefix(&self) -> String {
        "".to_owned()
    }

    /// Dumps some data from this artifact.
    fn dump(
        &self,
        item: DumpItem,
        asset_content_nap: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>>;

    /// Returns the assets associated with this artifact.
    fn assets(&self) -> &[Rc<FeAsset>];
}
