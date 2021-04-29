use std::io;

use isahc::prelude::*;
use regex::Regex;
use thiserror::Error;
use url::Url;

use crate::discord;

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("http client error")]
    HttpError(#[from] isahc::error::Error),

    #[error("failed to decode string")]
    DecodingError(#[source] io::Error),

    #[error("asset error: {0}")]
    AssetError(&'static str),
}

fn get_text(url: Url) -> Result<String, ScrapeError> {
    // TODO: use custom headers here
    let mut response = isahc::get(url.as_str())?;
    Ok(response.text().map_err(ScrapeError::DecodingError)?)
}

/// Scrapes a frontend build.
pub fn scrape_fe(branch: discord::Branch) -> Result<discord::FeBuild, ScrapeError> {
    let html = fetch_branch_page(branch)?;
    let assets = extract_assets_from_tags(&html);

    if assets.is_empty() {
        return Err(ScrapeError::AssetError("no assets were found"));
    }

    let assets_of_type = |typ| assets.iter().filter(move |asset| asset.typ == typ);
    let count_assets_of_type = |typ| assets_of_type(typ).count();

    if count_assets_of_type(discord::FeAssetType::Js) < 1 {
        return Err(ScrapeError::AssetError(
            "failed to extract at least 1 js asset",
        ));
    }
    if count_assets_of_type(discord::FeAssetType::Css) < 1 {
        return Err(ScrapeError::AssetError(
            "failed to extract at least 1 css asset",
        ));
    }

    let scripts = assets_of_type(discord::FeAssetType::Js);
    let (hash, number) = discover_fe_build_info(scripts)?;

    Ok(discord::FeBuild {
        branch,
        hash,
        number,
        assets,
    })
}

/// Discover static build information from a sequence of `Js` assets.
///
/// This will make HTTP requests as necessary.
pub fn discover_fe_build_info<'a>(
    scripts: impl IntoIterator<Item = &'a discord::FeAsset>,
) -> Result<(String, u32), ScrapeError> {
    let scripts: Vec<_> = scripts.into_iter().collect();

    if scripts.is_empty() {
        panic!("can't discover build info from no scripts");
    }

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

    let main_bundle = scripts.last().unwrap();

    let text = get_text(main_bundle.url())?;

    lazy_static::lazy_static! {
        static ref BUILD_INFO_RE: Regex = Regex::new(r#"Build Number: (?P<number>\d+), Version Hash: (?P<hash>[0-9a-f]+)"#).unwrap();
    }

    let caps = BUILD_INFO_RE
        .captures(&text)
        .ok_or(ScrapeError::AssetError(
            "failed to extract static build information from js bundle",
        ))?;

    Ok((caps["hash"].to_owned(), caps["number"].parse().unwrap()))
}

/// Fetches the main application page for a branch.
///
/// This uses the default Isahc client.
pub fn fetch_branch_page(branch: discord::Branch) -> Result<String, ScrapeError> {
    let url = branch.base().join("channels/@me").unwrap();
    Ok(get_text(url)?)
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
