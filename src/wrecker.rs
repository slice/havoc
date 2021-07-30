use anyhow::{Context, Result};

use crate::artifact::{Artifact, DumpItem, DumpResult};
use crate::assets::Assets;
use crate::discord::Branch;

pub struct Wrecker {
    pub artifact: Box<dyn Artifact>,
    assets: Assets,
}

impl Wrecker {
    pub fn scrape_fe_manifest(branch: Branch) -> Result<Wrecker> {
        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        Ok(Wrecker {
            artifact: Box::new(manifest),
            assets: Assets::new(),
        })
    }

    pub fn scrape_fe_build(branch: Branch) -> Result<Wrecker> {
        let manifest = crate::scrape::scrape_fe_manifest(branch)
            .context("failed to scrape frontend manifest")?;

        let mut assets = Assets::with_assets(manifest.assets.clone());

        let build = crate::scrape::glean_frontend_build(manifest, &mut assets)
            .context("failed to glean frontend build")?;

        Ok(Wrecker {
            artifact: Box::new(build),
            assets,
        })
    }

    pub fn dump(&mut self, dump_item: DumpItem) -> Result<Vec<DumpResult>> {
        let dump_span = tracing::info_span!("dumping", ?dump_item);
        let _enter = dump_span.enter();

        self.artifact
            .dump(dump_item, &mut self.assets)
            .map_err(|err| anyhow::anyhow!(err))
    }
}
