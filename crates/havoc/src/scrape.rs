use std::io;
use std::str::Utf8Error;

use isahc::AsyncReadResponseExt;
use regex::Regex;
use thiserror::Error;
use url::Url;

use crate::discord::{self, Assets, RootScript};

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("http error")]
    Http(#[from] isahc::error::Error),

    #[error("malformed utf-8")]
    Decoding(#[source] Utf8Error),

    #[error("failed to read http response")]
    ReadingHttpResponse(io::Error),

    #[error("branch page is missing assets: {0}")]
    MissingBranchPageAssets(&'static str),

    #[error("cannot find static build information in entrypoint script")]
    MissingBuildInformation,
}

pub(crate) async fn fetch_url_content(url: Url) -> Result<Vec<u8>, ScrapeError> {
    tracing::info!("GET {}", url.as_str());

    // TODO: use custom headers here
    let mut response = isahc::get_async(url.as_str()).await?;
    response
        .bytes()
        .await
        .map_err(ScrapeError::ReadingHttpResponse)
}

/// Scrapes a [`discord::FeManifest`] for a specific [`discord::Branch`].
pub async fn scrape_fe_manifest(
    branch: discord::Branch,
) -> Result<discord::FeManifest, ScrapeError> {
    let html = fetch_branch_page(branch).await?;
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

    Ok(discord::FeManifest {
        branch,
        assets: assets.into_iter().collect(),
    })
}

/// Scrapes a [`discord::FeBuild`] from a [`discord::FeManifest`].
///
/// Builds contain a superset of the information encapsulated within manifests.
pub async fn scrape_fe_build(
    fe_manifest: discord::FeManifest,
    assets: &mut Assets,
) -> Result<discord::FeBuild, ScrapeError> {
    // locate the entrypoint script, which contains the build information we're
    // interested in.
    let entrypoint_asset = assets.find_root_script(RootScript::Entrypoint).expect(
        "unable to locate entrypoint root script; discord has updated their /channels/@me html",
    );

    let content = assets.content(&entrypoint_asset).await?;
    let entrypoint_js = std::str::from_utf8(content).map_err(ScrapeError::Decoding)?;

    let (hash, number) = match_static_build_information(entrypoint_js)?;

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
        static ref BUILD_INFO_SWC_RE: Regex = Regex::new(
            r#"Build Number: "\).concat\("(?P<number>\d+)",", Version Hash: "\).concat\("(?P<hash>[0-9a-f]+)"\)"#
        ).unwrap();
    }

    let caps = BUILD_INFO_SWC_RE
        .captures(js)
        .or_else(|| BUILD_INFO_RE.captures(js))
        .ok_or(ScrapeError::MissingBuildInformation)?;

    Ok((caps["hash"].to_owned(), caps["number"].parse().unwrap()))
}

/// Fetches the main application page for a branch.
///
/// This uses the default Isahc client.
pub async fn fetch_branch_page(branch: discord::Branch) -> Result<String, ScrapeError> {
    let url = branch.base().join("channels/@me").unwrap();

    String::from_utf8(fetch_url_content(url).await?)
        .map_err(|err| ScrapeError::Decoding(err.utf8_error()))
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
