use anyhow::Result;
use chrono::Utc;
use havoc::discord::{self, FeAsset, FeAssetType};
use isahc::{AsyncReadResponseExt, Request, RequestExt};

use crate::subscription::Subscription;

#[tracing::instrument(skip_all, fields(%build.manifest.branch, %build.number, ?subscription))]
pub async fn post_build_to_webhook(
    build: &discord::FeBuild,
    subscription: &Subscription,
) -> Result<()> {
    use serde_json::json;

    let assets = &build.manifest.assets;

    let format_asset =
        |asset: &FeAsset| format!("[`{}.{}`]({})", asset.name, asset.typ.ext(), asset.url());

    let scripts = assets
        .filter_by_type(FeAssetType::Js)
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
        .filter_by_type(FeAssetType::Css)
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
        "description": format!("Hash: `{}`", build.manifest.hash),
        "fields": [
            {"name": "Scripts", "value": scripts_listing, "inline": false},
            {"name": "Styles", "value": styles_listing, "inline": false},
        ],
        "footer": {"text": format!("Pacific: {}", pacific_time)},
        "timestamp": utc_timestamp
    });

    let payload = json!({ "username": "watchdog", "embeds": [embed] });

    tracing::debug!(?payload, "webhook payload");

    let mut response = Request::post(&subscription.discord_webhook_url)
        .header("content-type", "application/json")
        .header(
            "user-agent",
            "watchdog/0.0 (https://github.com/slice/havoc)",
        )
        .body(serde_json::to_vec(&payload)?)?
        .send_async()
        .await?;

    tracing::info!("received {} from discord", response.status());

    let body_string = response.text().await?;
    tracing::info!("discord response body: {}", body_string);

    Ok(())
}
