use anyhow::{anyhow, Context, Result};

use crate::discord::{FeAsset, FeBuild, FeManifest};
use crate::scrape::Target;
use crate::util::measure;

use std::collections::HashMap;
use std::rc::Rc;

pub type AssetContentMap = HashMap<Rc<FeAsset>, String>;

pub struct Wrecker<I> {
    pub item: I,
    asset_content: AssetContentMap,
}

impl<I> Wrecker<I> {
    pub fn scrape(target: Target) -> Result<Wrecker<FeManifest>> {
        let Target::Frontend(branch) = target;
        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        Ok(Wrecker {
            item: manifest,
            asset_content: HashMap::new(),
        })
    }
}

impl Wrecker<FeManifest> {
    pub fn glean_fe(self) -> Result<Wrecker<FeBuild>> {
        let build = crate::scrape::glean_frontend_build(self.item, &self.asset_content)
            .context("failed to glean frontend build")?;

        Ok(Wrecker {
            item: build,
            asset_content: self.asset_content,
        })
    }

    pub fn fetch_assets(&mut self) -> Result<()> {
        for asset in &self.item.assets {
            let content = measure(&format!("fetching {}", asset.url()), || {
                crate::scrape::get_text(asset.url())
            })
            .with_context(|| format!("failed to prefetch {}", asset.url()))?;

            self.asset_content.insert(Rc::clone(&asset), content);
        }

        Ok(())
    }
}

impl Wrecker<FeBuild> {
    pub fn dump_classes(&self) -> Result<()> {
        let asset = &self
            .item
            .manifest
            .assets
            .get(1)
            .ok_or(anyhow!("no classes asset"))?;
        let js = self
            .asset_content
            .get(*asset)
            .ok_or(anyhow!("couldn't find classes js"))?;
        let script = crate::parse::parse_script(&js).context("failed to parse classes js")?;
        let mapping = crate::parse::walk_classes_chunk(&script)
            .map_err(|_| anyhow!("failed to walk classes js"))?;
        let serialized =
            serde_json::to_string(&mapping).context("failed to serialize classes mapping")?;

        std::fs::write(
            &format!(
                "{:?}_{}_class_mappings.json",
                self.item.manifest.branch, self.item.number
            ),
            serialized,
        )?;

        Ok(())
    }

    pub fn parse_chunks(&self) -> Result<(swc_ecma_ast::Script, crate::parse::WebpackChunk)> {
        let assets = &self.item.manifest.assets;

        let last_script = assets
            .iter()
            .filter(|asset| asset.typ == crate::discord::FeAssetType::Js)
            .last()
            .ok_or(anyhow!("couldn't find entrypoint js"))?;

        let entrypoint_js = self
            .asset_content
            .get(last_script)
            .ok_or(anyhow!("no entrypoint js content"))?;

        let script = measure("parsing entrypoint script", || {
            crate::parse::parse_script(&entrypoint_js)
        })
        .context("failed to parse entrypoint js")?;
        let chunk =
            crate::parse::walk_webpack_chunk(&script).context("failed to walk entrypoint js")?;

        Ok((script, chunk))
    }
}
