use havoc::discord;
use havoc::scrape;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = scrape::scrape_fe(discord::Branch::Canary)?;
    dbg!(result);
    Ok(())
}
