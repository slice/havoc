use serde::Serialize;
use std::ops::{Deref, DerefMut};

use crate::discord::{FeAsset, FeAssetType, RootScript};

// TODO: Maybe replace with AssetsExt. X_X

/// A collection of [`crate::discord::FeAsset`]s.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Assets {
    /// The contained assets.
    #[serde(flatten)]
    pub inner: Vec<FeAsset>,
}

impl Assets {
    /// Returns an iterator over all assets of a certain type contained within
    /// this `Assets`.
    pub fn filter_by_type(&self, typ: FeAssetType) -> impl Iterator<Item = &FeAsset> {
        Box::new(self.inner.iter().filter(move |asset| asset.typ == typ))
    }

    /// Tries to find a script that corresponds to the assumed index of a kind
    /// of root script.
    pub fn find_root_script(&self, root_script_type: RootScript) -> Option<FeAsset> {
        self.filter_by_type(FeAssetType::Js)
            .nth(root_script_type.assumed_index())
            .cloned()
    }
}

impl Deref for Assets {
    type Target = Vec<FeAsset>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Assets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a> IntoIterator for &'a Assets {
    type Item = &'a FeAsset;
    type IntoIter = <&'a Vec<FeAsset> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl From<Vec<FeAsset>> for Assets {
    fn from(value: Vec<FeAsset>) -> Self {
        Self { inner: value }
    }
}
