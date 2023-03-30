use serde::Serialize;
use std::ops::Deref;

use crate::discord::{FeAsset, FeAssetType, RootScript};

/// A collection of [`crate::discord::FeAsset`]s.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Assets {
    /// The contained assets.
    #[serde(flatten)]
    pub inner: Vec<FeAsset>,
}

impl Assets {
    /// Filters all assets by type.
    pub fn filter_by_type(
        &self,
        typ: FeAssetType,
    ) -> Box<dyn Iterator<Item = &FeAsset> + '_ + Send> {
        Box::new(self.inner.iter().filter(move |asset| asset.typ == typ))
    }

    /// Attempts to locate a root script of a certain type.
    pub fn find_root_script(&self, root_script_type: RootScript) -> Option<FeAsset> {
        self.filter_by_type(FeAssetType::Js)
            .nth(root_script_type.assumed_index())
            .cloned()
    }
}

impl Deref for Assets {
    type Target = [FeAsset];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> IntoIterator for &'a Assets {
    type Item = &'a FeAsset;
    type IntoIter = <&'a Vec<FeAsset> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.inner).into_iter()
    }
}

impl From<Vec<FeAsset>> for Assets {
    fn from(value: Vec<FeAsset>) -> Self {
        Self { inner: value }
    }
}
