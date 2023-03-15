use std::path::PathBuf;

use serde::Deserialize;

use crate::subscription::Subscription;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub interval_milliseconds: u64,
    pub database_file_path: PathBuf,
    pub subscriptions: Vec<Subscription>,
    pub http_api_server_bind_address: std::net::SocketAddr,
}
