use std::io;
use std::str::Utf8Error;
use std::time::Duration;

use isahc::prelude::Configurable;
use isahc::{AsyncReadResponseExt, RequestExt};
use regex::Regex;
use thiserror::Error;
use url::Url;

use crate::discord::{self, AssetCache, RootScript};

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("network error")]
    Network(#[from] NetworkError),

    #[error("malformed utf-8")]
    Decoding(#[source] Utf8Error),

    #[error("branch page is missing assets: {0}")]
    MissingBranchPageAssets(&'static str),

    #[error("missing static build information")]
    MissingStaticBuildInformation,

    #[error("missing networked build information")]
    MissingNetworkBuildInformation,
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("isahc error")]
    Isahc(#[from] isahc::error::Error),

    #[error("http error")]
    Http(#[from] http::Error),

    #[error("encountered malformed HTTP header")]
    MalforedHeader,

    #[error("failed to perform i/o")]
    Io(#[from] io::Error),
}

type IsahcResponse = http::Response<isahc::AsyncBody>;

pub(crate) async fn get_async(url: Url) -> Result<IsahcResponse, NetworkError> {
    tracing::info!("GET {}", url.as_str());

    let response = isahc::Request::get(url.as_str())
        .timeout(Duration::from_secs(10))
        .body(())?
        .send_async()
        .await?;

    Ok(response)
}

/// Scrapes a [`discord::FeManifest`] for a specific [`discord::Branch`].
pub async fn scrape_fe_manifest(
    branch: discord::Branch,
) -> Result<discord::FeManifest, ScrapeError> {
    let mut response = request_branch_page(branch).await?;
    let html = response.text().await.map_err(NetworkError::Io)?;

    let assets = extract_assets_from_tags(&html);

    use ScrapeError::MissingBranchPageAssets;

    if assets.is_empty() {
        return Err(MissingBranchPageAssets("no assets were found whatsoever"));
    }

    let count_assets_of_type = |typ| assets.iter().filter(|asset| asset.typ == typ).count();

    // Enforce some useful invariants.
    if count_assets_of_type(discord::FeAssetType::Js) < 1 {
        return Err(MissingBranchPageAssets("couldn't find at least one script"));
    }
    if count_assets_of_type(discord::FeAssetType::Css) < 1 {
        return Err(MissingBranchPageAssets(
            "couldn't find at least one stylesheet",
        ));
    }

    let hash = response
        .headers()
        .get("x-build-id")
        .ok_or(ScrapeError::MissingNetworkBuildInformation)?
        .to_str()
        .map_err(|_| NetworkError::MalforedHeader)?;

    Ok(discord::FeManifest {
        branch,
        hash: hash.to_owned(),
        assets: assets.into(),
    })
}

/// Scrapes a [`discord::FeBuild`] from a [`discord::FeManifest`].
///
/// Builds contain a superset of the information encapsulated within manifests.
pub async fn scrape_fe_build(
    fe_manifest: discord::FeManifest,
    cache: &mut AssetCache,
) -> Result<discord::FeBuild, ScrapeError> {
    // locate the entrypoint script, which contains the build information we're
    // interested in.
    let entrypoint_asset = fe_manifest
        .assets
        .find_root_script(RootScript::Entrypoint)
        .expect(
            "unable to locate entrypoint root script; discord has updated their /channels/@me html",
        );

    let content = cache.raw_content(&entrypoint_asset).await?;
    let entrypoint_js = std::str::from_utf8(content).map_err(ScrapeError::Decoding)?;
    let (_, number) = match_static_build_information(entrypoint_js)?;

    Ok(discord::FeBuild {
        manifest: fe_manifest,
        number,
    })
}

/// Extracts static build information from the main bundle's JavaScript.
pub fn match_static_build_information(js: &str) -> Result<(String, u32), ScrapeError> {
    lazy_static::lazy_static! {
        static ref BUILD_INFO_RE: Regex = Regex::new(r#"Build Number: (?P<number>\d+), Version Hash: (?P<hash>[0-9a-f]+)"#).unwrap();
        static ref BUILD_INFO_SWC_RE: Regex = Regex::new(
            r#"Build Number: "\).concat\("(?P<number>\d+)",", Version Hash: "\).concat\("(?P<hash>[0-9a-f]+)"\)"#
        ).unwrap();
    }

    let caps = BUILD_INFO_SWC_RE
        .captures(js)
        .or_else(|| BUILD_INFO_RE.captures(js))
        .ok_or(ScrapeError::MissingStaticBuildInformation)?;

    Ok((caps["hash"].to_owned(), caps["number"].parse().unwrap()))
}

/// Request the main application page for the branch.
///
/// This makes an HTTP request to `/channels/@me` with the default Isahc client.
pub async fn request_branch_page(branch: discord::Branch) -> Result<IsahcResponse, NetworkError> {
    get_async(branch.base().join("channels/@me").unwrap()).await
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
            .captures_iter(page_content)
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
