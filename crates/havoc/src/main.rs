use std::io::Write;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, ArgMatches, Command};
use termcolor::{Color, ColorChoice, ColorSpec, WriteColor};
use tracing::Instrument;

use havoc::artifact::Artifact;
use havoc::discord::{AssetCache, FeAsset, FeAssetType, FeBuild, RootScript};
use havoc::dump::Dump;
use havoc::scrape::{self, extract_assets_from_chunk_loader};

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
        .arg(
            clap::arg!(color: --color "whether to color the output")
                .default_value("auto")
                .default_missing_value("auto")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(ColorChoice))
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
                    clap::arg!(--links "add hyperlinks to the output (for supported terminals)")
                        .action(ArgAction::SetTrue),
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

fn create_stdout(matches: &ArgMatches) -> (ColorChoice, termcolor::StandardStream) {
    let color_choice: ColorChoice = {
        let choice_args = *matches
            .get_one::<ColorChoice>("color")
            .unwrap_or(&ColorChoice::Auto);

        if choice_args == ColorChoice::Auto && !atty::is(atty::Stream::Stdout) {
            ColorChoice::Never
        } else {
            choice_args
        }
    };

    let stdout = termcolor::StandardStream::stdout(color_choice);
    (color_choice, stdout)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = app();
    let matches = app.get_matches();
    let (_color_choice, mut stdout) = create_stdout(&matches);

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

        print_build(&build, &mut cache, matches, &mut stdout).await?;

        if let Some(dump_values) = matches.get_many("dump") {
            let dumping = dump_values.copied().collect::<Vec<_>>();
            dump_items(&dumping, &mut build, &mut cache).await?;
        }
    }

    Ok(())
}

async fn print_build(
    build: &FeBuild,
    cache: &mut AssetCache,
    matches: &ArgMatches,
    output: &mut termcolor::StandardStream,
) -> Result<()> {
    use havoc::discord::Branch::*;

    let bg = match build.manifest.branch {
        Canary => Color::Yellow,
        Stable => Color::Green,
        Ptb => Color::Blue,
        Development => Color::White,
    };
    let fg = match build.manifest.branch {
        Canary | Development => Color::Black,
        _ => Color::White,
    };

    output.set_color(ColorSpec::new().set_bg(Some(bg)).set_fg(Some(fg)))?;
    write!(output, "{}", build)?;
    output.set_color(ColorSpec::new().set_bg(None).set_fg(None))?;
    output.reset()?;
    writeln!(output, "\n")?;

    let assets = &build.manifest.assets;

    writeln!(output, "surface assets ({}):", assets.len())?;

    let mut write_asset_plain = |asset: &FeAsset, detail: Option<String>| -> Result<()> {
        output.set_color(ColorSpec::new().set_bold(true))?;
        write!(output, "\t")?;

        let hyperlinking = *matches.get_one("links").unwrap_or(&false);
        if hyperlinking {
            write!(output, "\x1b]8;;{}\x1b\\", asset.url())?;
        }

        write!(output, "{}", asset.filename())?;
        output.set_color(ColorSpec::new().set_bold(false))?;

        if hyperlinking {
            write!(output, "\x1b]8;;\x1b\\")?;
        }

        if let Some(detail) = detail {
            writeln!(output, " ({})", detail)?;
        } else {
            writeln!(output)?;
        }

        Ok(())
    };

    for (asset, root_script_type) in assets
        .filter_by_type(FeAssetType::Js)
        .zip(RootScript::assumed_ordering().into_iter())
    {
        match root_script_type {
            RootScript::ChunkLoader if matches.get_flag("deep") => {
                if matches.get_flag("deep") {
                    let script_chunks = extract_assets_from_chunk_loader(&build.manifest, cache)
                        .await
                        .context("failed to extract assets from chunk loader")?;
                    write_asset_plain(
                        asset,
                        Some(format!(
                            "chunk loader, {} script chunks",
                            script_chunks.len()
                        )),
                    )?;

                    for (chunk_id, script_chunk) in script_chunks.iter().take(7) {
                        println!("\t\t{}: {}", chunk_id, script_chunk.filename());
                    }
                    println!("\t\t...");
                }
            }
            _ => {
                write_asset_plain(asset, Some(format!("{}", root_script_type)))?;
            }
        }
    }
    for asset in assets.filter_by_type(FeAssetType::Css) {
        write_asset_plain(asset, None)?;
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
