use anyhow::Context;
use clap::{Arg, SubCommand};
use havoc::scrape;
use havoc::wrecker::Wrecker;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

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

        let mut wrecker = Wrecker::<()>::scrape(target)?;
        wrecker.fetch_assets()?;
        let wrecker = wrecker.glean_fe()?;

        println!(
            "Discord {:?} ({})",
            wrecker.item.manifest.branch, wrecker.item.number
        );

        println!("\nAssets:");
        for asset in &wrecker.item.manifest.assets {
            println!("- {}.{}", asset.name, asset.typ.ext());
        }

        if let Some(dumping) = matches
            .values_of("dump")
            .map(|values| values.collect::<Vec<_>>())
        {
            for item in &dumping {
                match *item {
                    "classes" => {
                        let class_module_map = wrecker.parse_classes()?;
                        let json = serde_json::to_string(&class_module_map)
                            .context("failed to serialize class module map")?;

                        let filename = format!(
                            "havoc_{:?}_{}_class_mappings.json",
                            wrecker.item.manifest.branch, wrecker.item.number
                        );

                        std::fs::write(&filename, json)
                            .context("failed to write serialized class module map to disk")?;
                    }
                    "chunks" => {
                        let (script, _chunk) = wrecker.parse_chunks()?;
                        let json = serde_json::to_string(&script)?;
                        let filename = format!(
                            "havoc_{:?}_{}_entrypoint_ast.json",
                            wrecker.item.manifest.branch, wrecker.item.number
                        );
                        std::fs::write(&filename, &json)
                            .context("failed to write serialized entrypoint ast to disk")?;
                    }
                    _ => {
                        clap::Error::value_validation_auto(format!("Unknown dump item: {}", *item))
                            .exit()
                    }
                }
            }
        }
    }

    Ok(())
}
