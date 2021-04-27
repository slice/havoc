use url::Url;

/// A Discord branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Stable,
    Ptb,
    Canary,
    Development,
}

impl Branch {
    /// Returns the base URL of this branch.
    pub fn base(&self) -> Url {
        use Branch::*;

        match self {
            Stable => "https://discord.com".parse().unwrap(),
            Ptb => "https://ptb.discord.com".parse().unwrap(),
            Canary => "https://canary.discord.com".parse().unwrap(),
            Development => panic!("called `Branch::base()` on `Branch::Development`"),
        }
    }
}

/// A kind of frontend asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeAsset {
    pub name: String,
    pub typ: FeAssetType,
}

impl FeAsset {
    /// Returns a `Url` to this asset.
    pub fn url(&self) -> Url {
        let base = Url::parse("https://discord.com/assets/").unwrap();
        base.join(&format!("{}.{}", self.name, self.typ.ext()))
            .expect("failed to construct asset url")
    }
}

/// A frontend build.
///
/// "Frontend" refers to the web application that is deployed to `discord.com`,
/// `canary.discord.com`, etc. It should be clarified that the desktop
/// application loads these pages, too; it just enables additional
/// functionality such as push to talk, keybinds, etc.
#[derive(Debug, Clone)]
pub struct FeBuild {
    pub branch: Branch,
    pub hash: String,
    pub number: u32,
    pub assets: Vec<FeAsset>,
}

impl PartialEq for FeBuild {
    fn eq(&self, other: &Self) -> bool {
        // avoid comparing all fields and rely on the build number
        self.number == other.number
    }
}

impl Eq for FeBuild {}
