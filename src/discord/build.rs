use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::artifact::{Artifact, AssetContentMap, DumpItem, DumpResult};
use crate::discord::{FeAsset, FeAssetType, FeManifest};
use crate::parse::webpack::ModuleId;

use serde::Serialize;

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

    fn parse_webpack_chunk<'acm>(
        &self,
        acm: &'acm AssetContentMap,
    ) -> Result<(swc_ecma_ast::Script, HashMap<ModuleId, &'acm str>), Box<dyn Error + Send + Sync>>
    {
        let assets = &self.manifest.assets;

        let last_script = assets
            .iter()
            .filter(|asset| asset.typ == FeAssetType::Js)
            .last()
            .ok_or("couldn't find entrypoint js")?;

        let entrypoint_js = acm.get(last_script).ok_or("no entrypoint js content")?;

        tracing::info!("parsing entrypoint script");
        let script = crate::parse::parse_script(&entrypoint_js)?;

        let chunk = crate::parse::walk_webpack_chunk(&script)?;

        let modules: HashMap<ModuleId, &str> = chunk
            .modules
            .iter()
            .map(|(module_id, module)| {
                use swc_common::Spanned;
                let span = module.func.span();

                let lo = span.lo.0 as usize;
                let hi = span.hi.0 as usize;
                (*module_id, &entrypoint_js[lo..hi])
            })
            .collect();

        Ok((script, modules))
    }

    fn dump_webpack_modules(
        &self,
        acm: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        let (_, modules) = self.parse_webpack_chunk(acm)?;
        Ok(vec![DumpResult::from_serializable(&modules, "entrypoint")?])
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
            DumpItem::CssClasses | DumpItem::WebpackModules | DumpItem::Itself
        )
    }

    fn dump(
        &self,
        item: DumpItem,
        acm: &AssetContentMap,
    ) -> Result<Vec<DumpResult>, Box<dyn Error + Send + Sync>> {
        match item {
            DumpItem::CssClasses => self.dump_classes(acm),
            DumpItem::WebpackModules => self.dump_webpack_modules(acm),
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
