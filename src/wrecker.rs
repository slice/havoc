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
    pub fn scrape(target: Target) -> Result<Wrecker<Rc<FeManifest>>> {
        let Target::Frontend(branch) = target;
        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        Ok(Wrecker {
            item: Rc::new(manifest),
            asset_content: HashMap::new(),
        })
    }
}

impl Wrecker<Rc<FeManifest>> {
    pub fn glean_fe(self) -> Result<Wrecker<FeBuild>> {
        let build = crate::scrape::glean_frontend_build(Rc::clone(&self.item), &self.asset_content)
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
        let mapping = crate::parse::parse_classes_file(js)
            .map_err(|_| anyhow!("failed to parse classes js"))?;
        let serialized =
            serde_json::to_string(&mapping).context("failed to serialize classes mapping")?;

        std::fs::write(
            &format!(
                "{:?}_{}_class_mappings.json",
                self.item.manifest.branch,
                self.item.number
            ),
            serialized,
        )?;

        Ok(())
    }
}
