use std::collections::HashMap;

use anyhow::{Context, Result};
use havoc::discord::{AssetCache, Branch};
use tracing::Instrument;

use crate::{config::Config, db::Db, subscription::Subscription};
use havoc::scrape;

pub async fn detect_changes_on_branch(
    db: &Db,
    branch: Branch,
    subscriptions: &[&Subscription],
) -> Result<()> {
    let manifest = scrape::scrape_fe_manifest(branch).await?;
    let mut cache = AssetCache::new();

    if db.last_known_build_hash_on_branch(branch).await? == Some(manifest.hash.clone()) {
        tracing::trace!("{} is stale", branch);
        return Ok(());
    }

    let build = scrape::scrape_fe_build(manifest, &mut cache).await?;

    tracing::info!(
        "detected new build (branch: {}, number: {})",
        branch,
        build.number,
    );

    db.detected_build_change_on_branch(&build, branch).await?;
    db.detected_assets(&build).await?;

    for subscription in subscriptions {
        crate::webhook::post_build_to_webhook(&build, subscription)
            .await
            .context("failed to publish")?;
    }

    Ok(())
}

pub async fn scrape_forever(config: &Config, db: &Db) -> Result<()> {
    // Go from [subscription] to {branch: [subscription]}.
    let mut branches: HashMap<Branch, Vec<&Subscription>> = HashMap::new();
    for subscription in &config.subscriptions {
        for branch in &subscription.branches {
            branches
                .entry(*branch)
                .or_insert_with(Vec::new)
                .push(subscription);
        }
    }

    tracing::info!(?branches, "scraping continuously");

    loop {
        for (&branch, subscriptions) in &branches {
            let scrape_span = tracing::info_span!("scrape", ?branch);
            detect_changes_on_branch(db, branch, subscriptions)
                .instrument(scrape_span)
                .await?;
        }

        tracing::trace!("sleeping for {}ms", config.interval_milliseconds);
        let duration = std::time::Duration::from_millis(config.interval_milliseconds);
        tokio::time::sleep(duration).await;
    }
}
