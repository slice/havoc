use anyhow::Result;
use chrono::Utc;
use havoc::discord::{self, AssetsExt, FeAsset, FeAssetType};
use isahc::{AsyncReadResponseExt, Request, RequestExt};

use crate::subscription::Subscription;

#[tracing::instrument(skip_all, fields(%build.manifest.branch, %build.number, ?subscription))]
pub async fn post_build_to_webhook(
    build: &discord::FeBuild,
    subscription: &Subscription,
) -> Result<()> {
    use serde_json::json;

    let utc_timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let pacific_time = Utc::now()
        .with_timezone(&chrono_tz::America::Los_Angeles)
        .format("%a %b %-d, %-H:%M");

    let embed = json!({
        "title": format!("{} {}", build.manifest.branch, build.number),
        "color": build.manifest.branch.color(),
        "description": format!("Hash: `{}`", build.manifest.hash),
        "footer": {"text": format!("Pacific Time: {}", pacific_time)},
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
