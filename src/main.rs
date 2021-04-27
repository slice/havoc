use havoc::discord;
use havoc::scrape;

fn main() {
    let result = scrape::scrape_fe(discord::Branch::Canary);
    dbg!(result);
}
