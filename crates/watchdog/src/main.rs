use anyhow::{Context, Result};
use watchdog::config::Config;
use watchdog::db::Db;

async fn run(config: Config) -> Result<()> {
    let conn = rusqlite::Connection::open(&config.database_file_path)?;

    let db = Db::new(conn);

    let scraper_db = db.clone();
    let scraper_config = config.clone();
    tokio::spawn(async move {
        watchdog::scraping::scrape_indefinitely(&scraper_config, scraper_db)
            .await
            .expect("scraper crashed");
    });

    let web_db = db.clone();
    let state = watchdog::api::AppState { db: web_db };
    let router = watchdog::api::create_router().with_state(state);

    tracing::info!(
        "binding http api server to {:?}",
        config.http_api_server_bind_address
    );
    axum::Server::bind(&config.http_api_server_bind_address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config_file_path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: watchdog <path/to/config.toml>");
            std::process::exit(1);
        }
    };

    let config_file_text =
        std::fs::read_to_string(config_file_path).context("cannot read config file")?;
    let config: Config = toml::from_str(&config_file_text).context("cannot parse config file")?;

    run(config).await
}
