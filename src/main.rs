use anyhow::{anyhow, Context, Result};
use clap::{Arg, SubCommand};

use havoc::artifact::Artifact;
use havoc::discord::Assets;
use havoc::dump::DumpItem;
use havoc::scrape;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let matches = clap::App::new("havoc")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .version("0.1.0")
        .author("slice <tinyslices@gmail.com>")
        .about("discord client scraping and processing toolkit")
        .subcommand(
            SubCommand::with_name("scrape")
                .about("Scrape a single target, once")
                .arg(
                    Arg::with_name("TARGET")
                        .required(true)
                        .help("the target to scrape")
                        .takes_value(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("dump")
                        .long("dump")
                        .short("d")
                        .multiple(true)
                        .help("build items to dump")
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("scrape") {
        let target_str = matches.value_of("TARGET").unwrap();

        let target = match target_str.parse::<scrape::Target>() {
            Ok(target) => target,
            Err(err) => {
                let clap_err =
                    clap::Error::value_validation_auto(format!("Invalid scrape target: {}", err));
                clap_err.exit();
            }
        };

        let scrape::Target::Frontend(branch) = target;

        let manifest =
            scrape::scrape_fe_manifest(branch).context("failed to scrape frontend manifest")?;
        let mut assets = havoc::discord::Assets::with_assets(manifest.assets.clone());
        let mut build = crate::scrape::scrape_fe_build(manifest, &mut assets)
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
            dump_items(&dumping, &mut build, &mut assets)?;
        }
    }

    Ok(())
}

fn dump_items(dumping: &[&str], artifact: &mut dyn Artifact, assets: &mut Assets) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to obtain current working dir")?;

    for item in dumping {
        let dump_item: DumpItem = item
            .parse()
            .map_err(|_| anyhow!("`{}` is not a valid dump item", item))
            .context("invalid dump item")?;

        if !artifact.supports_dump_item(dump_item) {
            return Err(anyhow!("unsupported dump item for this artifact"));
        }

        print!("dumping item \"{}\" ...", item);

        let dump_span = tracing::info_span!("dumping", ?dump_item);
        let _span = dump_span.enter();

        let dump_results = artifact
            .dump(dump_item, assets)
            .with_context(|| format!("failed to dump {:?} ({})", dump_item, item))?;

        println!(" {} result(s)", dump_results.len());

        for result in &dump_results {
            let filename = result.filename();

            let full_filename = format!("havoc_{}_{}", artifact.dump_prefix(), filename);
            let dest = cwd.join(full_filename.clone());

            print!(
                "\twriting \"{}\" ({:?}, {}) to {} ...",
                result.name,
                result.typ,
                result.content.len(),
                full_filename
            );

            result
                .write(&dest)
                .with_context(|| format!("failed to write {:?} ({}) to disk", dump_item, item))?;

            println!(" done");
        }
    }

    Ok(())
}
