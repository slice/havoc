use anyhow::{anyhow, Context, Result};

use crate::discord::{FeAsset, FeBuild, FeManifest};
use crate::scrape::Target;
use crate::util::measure;

use std::collections::HashMap;
use std::rc::Rc;

pub type AssetContentMap = HashMap<Rc<FeAsset>, String>;

pub struct Wrecker {
    pub manifest: Rc<FeManifest>,
    // NOTE(slice): doesn't seem ideal
    pub build: Option<FeBuild>,
    asset_content: AssetContentMap,
}

impl Wrecker {
    pub fn scrape(target: Target) -> Result<Self> {
        let Target::Frontend(branch) = target;
        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        Ok(Self {
            manifest: Rc::new(manifest),
            build: None,
            asset_content: HashMap::new(),
        })
    }

    pub fn glean_fe(&mut self) -> Result<()> {
        self.build = Some(
            crate::scrape::glean_frontend_build(Rc::clone(&self.manifest), &self.asset_content)
                .context("failed to glean frontend build")?,
        );

        Ok(())
    }

    pub fn fetch_assets(&mut self) -> Result<()> {
        for asset in &self.manifest.assets {
            let content = measure(&format!("fetching {}", asset.url()), || {
                crate::scrape::get_text(asset.url())
            })
            .with_context(|| format!("failed to prefetch {}", asset.url()))?;

            self.asset_content.insert(Rc::clone(&asset), content);
        }

        Ok(())
    }

    pub fn dump_classes(&self) -> Result<()> {
        let asset = &self
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
                self.manifest.branch,
                self.build.as_ref().unwrap().number
            ),
            serialized,
        )?;

        Ok(())
    }
}
