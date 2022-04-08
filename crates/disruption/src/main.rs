use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result};
use chrono::Utc;
use havoc::discord::{Assets, Branch, FeAsset, FeAssetType};
use havoc::{discord, scrape};
use isahc::{Request, RequestExt};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    interval_milliseconds: u64,
    state_file_path: PathBuf,
    subscriptions: Vec<Subscription>,
}

#[derive(Deserialize, Debug, Clone)]
struct Subscription {
    branches: Vec<Branch>,
    discord_webhook_url: String,
}

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
                    publish_new_build(&build, *subscription)?;
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

fn publish_new_build(build: &discord::FeBuild, subscription: &Subscription) -> Result<()> {
    let publish_span =
        tracing::info_span!("publish", %build.number, %build.manifest.branch, ?subscription);
    let _enter = publish_span.enter();

    use serde_json::json;

    let assets = &build.manifest.assets;

    let format_asset =
        |asset: &Rc<FeAsset>| format!("[`{}.{}`]({})", asset.name, asset.typ.ext(), asset.url());

    let scripts = assets
        .iter()
        .filter(|asset| asset.typ == FeAssetType::Js)
        .map(format_asset)
        .collect::<Vec<_>>();

    let scripts_listing = if scripts.len() == 4 {
        scripts
            .iter()
            .zip(["chunk loader", "classes", "vendor", "entrypoint"])
            .map(|(formatted_link, label)| format!("{} ({})", formatted_link, label))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        scripts.join("\n")
    };

    let styles_listing = assets
        .iter()
        .filter(|asset| asset.typ == FeAssetType::Css)
        .map(format_asset)
        .collect::<Vec<_>>()
        .join("\n");

    let utc_timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let pacific_time = Utc::now()
        .with_timezone(&chrono_tz::America::Los_Angeles)
        .format("%b %-d, %-H:%M (%a)");

    let embed = json!({
        "title": format!("{} {}", build.manifest.branch, build.number),
        "color": build.manifest.branch.color(),
        "description": format!("Hash: `{}`", build.hash),
        "fields": [
            {"name": "Scripts", "value": scripts_listing, "inline": false},
            {"name": "Styles", "value": styles_listing, "inline": false},
        ],
        "footer": {"text": format!("Pacific: {}", pacific_time)},
        "timestamp": utc_timestamp
    });

    let payload = json!({ "username": "disruption", "embeds": [embed] });

    tracing::debug!(?payload, "webhook payload");

    let response = Request::post(&subscription.discord_webhook_url)
        .header("content-type", "application/json")
        .header(
            "user-agent",
            "disruption/0.0 (https://github.com/slice/havoc)",
        )
        .body(serde_json::to_vec(&payload)?)?
        .send()?;

    tracing::info!("received {} from discord", response.status());

    let mut body_string = String::new();
    let _ = response.into_body().read_to_string(&mut body_string)?;

    tracing::info!("discord response body: {}", body_string);

    Ok(())
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
