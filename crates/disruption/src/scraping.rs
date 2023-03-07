use std::collections::HashMap;

use anyhow::{Context, Result};
use havoc::discord::{Assets, Branch};

use crate::{config::Config, db::Db, subscription::Subscription};
use havoc::scrape;

pub async fn detect_changes_on_branch(
    db: &Db,
    branch: Branch,
    subscriptions: &[&Subscription],
) -> Result<()> {
    let scrape_span = tracing::info_span!("scrape", ?branch);
    let _enter = scrape_span.enter();

    let manifest = scrape::scrape_fe_manifest(branch).await?;
    let mut assets = Assets::with_assets(manifest.assets.clone());

    if db.last_known_build_hash_on_branch(branch).await? == Some(manifest.hash.clone()) {
        tracing::trace!("{} is stale", branch);
        return Ok(());
    }

    let build = scrape::scrape_fe_build(manifest, &mut assets).await?;
    let publish = || -> Result<()> {
        for subscription in subscriptions {
            crate::webhook::post_build_to_webhook(&build, *subscription)?;
        }
        Ok(())
    };

    tracing::info!(
        "detected new build (branch: {}, number: {})",
        branch,
        build.number,
    );

    db.detected_build_change_on_branch(&build, branch).await?;

    publish().context("failed to publish")?;

    Ok(())
}

pub async fn scrape_indefinitely(config: &Config, db: Db) -> Result<()> {
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
            detect_changes_on_branch(&db, branch, subscriptions).await?;
        }

        tracing::trace!("sleeping for {}ms", config.interval_milliseconds);
        let duration = std::time::Duration::from_millis(config.interval_milliseconds);
        std::thread::sleep(duration);
    }
}
