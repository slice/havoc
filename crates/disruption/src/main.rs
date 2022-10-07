use std::collections::HashMap;

use anyhow::{Context, Result};
use disruption::config::Config;
use disruption::db::Db;
use disruption::subscription::Subscription;
use havoc::discord::{Assets, Branch};
use havoc::scrape;

async fn detect_changes_on_branch(
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
            disruption::webhook::post_build_to_webhook(&build, *subscription)?;
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

async fn run(config: Config) -> Result<()> {
    let conn = rusqlite::Connection::open(config.database_file_path)?;
    let db = Db::new(conn);

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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config_file_path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: disruption <path/to/config.toml>");
            std::process::exit(1);
        }
    };

    let config_file_text =
        std::fs::read_to_string(config_file_path).context("cannot read config file")?;
    let config: Config = toml::from_str(&config_file_text).context("cannot parse config file")?;

    run(config).await
}
