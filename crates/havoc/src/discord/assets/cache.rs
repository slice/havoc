use futures::future::BoxFuture;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::discord::{FeAsset, FeAssetType};
use crate::scrape::NetworkError;

// this is probably good enough amirite
pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

pub type AssetPreprocessor =
    Box<dyn Fn(&[u8]) -> BoxFuture<Result<Vec<u8>, AnyError>> + Send + Sync>;

pub type AssetContent = Vec<u8>;

/// Keeps assets' contents in memory to prevent repeated fetching.
///
/// Because this type provides an abstraction over fetching the content of
/// assets, it takes on the further responsibility of preprocessing them as well
/// (since you typically want to cache them too).
pub struct AssetCache {
    raw_content: HashMap<String, AssetContent>,
    preprocessors: HashMap<FeAssetType, AssetPreprocessor>,
    preprocessed_content: HashMap<String, AssetContent>,
}

impl AssetCache {
    /// Creates an empty asset cache.
    pub fn new() -> Self {
        Self {
            raw_content: HashMap::new(),
            preprocessors: HashMap::new(),
            preprocessed_content: HashMap::new(),
        }
    }

    /// Indicates a preprocessor to be used for a specific asset type.
    ///
    /// This will overwrite any previously set preprocessor.
    pub fn set_preprocessor(&mut self, typ: FeAssetType, preprocessor: AssetPreprocessor) {
        self.preprocessors.insert(typ, preprocessor);
    }

    /// Returns the raw (un-preprocessed) content of an asset, fetching it and
    /// caching it if necessary.
    pub async fn raw_content(&mut self, asset: &FeAsset) -> Result<&[u8], NetworkError> {
        raw_content_inner(&mut self.raw_content, asset).await
    }

    /// Returns the preprocessed content of an asset, fetching and caching both
    /// the raw and preprocessed work if necessary.
    pub async fn preprocessed_content(
        &mut self,
        asset: &FeAsset,
    ) -> Result<Result<&[u8], AnyError>, NetworkError> {
        match self.preprocessed_content.entry(asset.name.clone()) {
            Entry::Vacant(cache_entry) => {
                let raw_content = raw_content_inner(&mut self.raw_content, asset).await?;
                if let Some(preprocessor) = self.preprocessors.get(&asset.typ) {
                    Ok(preprocessor(raw_content)
                        .await
                        .map(|content| cache_entry.insert(content).as_slice()))
                } else {
                    Ok(Ok(raw_content))
                }
            }
            Entry::Occupied(cache_entry) => Ok(Ok(cache_entry.into_mut())),
        }
    }
}

async fn raw_content_inner<'cache>(
    content_cache: &'cache mut HashMap<String, AssetContent>,
    asset: &'_ FeAsset,
) -> Result<&'cache [u8], NetworkError> {
    match content_cache.entry(asset.name.clone()) {
        Entry::Occupied(entry) => {
            tracing::debug!(?asset, "asset content is cached");
            Ok(entry.into_mut())
        }
        Entry::Vacant(entry) => {
            use isahc::AsyncReadResponseExt;
            tracing::info!(?asset, "unfetched asset content requested, fetching");
            let mut response = crate::scrape::get_async(asset.url()).await?;
            Ok(entry.insert(response.bytes().await.map_err(NetworkError::Io)?))
        }
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}
