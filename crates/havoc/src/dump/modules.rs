//! Webpack module dumping.

use std::collections::HashMap;

use crate::{
    artifact::Artifact,
    discord::{AssetCache, Assets, RootScript},
    dump::{Dump, DumpError},
    parse::ModuleId,
    scrape::ScrapeError,
};

use super::DumpResult;

pub struct WebpackModules;

async fn parse_webpack_chunk<'cache>(
    assets: &'_ Assets,
    cache: &'cache mut AssetCache,
) -> Result<(swc_ecma_ast::Script, HashMap<ModuleId, &'cache str>), DumpError> {
    let entrypoint_asset = assets.find_root_script(RootScript::Entrypoint).ok_or(
        ScrapeError::MissingBranchPageAssets(
            "failed to locate root entrypoint script; discord has updated their HTML",
        ),
    )?;

    let content = cache
        .preprocessed_content(&entrypoint_asset)
        .await?
        .map_err(DumpError::Preprocessing)?;
    let entrypoint_js = std::str::from_utf8(content).map_err(ScrapeError::Decoding)?;

    tracing::info!("parsing entrypoint script");
    let script = crate::parse::parse_script(entrypoint_js.to_owned())?;

    let chunk = crate::parse::walk_webpack_chunk(&script)?;

    let modules: HashMap<ModuleId, &str> = chunk
        .modules
        .iter()
        .map(|(module_id, module)| {
            let span = module.func.span();

            // swc's spans seem to start at one.
            let module_beginning = span.lo.0 as usize - 1;
            let module_end = span.hi.0 as usize - 1;
            (*module_id, &entrypoint_js[module_beginning..module_end])
        })
        .collect();

    Ok((script, modules))
}

#[async_trait::async_trait]
impl Dump for WebpackModules {
    async fn dump(
        &mut self,
        artifact: &(dyn Artifact + Sync),
        cache: &mut AssetCache,
    ) -> Result<DumpResult, DumpError> {
        let (_, modules) = parse_webpack_chunk(artifact.assets(), cache).await?;
        Ok(DumpResult::from_serializable(
            &modules,
            "entrypoint_modules",
        )?)
    }
}
