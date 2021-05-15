pub mod asset;
pub mod branch;

use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub use asset::*;
pub use branch::*;

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
    pub manifest: FeManifest,
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
