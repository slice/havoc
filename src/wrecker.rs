use anyhow::{Context, Result};

use std::collections::HashMap;
use std::rc::Rc;

use crate::artifact::{Artifact, AssetContentMap, DumpItem, DumpResult};
use crate::discord::FeAsset;
use crate::scrape::Target;
use crate::util::measure;

pub struct Wrecker {
    asset_content: AssetContentMap,
    pub artifact: Box<dyn Artifact>,
}

impl Wrecker {
    pub fn scrape_fe_manifest(target: Target) -> Result<Wrecker> {
        let Target::Frontend(branch) = target;

        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        Ok(Wrecker {
            artifact: Box::new(manifest),
            asset_content: HashMap::new(),
        })
    }

    pub fn scrape_fe_build(target: Target) -> Result<Wrecker> {
        let Target::Frontend(branch) = target;

        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        let acm = fetch_assets(&manifest.assets)?;

        let build = crate::scrape::glean_frontend_build(manifest, &acm)
            .context("failed to glean frontend build")?;

        Ok(Wrecker {
            artifact: Box::new(build),
            asset_content: acm,
        })
    }

    pub fn fetch_assets(&mut self) -> Result<()> {
        self.asset_content = fetch_assets(&self.artifact.assets())?;
        Ok(())
    }

    pub fn dump(&self, dump_item: DumpItem) -> Result<Vec<DumpResult>> {
        self.artifact
            .dump(dump_item, &self.asset_content)
            .map_err(|err| anyhow::anyhow!(err))
    }
}

pub fn fetch_assets(assets: &[Rc<FeAsset>]) -> Result<AssetContentMap> {
    let mut map = HashMap::new();

    for asset in assets {
        let content = measure(&format!("fetching {}", asset.url()), || {
            crate::scrape::get_text(asset.url())
        })
        .with_context(|| format!("failed to prefetch {}", asset.url()))?;

        map.insert(Rc::clone(&asset), content);
    }

    Ok(map)
}
