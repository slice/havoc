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

        let mut wrecker = Wrecker::scrape(target)?;
        wrecker.fetch_assets()?;
        wrecker.glean_fe()?;

        println!(
            "Discord {:?} ({})",
            wrecker.manifest.branch,
            wrecker.build.unwrap().number
        );

        println!("\nAssets:");
        for asset in &wrecker.manifest.assets {
            println!("- {}.{}", asset.name, asset.typ.ext());
        }
    }

    Ok(())
}
