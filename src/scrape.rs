use std::io;
use std::rc::Rc;

use isahc::prelude::*;
use regex::Regex;
use thiserror::Error;
use url::Url;

use crate::discord;

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("http client error")]
    HttpError(#[from] isahc::error::Error),

    #[error("encountered malformed utf-8 string")]
    DecodingError(#[source] io::Error),

    #[error("asset error: {0}")]
    AssetError(&'static str),
}

pub(crate) fn get_text(url: Url) -> Result<String, ScrapeError> {
    log::debug!("GET {}", url.as_str());
    // TODO: use custom headers here
    let mut response = isahc::get(url.as_str())?;
    response.text().map_err(ScrapeError::DecodingError)
}

/// Scrapes a `[discord::FeManifest]`.
pub fn scrape_fe_manifest(branch: discord::Branch) -> Result<discord::FeManifest, ScrapeError> {
    let html = fetch_branch_page(branch)?;
    let assets = extract_assets_from_tags(&html);

    if assets.is_empty() {
        return Err(ScrapeError::AssetError("no assets were found"));
    }

    let count_assets_of_type = |typ| assets.iter().filter(|asset| asset.typ == typ).count();

    // Enforce some useful variants.
    if assets.is_empty() {
        return Err(ScrapeError::AssetError("failed to scrape any assets"));
    }
    if count_assets_of_type(discord::FeAssetType::Js) < 1 {
        return Err(ScrapeError::AssetError(
            "failed to scrape at least 1 js asset",
        ));
    }
    if count_assets_of_type(discord::FeAssetType::Css) < 1 {
        return Err(ScrapeError::AssetError(
            "failed to scrape at least 1 css asset",
        ));
    }

    Ok(discord::FeManifest {
        branch,
        assets: assets.into_iter().map(Rc::new).collect(),
    })
}

/// Gleans a [`discord::FeBuild`] from a [`discord::FeManifest`].
pub fn glean_frontend_build(
    fe_manifest: discord::FeManifest,
    asset_content_map: &crate::wrecker::AssetContentMap,
) -> Result<discord::FeBuild, ScrapeError> {
    // Right now, the scripts tags appear within in the page content in this
    // specific order:
    //
    // #0: chunk loader (webpack)
    // #1: CSS classnames
    // #2: vendor (??)
    // #3: main
    //
    // We can't depend on this ordering forever, so in the future we should
    // attempt to fetch and scan other scripts for build information based on
    // some heuristic, instead of just assuming that the last one has it.
    //
    // Here we extract static build information from the main bundle, relying
    // on the aforementioned assumptions.

    let last_script_asset = fe_manifest
        .assets
        .iter()
        .filter(|asset| asset.typ == discord::FeAssetType::Js)
        .last()
        .unwrap();

    let (hash, number) =
        match_static_build_information(&asset_content_map.get(last_script_asset).unwrap())?;

    Ok(discord::FeBuild {
        manifest: fe_manifest,
        hash,
        number,
    })
}

/// Extracts static build information from the main bundle's JavaScript.
pub fn match_static_build_information(js: &str) -> Result<(String, u32), ScrapeError> {
    lazy_static::lazy_static! {
        static ref BUILD_INFO_RE: Regex = Regex::new(r#"Build Number: (?P<number>\d+), Version Hash: (?P<hash>[0-9a-f]+)"#).unwrap();
    }

    let caps = BUILD_INFO_RE.captures(&js).ok_or(ScrapeError::AssetError(
        "failed to match static build information from main js bundle",
    ))?;

    Ok((caps["hash"].to_owned(), caps["number"].parse().unwrap()))
}

/// Fetches the main application page for a branch.
///
/// This uses the default Isahc client.
pub fn fetch_branch_page(branch: discord::Branch) -> Result<String, ScrapeError> {
    let url = branch.base().join("channels/@me").unwrap();
    get_text(url)
}

/// Extracts [`discord::FeAsset`]s from `<script>` and `<link>` tags on an HTML
/// page.
///
/// This function is designed to be used on the HTML content of `/channels/@me`
/// pages. Currently, crude regex matching is used instead of proper parsing.
pub fn extract_assets_from_tags(page_content: &str) -> Vec<discord::FeAsset> {
    lazy_static::lazy_static! {
        static ref SCRIPT_TAG_RE: Regex = Regex::new(r#"<script src="/assets/(?P<name>[\.0-9a-f]+)\.js" integrity="[^"]+"></script>"#).unwrap();
        static ref STYLE_TAG_RE: Regex = Regex::new(r#"<link rel="stylesheet" href="/assets/(?P<name>[\.0-9a-f]+)\.css" integrity="[^"]+">"#).unwrap();
    }

    let collect_assets = |regex: &Regex, typ: discord::FeAssetType| {
        regex
            .captures_iter(&page_content)
            .map(|caps| discord::FeAsset {
                name: caps["name"].to_owned(),
                typ,
            })
            .collect::<Vec<_>>()
    };

    let mut assets = collect_assets(&SCRIPT_TAG_RE, discord::FeAssetType::Js);
    assets.append(&mut collect_assets(
        &STYLE_TAG_RE,
        discord::FeAssetType::Css,
    ));

    assets
}

/// A scrape target.
pub enum Target {
    Frontend(discord::Branch),
}

impl std::str::FromStr for Target {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let index = s.find(':').ok_or("missing colon")?;
        let (source, variant) = s.split_at(index);
        let variant: String = variant.chars().skip(1).collect();

        match source {
            "fe" => Ok(Target::Frontend(
                variant
                    .parse::<discord::Branch>()
                    .map_err(|_| "invalid branch")
                    .and_then(|branch| {
                        if !branch.has_frontend() {
                            Err("branch has no frontend")
                        } else {
                            Ok(branch)
                        }
                    })?,
            )),
            _ => Err("unknown source"),
        }
    }
}
