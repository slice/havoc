use havoc::discord;
use havoc::scrape;

fn main() {
    pretty_env_logger::init();

    match scrape::scrape_fe(discord::Branch::Stable) {
        Ok(build) => {
            println!("Discord {:?} ({})", build.branch, build.number);
            println!("\nAssets:");
            for asset in &build.assets {
                println!("  {}.{}", asset.name, asset.typ.ext());
            }
        }
        Err(err) => {
            eprintln!("failed to scrape: {}", err);
        }
    }
}
