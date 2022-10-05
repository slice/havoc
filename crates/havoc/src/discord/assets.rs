use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use thiserror::Error;

use crate::discord::{FeAsset, FeAssetType};
use crate::scrape::NetworkError;

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

#[derive(Error, Debug)]
pub enum AssetError {
    #[error("failed to make network request")]
    Network(#[from] NetworkError),

    #[error("failed to preprocess asset content")]
    Preprocessing(#[from] AnyError),
}

// this is probably good enough amirite
pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

pub type AssetPreprocessor =
    Box<dyn Fn(&[u8]) -> BoxFuture<Result<Vec<u8>, AnyError>> + Send + Sync>;

type AssetContent = Arc<[u8]>;

/// A collection of assets and their scraped content.
pub struct Assets {
    pub assets: Vec<FeAsset>,
    raw_content: HashMap<String, AssetContent>,
    preprocessors: HashMap<FeAssetType, AssetPreprocessor>,
    preprocessed_content: HashMap<String, AssetContent>,
}

impl Assets {
    /// Creates an empty asset collection.
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            raw_content: HashMap::new(),
            preprocessors: HashMap::new(),
            preprocessed_content: HashMap::new(),
        }
    }

    /// Creates a collection from a [`Vec`] of [`FeAsset`]s with an empty content map.
    pub fn with_assets(assets: Vec<FeAsset>) -> Self {
        Self {
            assets,
            ..Default::default()
        }
    }

    /// Sets a preprocessor for a specific asset type.
    pub fn set_preprocessor(&mut self, typ: FeAssetType, preprocessor: AssetPreprocessor) {
        self.preprocessors.insert(typ, preprocessor);
    }

    /// Returns the raw content of an asset, fetching it if necessary.
    ///
    /// This method does not trigger preprocessors.
    pub async fn raw_content(&mut self, asset: &FeAsset) -> Result<Arc<[u8]>, AssetError> {
        match self.raw_content.entry(asset.name.clone()) {
            Entry::Occupied(entry) => {
                tracing::debug!(?asset, "content is already fetched");
                Ok(Arc::clone(entry.get()))
            }
            Entry::Vacant(entry) => {
                tracing::info!(asset = ?asset, "unfetched content requested, fetching...");
                let content = crate::scrape::fetch_url_content(asset.url()).await?;
                Ok(Arc::clone(entry.insert(content.into())))
            }
        }
    }

    /// Returns the content of an asset, fetching and preprocessing it if necessary.
    pub async fn preprocessed_content(&mut self, asset: &FeAsset) -> Result<Arc<[u8]>, AssetError> {
        let raw_content: Arc<[u8]> = self.raw_content(asset).await?.into();

        match self.preprocessed_content.entry(asset.name.clone()) {
            Entry::Vacant(cache_entry) => {
                if let Some(preprocessor) = self.preprocessors.get(&asset.typ) {
                    let preprocessed_content: Arc<[u8]> = preprocessor(&raw_content).await?.into();
                    Ok(Arc::clone(cache_entry.insert(preprocessed_content)))
                } else {
                    Ok(raw_content)
                }
            }
            Entry::Occupied(cache_entry) => Ok(Arc::clone(cache_entry.get())),
        }
    }

    /// Attempts to locate a root script of a certain type.
    pub fn find_root_script(&self, root_script_type: RootScript) -> Option<FeAsset> {
        self.assets
            .iter()
            .filter(|asset| asset.typ == FeAssetType::Js)
            .nth(root_script_type.asset_index())
            .cloned()
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}
