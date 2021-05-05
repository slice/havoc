use crate::discord::{FeAsset, FeBuild, FeManifest};
use crate::scrape::{ScrapeError, Target};
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
    pub fn scrape(target: Target) -> Result<Self, ScrapeError> {
        let Target::Frontend(branch) = target;
        let manifest = crate::scrape::scrape_fe_manifest(branch)?;

        Ok(Self {
            manifest: Rc::new(manifest),
            build: None,
            asset_content: HashMap::new(),
        })
    }

    pub fn glean_fe(&mut self) -> Result<(), ScrapeError> {
        self.build = Some(crate::scrape::glean_frontend_build(
            Rc::clone(&self.manifest),
            &self.asset_content,
        )?);
        Ok(())
    }

    pub fn fetch_assets(&mut self) -> Result<(), ScrapeError> {
        for asset in &self.manifest.assets {
            let content = measure(&format!("fetching {}", asset.url()), || {
                crate::scrape::get_text(asset.url())
            })?;
            self.asset_content.insert(Rc::clone(&asset), content);
        }

        Ok(())
    }
}
