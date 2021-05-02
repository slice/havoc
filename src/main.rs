use havoc::discord;
use havoc::wrecker::Wrecker;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let mut wrecker = Wrecker::scrape_fe(discord::Branch::Stable)?;
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

    Ok(())
}
