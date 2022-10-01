use std::path::PathBuf;

use serde::Deserialize;

use crate::subscription::Subscription;

#[derive(Deserialize)]
pub struct Config {
    pub interval_milliseconds: u64,
    pub database_file_path: PathBuf,
    pub subscriptions: Vec<Subscription>,
}
