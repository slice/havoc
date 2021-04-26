/// A Discord branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Stable,
    Ptb,
    Canary,
    Development,
}

/// A kind of frontend asset.
///
/// Here we model the most common types, but you can specify your own custom
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeAssetType {
    Css,
    Js,
    Ico,
    Svg,
    Custom(String),
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
            Custom(ext) => &ext,
        }
    }
}

/// A frontend asset.
///
/// This refers to a CSS, JS, ICO, or SVG file that has been deployed onto
/// Discord's CDN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeAsset {
    pub name: String,
    pub typ: FeAssetType,
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
