use std::collections::hash_map::Entry;
use std::collections::HashMap;

use anyhow::{Context, Result};
use disruption::config::Config;
use disruption::subscription::Subscription;
use havoc::discord::{Assets, Branch};
use havoc::scrape;

type State = HashMap<Branch, u32>;

fn run(config: Config) -> Result<()> {
    // Tracks the last known build numbers for each branch.
    let mut state: State = HashMap::new();

    if let Ok(state_file_text) = std::fs::read_to_string(&config.state_file_path) {
        tracing::info!(path = ?config.state_file_path, "using state file");
        state = serde_json::from_str(&state_file_text).context("failed to decode state file")?;
        tracing::info!(?state, "loaded state");
    } else {
        tracing::info!("cannot load state file, beginning with empty state");
    }

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
            let _enter = scrape_span.enter();

            let build = match scrape::scrape_fe_manifest(branch).and_then(|manifest| {
                let mut assets = Assets::with_assets(manifest.assets.clone());
                scrape::scrape_fe_build(manifest, &mut assets)
            }) {
                Ok(build) => build,
                Err(err) => {
                    tracing::error!(?branch, "failed to scrape {:?}", err);
                    continue;
                }
            };

            let publish = || -> Result<()> {
                for subscription in subscriptions {
                    disruption::webhook::post_build_to_webhook(&build, *subscription)?;
                }
                Ok(())
            };

            match state.entry(branch) {
                Entry::Occupied(state_entry) if *state_entry.get() == build.number => {
                    tracing::trace!("{} is stale", branch);
                }
                _ => {
                    tracing::info!(
                        "detected new build (branch: {}, number: {})",
                        branch,
                        build.number,
                    );
                    state.insert(branch, build.number);
                    publish().context("failed to publish")?;

                    tracing::debug!(path = ?config.state_file_path, "writing to state file");
                    std::fs::write(&config.state_file_path, serde_json::to_string(&state)?)
                        .context("failed to write to state file")?;
                }
            }
        }

        tracing::trace!("sleeping for {}ms", config.interval_milliseconds);
        let duration = std::time::Duration::from_millis(config.interval_milliseconds);
        std::thread::sleep(duration);
    }
}

fn main() -> Result<()> {
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

    run(config)
}
