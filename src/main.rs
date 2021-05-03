use std::collections::HashMap;

use clap::{Arg, SubCommand};
use havoc::discord;
use havoc::wrecker::Wrecker;

lazy_static::lazy_static! {
    // TODO(slice): `Into<Target>` system.
    static ref BRANCHES: HashMap<&'static str, discord::Branch> = {
        let mut h = HashMap::new();
        h.insert("canary", discord::Branch::Canary);
        h.insert("ptb", discord::Branch::Ptb);
        h.insert("stable", discord::Branch::Stable);
        h
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let validate_branch = |arg: String| {
        let q: &str = &arg;
        BRANCHES
            .contains_key(q)
            .then(|| ())
            .ok_or("Invalid branch.".to_owned())
    };

    let matches = clap::App::new("havoc")
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
                        .validator(validate_branch)
                        .index(1),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("scrape") {
        let target_str = matches.value_of("TARGET").unwrap();
        let target_branch = BRANCHES.get(target_str).unwrap();

        // NOTE(slice): Perform crude matching for now.
        let mut wrecker = Wrecker::scrape_fe(*target_branch)?;
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
