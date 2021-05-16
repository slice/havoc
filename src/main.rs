use anyhow::Context;
use clap::{Arg, SubCommand};

use havoc::artifact::DumpItem;
use havoc::scrape;
use havoc::wrecker::Wrecker;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let cwd = std::env::current_dir().expect("couldn't access current directory");

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

        let wrecker = Wrecker::scrape_fe_build(target)?;

        println!("{}", wrecker.artifact);

        println!("\nAssets:");
        for asset in wrecker.artifact.assets() {
            println!("- {}.{}", asset.name, asset.typ.ext());
        }

        if let Some(dumping) = matches
            .values_of("dump")
            .map(|values| values.collect::<Vec<_>>())
        {
            for item in &dumping {
                let dump_item: DumpItem = item
                    .parse()
                    .map_err(|_| format!("`{}` is not a valid dump item", item))
                    .expect("invalid dump item");

                if !wrecker.artifact.supports_dump_item(dump_item) {
                    panic!("unsupported dump item for this artifact");
                }

                print!("dumping item \"{}\" ...", item);

                let dump_results = wrecker
                    .dump(dump_item)
                    .with_context(|| format!("failed to dump {:?} ({})", dump_item, item))?;

                println!(" {} result(s)", dump_results.len());

                for result in &dump_results {
                    let filename = result.filename();

                    let full_filename =
                        format!("havoc_{}_{}", wrecker.artifact.dump_prefix(), filename);
                    let dest = cwd.join(full_filename.clone());

                    println!(
                        "\twriting \"{}\" ({:?}, {}) to {}",
                        result.name,
                        result.typ,
                        result.content.len(),
                        full_filename
                    );

                    result.dump_to(&dest).unwrap_or_else(|err| {
                        panic!(
                            "failed to dump {:?} ({}) to disk: {:?}",
                            dump_item, item, err
                        )
                    });
                }
            }
        }
    }

    Ok(())
}
