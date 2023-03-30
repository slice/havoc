use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use watchdog::config::Config;
use watchdog::db::Db;

async fn run(config: Config) -> Result<()> {
    tracing::info!("connecting to postgres: {}", config.postgres.url);

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.postgres.max_connections)
        .connect(&config.postgres.url)
        .await?;

    let db = Db::new(pool);

    spawn_indefinite_scraper(db.clone(), config.clone());

    let state = watchdog::api::AppState { db: db.clone() };
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

fn spawn_indefinite_scraper(db: Db, config: Config) {
    let supervisor = tokio::spawn(async move {
        let default_backoff = Duration::from_secs(1);
        let mut restart_backoff = default_backoff;
        let mut last_backoff: Option<Instant> = None;

        loop {
            let Err(err) = watchdog::scraping::scrape_forever(&config, &db).await else {
                panic!("indefinite scraper terminated without an error (this should never happen)");
            };
            tracing::error!(?restart_backoff, "indefinite scraper died: {}", err);

            // If it's been five minutes or longer since the last backoff, reset
            // it back to the default wait time.
            let resetting_backoff =
                last_backoff.map_or(false, |last| last.elapsed() >= Duration::new(60 * 5, 0));
            if resetting_backoff {
                restart_backoff = default_backoff;
            }

            tokio::time::sleep(restart_backoff).await;

            if !resetting_backoff {
                restart_backoff = restart_backoff.checked_mul(2).unwrap_or(default_backoff);
            }

            last_backoff = Some(Instant::now());
        }
    });

    tokio::spawn(async move {
        let err = supervisor.await.expect_err(
            "indefinite scraper supervisor terminated without an error (this should never happen)",
        );

        tracing::error!(
            ?err,
            "indefinite scraper panicked! something is very wrong here, aborting: {}",
            err
        );

        std::process::exit(1);
    });
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
