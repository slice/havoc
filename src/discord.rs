use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use url::Url;

/// A Discord branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn has_frontend(&self) -> bool {
        *self != Branch::Development
    }
}

impl std::str::FromStr for Branch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Branch::*;

        match s {
            "stable" => Ok(Stable),
            "ptb" => Ok(Ptb),
            "canary" => Ok(Canary),
            "development" => Ok(Development),
            _ => Err(()),
        }
    }
}

/// A kind of frontend asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeAsset {
    pub name: String,
    pub typ: FeAssetType,
}

impl FeAsset {
    /// Returns a [`Url`] to this asset.
    pub fn url(&self) -> Url {
        let base = Url::parse("https://discord.com/assets/").unwrap();
        base.join(&format!("{}.{}", self.name, self.typ.ext()))
            .expect("failed to construct asset url")
    }
}

/// A frontend manifest.
///
/// "Manifest" refers to a surface-level snapshot of a build which only
/// contains minimal information. Further details can be gleaned from the data
/// within this structure. For more information, see [`FeBuild`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeManifest {
    pub branch: Branch,
    pub assets: Vec<Rc<FeAsset>>,
}

/// A frontend build.
///
/// "Frontend" refers to the web application that is deployed to `discord.com`,
/// `canary.discord.com`, etc. It should be clarified that the desktop
/// application loads these pages, too; it just enables additional
/// functionality such as push to talk, keybinds, etc.
#[derive(Debug, Clone)]
pub struct FeBuild {
    pub manifest: Weak<FeManifest>,
    pub hash: String,
    pub number: u32,
}

impl Hash for FeBuild {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
        self.number.hash(state);
    }
}

impl PartialEq for FeBuild {
    fn eq(&self, other: &Self) -> bool {
        // avoid comparing all fields and rely on the build number
        self.number == other.number
    }
}

impl Eq for FeBuild {}
