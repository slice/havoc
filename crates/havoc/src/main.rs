use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Command};

use havoc::artifact::Artifact;
use havoc::discord::AssetCache;
use havoc::dump::Dump;
use havoc::scrape::{self, extract_assets_from_chunk_loader};
use tracing::Instrument;

fn app() -> clap::Command {
    clap::command!()
        .propagate_version(true)
        .subcommand_required(true)
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .arg(
            clap::arg!(-h --help "print help")
                .action(ArgAction::Help)
                .global(true),
        )
        .arg(clap::arg!(-V --version "print version").action(ArgAction::Version))
        .subcommand(
            Command::new("scrape")
                .about("scrape a target")
                .long_about(
                    "This subcommand scrapes from a target and outputs
human-readable information about it to standard output. Invoking dumpers on the
scraped artifact can also be done.",
                )
                .arg(
                    clap::arg!(-d --dump "what to dump from the target")
                        .required(false)
                        .long_help(
                            r#"the names of dumpers to invoke on the target
e.g. "modules", "classes""#,
                        )
                        .action(ArgAction::Append),
                )
                .arg(
                    clap::arg!(--deep "look for assets contained within assets")
                        .action(ArgAction::SetTrue)
                        .long_help(
                            "instructs havoc to look for assets that are
contained within other hashes (script chunks, artwork, etc.)",
                        ),
                )
                .arg(
                    clap::arg!(target: <TARGET> "what to scrape")
                        .value_parser(clap::value_parser!(scrape::Target))
                        .long_help(
                            r#"the scrape target, using target syntax
e.g. "fe:canary" to target latest canary"#,
                        ),
                )
                .after_help("invoke with --help for more information")
                .after_long_help(""),
        )
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = app();
    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("scrape") {
        let target = matches
            .get_one::<scrape::Target>("target")
            .expect("no scrape target specified");

        let scrape::Target::Frontend(branch) = target;

        let manifest = scrape::scrape_fe_manifest(*branch)
            .await
            .context("failed to scrape frontend manifest")?;

        let mut cache = AssetCache::new();
        let mut build = crate::scrape::scrape_fe_build(manifest, &mut cache)
            .await
            .context("failed to scrape frontend build")?;
        let assets = &build.manifest.assets;

        println!("{}", build);

        println!("surface assets ({}):", assets.len());

        for asset in assets {
            println!("\t{}.{}", asset.name, asset.typ.ext());
        }

        if matches.get_flag("deep") {
            println!("deep scanning ..");

            let script_chunks = extract_assets_from_chunk_loader(&build.manifest, &mut cache)
                .await
                .context("failed to extract assets from chunk loader")?;
            println!("\tchunk loader: {} scripts", script_chunks.len());
        }

        if let Some(dump_values) = matches.get_many("dump") {
            let dumping = dump_values.copied().collect::<Vec<_>>();
            dump_items(&dumping, &mut build, &mut cache).await?;
        }
    }

    Ok(())
}

fn resolve_dumper(name: &str) -> Option<Box<dyn Dump>> {
    match name {
        "classes" => Some(Box::new(havoc::dump::CSSClasses)),
        "modules" => Some(Box::new(havoc::dump::WebpackModules)),
        _ => None,
    }
}

async fn dump_items(
    dumping: &[&str],
    artifact: &mut (dyn Artifact + Sync),
    assets: &mut AssetCache,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to obtain current working dir")?;

    for item in dumping {
        let mut dumper: Box<dyn Dump> =
            resolve_dumper(item).ok_or_else(|| anyhow!("`{}` is an unknown dumper", item))?;

        print!("dumping item \"{}\" ...", item);

        async {
            let result = dumper
                .dump(artifact, assets)
                .await
                .with_context(|| format!("failed to dump using dumper `{}`", item))?;

            let filename = result.filename();

            let full_filename = format!("havoc_{}_{}", artifact.dump_prefix(), filename);
            let dest = cwd.join(full_filename.clone());

            print!("\twriting \"{}\" to {} ...", result.name, full_filename);

            result
                .write(&dest)
                .with_context(|| format!("failed to write dump result to disk at {:?}", dest))?;

            println!(" done");

            anyhow::Ok(())
        }
        .instrument(tracing::info_span!("dumping", dumper = ?item))
        .await?
    }

    Ok(())
}
