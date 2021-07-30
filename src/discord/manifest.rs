use std::error::Error;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::artifact::{Artifact, DumpItem, DumpResult};
use crate::discord::{Assets, Branch, FeAsset};

use serde::Serialize;

/// A frontend manifest.
///
/// The term "manifest" refers to a surface-level representation of a build
/// which only contains minimal information. Further details can be gathered
/// using the data within this structure. For more information,
/// see [`FeBuild`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct FeManifest {
    pub branch: Branch,
    pub assets: Vec<Rc<FeAsset>>,
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
    fn assets(&self) -> &[Rc<FeAsset>] {
        &self.assets
    }

    fn dump_prefix(&self) -> String {
        format!("fe_{}", format!("{:?}", self.branch).to_ascii_lowercase())
    }

    fn dump(
        &self,
        _: DumpItem,
        _: &mut Assets,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        panic!("unsupported dump operation")
    }
}
