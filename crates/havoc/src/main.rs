use anyhow::{anyhow, Context, Result};
use clap::{App, AppSettings, Arg};

use havoc::artifact::Artifact;
use havoc::discord::Assets;
use havoc::dump::Dump;
use havoc::scrape;

fn app() -> App<'static> {
    App::new("havoc")
        .global_setting(AppSettings::PropagateVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .version("0.1.0")
        .author("slice <tinyslices@gmail.com>")
        .about("discord client scraping and processing toolkit")
        .subcommand(
            App::new("scrape")
                .about("Scrape a target")
                .arg(
                    Arg::new("TARGET")
                        .required(true)
                        .help("the target to scrape")
                        .takes_value(true)
                        .index(1),
                )
                .arg(
                    Arg::new("dump")
                        .long("dump")
                        .short('d')
                        .multiple_occurrences(true)
                        .help("build items to dump")
                        .takes_value(true),
                ),
        )
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = app();
    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("scrape") {
        let target: scrape::Target = matches
            .value_of_t("TARGET")
            .unwrap_or_else(|err| err.exit());

        let scrape::Target::Frontend(branch) = target;

        let manifest = scrape::scrape_fe_manifest(branch)
            .await
            .context("failed to scrape frontend manifest")?;
        let mut assets = havoc::discord::Assets::with_assets(manifest.assets.clone());
        let mut build = crate::scrape::scrape_fe_build(manifest, &mut assets)
            .await
            .context("failed to scrape frontend build")?;

        println!("scraped: {}", build);

        println!("assets ({}):", assets.assets.len());

        for asset in &assets.assets {
            println!("\t{}.{} ({:?})", asset.name, asset.typ.ext(), asset.typ);
        }

        if let Some(dumping) = matches
            .values_of("dump")
            .map(|values| values.collect::<Vec<_>>())
        {
            dump_items(&dumping, &mut build, &mut assets).await?;
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
    artifact: &mut dyn Artifact,
    assets: &mut Assets,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to obtain current working dir")?;

    for item in dumping {
        let mut dumper: Box<dyn Dump> =
            resolve_dumper(item).ok_or_else(|| anyhow!("`{}` is an unknown dumper", item))?;

        print!("dumping item \"{}\" ...", item);

        let dump_span = tracing::info_span!("dumping", dumper = ?item);
        let _span = dump_span.enter();

        let result = dumper
            .dump(assets)
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
    }

    Ok(())
}
