use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;

use crate::discord::{FeAsset, FeAssetType};
use crate::scrape::ScrapeError;

/// The types of various `<script>` tags in Discord application's HTML.
/// Keep in mind that these are fragile assumptions and could potentially
/// change at any time.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RootScript {
    /// A script which handles loading other Webpack chunks that aren't root
    /// script.
    ChunkLoader,

    /// The Webpack chunk containing CSS chunk class mappings.
    Classes,

    /// The Webpack chunk containing various vendor modules, such as Sentry.
    Vendor,

    /// The principal Webpack chunk containing the bulk of the app code.
    Entrypoint,
}

impl RootScript {
    /// Returns an index index into the script asset list that corresponds to this root script.
    /// This is another fragile assumption that could change at any time.
    fn asset_index(&self) -> usize {
        use RootScript::*;

        match self {
            ChunkLoader => 0,
            Classes => 1,
            Vendor => 2,
            Entrypoint => 3,
        }
    }
}

/// Encapsulates assets and their scraped content.
pub struct Assets {
    pub assets: Vec<Rc<FeAsset>>,
    content: HashMap<String, String>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            content: HashMap::new(),
        }
    }

    pub fn with_assets(assets: Vec<Rc<FeAsset>>) -> Self {
        Self {
            assets,
            content: HashMap::new(),
        }
    }

    /// Returns the content of an asset, fetching it if necessary.
    pub fn content(&mut self, asset: &FeAsset) -> Result<&str, ScrapeError> {
        match self.content.entry(asset.name.clone()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                tracing::info!(asset = ?asset, "content requested for unfetched asset, fetching...");
                let content = crate::scrape::get_text(asset.url())?;
                Ok(entry.insert(content))
            }
        }
    }

    /// Attempts to locate a root script of a certain type.
    pub fn find_root_script(&self, root_script_type: RootScript) -> Option<Rc<FeAsset>> {
        self.assets
            .iter()
            .filter(|asset| asset.typ == FeAssetType::Js)
            .nth(root_script_type.asset_index())
            .cloned()
    }
}
