use havoc::discord::Branch;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Subscription {
    pub branches: Vec<Branch>,
    pub discord_webhook_url: String,
}
