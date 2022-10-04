use std::fmt::{Display, Formatter};

use crate::artifact::Artifact;
use crate::discord::{Branch, FeAsset};

use serde::Serialize;

/// A frontend manifest.
///
/// A manifest is a surface-level representation of a Discord client build
/// which contains only minimal information; namely, the branch the build
/// is associated with, and the client page's assets.
///
/// [`FeBuild`](crate::discord::FeBuild)s contain a superset of this
/// information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct FeManifest {
    pub branch: Branch,
    pub assets: Vec<FeAsset>,
}

impl Display for FeManifest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Discord {:?} ({} asset(s))",
            self.branch,
            self.assets.len()
        )
    }
}

impl Artifact for FeManifest {
    fn assets(&self) -> &[FeAsset] {
        &self.assets
    }

    fn dump_prefix(&self) -> String {
        format!("fe_{}", format!("{:?}", self.branch).to_ascii_lowercase())
    }
}
