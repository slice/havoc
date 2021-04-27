use std::io;

use thiserror::Error;
use isahc::prelude::*;
use regex::Regex;

use crate::discord;

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("http client error")]
    /// An error from the HTTP client.
    HttpError(#[from] isahc::error::Error),

    /// An error caused by invalid string decoding.
    #[error("failed to decode string")]
    DecodingError(io::Error),
}

/// Scrapes a frontend build.
pub fn scrape_fe(branch: discord::Branch) -> Result<discord::FeBuild, ScrapeError> {
    let mut response = isahc::get("https://discord.com/channels/@me")?;
    let text = response.text().map_err(ScrapeError::DecodingError)?;

    lazy_static::lazy_static! {
        static ref SCRIPT_TAG_RE: Regex = Regex::new(r#"<script src="/assets/(?P<name>[\.0-9a-f]+)\.js" integrity="[^"]+"></script>"#).unwrap();
        static ref STYLE_TAG_RE: Regex = Regex::new(r#"<link rel="stylesheet" href="/assets/(?P<name>[\.0-9a-f]+)\.css" integrity="[^"]+">"#).unwrap();
    }

    let collect_assets = |regex: &Regex, typ: discord::FeAssetType| {
        regex.captures_iter(&text).map(|caps| discord::FeAsset {
            name: caps["name"].to_owned(),
            typ,
        }).collect::<Vec<_>>()
    };

    let mut scripts = collect_assets(&SCRIPT_TAG_RE, discord::FeAssetType::Js);
    let mut styles = collect_assets(&STYLE_TAG_RE, discord::FeAssetType::Css);
    scripts.append(&mut styles);

    Ok(discord::FeBuild {
        branch,
        hash: "".to_owned(),
        number: 0,
        assets: scripts,
    })
}
