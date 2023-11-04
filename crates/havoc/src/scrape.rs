use std::io;
use std::str::Utf8Error;
use std::time::Duration;

use isahc::prelude::Configurable;
use isahc::{AsyncReadResponseExt, RequestExt};
use regex::Regex;
use thiserror::Error;
use url::Url;

use crate::discord::{self, AssetCache, AssetsExt, FeAsset, FeAssetType, RootScript};
use crate::parse::ChunkId;

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("network error")]
    Network(#[from] NetworkError),

    #[error("malformed utf-8")]
    Decoding(#[from] Utf8Error),

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
        assets,
    })
}

/// Identifies script and stylesheet chunks present in the chunkloader.
pub async fn extract_assets_from_chunk_loader(
    manifest: &discord::FeManifest,
    cache: &mut AssetCache,
) -> Result<Vec<(ChunkId, FeAsset)>, ScrapeError> {
    let chunk_loader = (&manifest.assets)
        .find_root_script(RootScript::ChunkLoader)
        .ok_or(ScrapeError::MissingBranchPageAssets("chunk loader"))?;
    let data = cache.raw_content(chunk_loader).await?;
    let text = std::str::from_utf8(data)?;

    // The chunk loader is bisected into two sections that handle scripts and
    // stylesheets accordingly.
    //
    // Whatever Discord is using to process their stylesheets emits a ton of
    // garbage hashes that don't correspond to actual assets, so the second
    // section is mostly useless data. There's only one real stylesheet that is
    // already present as a <link> tag in the HTML.
    //
    // Here, split the chunk loader via an arbitrary landmark.
    let script_section = text
        .split_once(r#"+".js""#)
        .map(|(script_section, _)| script_section)
        .ok_or(ScrapeError::MissingStaticBuildInformation)?;

    lazy_static::lazy_static! {
        static ref HASH_REGEX: Regex = Regex::new(r#"(?P<chunk_id>\d+):"(?P<name>[a-f0-9]{20})""#).unwrap();
    }

    let assets = HASH_REGEX
        .captures_iter(script_section)
        .map(|captures| {
            (
                captures["chunk_id"]
                    .parse::<ChunkId>()
                    .expect("chunk ID can't fit into u32"),
                FeAsset {
                    name: captures["name"].to_owned(),
                    typ: FeAssetType::Js,
                },
            )
        })
        .collect::<Vec<_>>();

    Ok(assets)
}

/// Scrapes a [`discord::FeBuild`] from a [`discord::FeManifest`].
///
/// Builds contain a superset of the information encapsulated within manifests.
pub async fn scrape_fe_build(
    fe_manifest: discord::FeManifest,
    cache: &mut AssetCache,
) -> Result<discord::FeBuild, ScrapeError> {
    let scripts = fe_manifest
        .assets
        .filter_by_type(FeAssetType::Js)
        .collect::<Box<[&FeAsset]>>(); // probably better than a vec since we don't need to resize

    // FIXME: We can't rely on the static build information always being
    // available in the penultimate script. In the future, maybe we can scan
    // all surface scripts for it.
    let penultimate_surface_script = scripts
        .iter()
        .nth_back(1)
        .expect("failed to penultimate surface script");

    let content = cache.raw_content(penultimate_surface_script).await?;
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
/// pages. Currently, imprecise regex matching is used instead of parsing.
pub fn extract_assets_from_tags(page_content: &str) -> Vec<discord::FeAsset> {
    lazy_static::lazy_static! {
        // Crude matches, but I don't feel like bringing in a proper HTML parser
        // unless I need to.
        static ref SCRIPT_TAG_RE: Regex = Regex::new(r#"<script src="/assets/(?P<name>[\.0-9a-z]+)\.js""#).unwrap();
        static ref STYLE_TAG_RE: Regex = Regex::new(r#"<link href="/assets/(?P<name>[\.0-9a-z]+)\.css""#).unwrap();
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
#[derive(Clone, Debug)]
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
