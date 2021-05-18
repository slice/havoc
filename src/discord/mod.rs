pub mod asset;
pub mod branch;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::artifact::{Artifact, AssetContentMap, DumpItem, DumpResult};
use crate::util::measure;
pub use asset::*;
pub use branch::*;

use serde::Serialize;

/// A frontend manifest.
///
/// "Manifest" refers to a surface-level snapshot of a build which only
/// contains minimal information. Further details can be gleaned from the data
/// within this structure. For more information, see [`FeBuild`].
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
        _: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        panic!("unsupported dump operation")
    }
}

/// A frontend build.
///
/// "Frontend" refers to the web application that is deployed to `discord.com`,
/// `canary.discord.com`, etc. It should be clarified that the desktop
/// application loads these pages, too; it just enables additional
/// functionality such as push to talk, keybinds, etc.
#[derive(Debug, Clone, Serialize)]
pub struct FeBuild {
    #[serde(flatten)]
    pub manifest: FeManifest,
    pub hash: String,
    pub number: u32,
}

impl FeBuild {
    pub fn parse_classes(
        &self,
        acm: &AssetContentMap,
    ) -> Result<crate::parse::ClassModuleMap, Box<dyn Error + Send + Sync>> {
        let asset = &self.manifest.assets.get(1).ok_or("no classes asset")?;
        let js = acm.get(*asset).ok_or("couldn't find classes js")?;
        let script = crate::parse::parse_script(&js)?;
        let mapping =
            crate::parse::walk_classes_chunk(&script).map_err(|_| "failed to walk classes js")?;

        Ok(mapping)
    }

    fn dump_classes(
        &self,
        acm: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        let class_module_map = self.parse_classes(acm)?;
        Ok(vec![DumpResult::from_serializable(
            &class_module_map,
            "classes",
        )?])
    }

    fn parse_webpack_chunks(
        &self,
        acm: &AssetContentMap,
    ) -> Result<(swc_ecma_ast::Script, crate::parse::WebpackChunk), Box<dyn Error + Send + Sync>>
    {
        let assets = &self.manifest.assets;

        let last_script = assets
            .iter()
            .filter(|asset| asset.typ == crate::discord::FeAssetType::Js)
            .last()
            .ok_or("couldn't find entrypoint js")?;

        let entrypoint_js = acm.get(last_script).ok_or("no entrypoint js content")?;

        let script = measure("parsing entrypoint script", || {
            crate::parse::parse_script(&entrypoint_js)
        })?;

        let chunk = crate::parse::walk_webpack_chunk(&script)?;

        Ok((script, chunk))
    }

    fn dump_webpack_chunks(
        &self,
        acm: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        let (_, webpack_chunk) = self.parse_webpack_chunks(acm)?;
        Ok(vec![DumpResult::from_serializable(
            &webpack_chunk.modules,
            "entrypoint",
        )?])
    }
}

impl Display for FeBuild {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Discord {:?} {}", self.manifest.branch, self.number)
    }
}

impl Artifact for FeBuild {
    fn assets(&self) -> &[Rc<FeAsset>] {
        &self.manifest.assets
    }

    fn dump_prefix(&self) -> String {
        let branch = format!("{:?}", self.manifest.branch).to_ascii_lowercase();
        format!("fe_{}_{}", branch, self.number)
    }

    fn supports_dump_item(&self, item: DumpItem) -> bool {
        matches!(
            item,
            DumpItem::CssClasses | DumpItem::WebpackChunks | DumpItem::Itself
        )
    }

    fn dump(
        &self,
        item: DumpItem,
        acm: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        match item {
            DumpItem::CssClasses => self.dump_classes(acm),
            DumpItem::WebpackChunks => self.dump_webpack_chunks(acm),
            DumpItem::Itself => Ok(vec![DumpResult::from_serializable(self, "build")?]),
        }
    }
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
