use serde::Serialize;
use url::Url;

/// A kind of frontend asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FeAssetType {
    Css,
    Js,
    Ico,
    Svg,
    Webm,
    Webp,
    Gif,
}

impl FeAssetType {
    /// Returns the file extension of this asset type.
    pub fn ext(&self) -> &str {
        use FeAssetType::*;

        match self {
            Css => "css",
            Js => "js",
            Ico => "ico",
            Svg => "svg",
            Webm => "webm",
            Webp => "webp",
            Gif => "gif",
        }
    }
}

/// A frontend asset.
///
/// This refers to a file that has been deployed onto Discord's CDN.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct FeAsset {
    pub name: String,
    #[serde(rename = "type")]
    pub typ: FeAssetType,
}

impl FeAsset {
    /// Returns the combined name and extension of this asset separated by a
    /// period, akin to a filename.
    pub fn filename(&self) -> String {
        format!("{}.{}", self.name, self.typ.ext())
    }

    /// Returns a [`Url`] to this asset.
    pub fn url(&self) -> Url {
        let base = Url::parse("https://discord.com/assets/").unwrap();
        base.join(&self.filename())
            .expect("failed to construct asset url")
    }
}
