use serde::Serialize;
use url::Url;

/// A Discord branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Branch {
    Stable,
    Ptb,
    Canary,
    Development,
}

impl Branch {
    /// Returns the base URL of this branch.
    pub fn base(&self) -> Url {
        use Branch::*;

        match self {
            Stable => "https://discord.com".parse().unwrap(),
            Ptb => "https://ptb.discord.com".parse().unwrap(),
            Canary => "https://canary.discord.com".parse().unwrap(),
            Development => panic!("called `Branch::base()` on `Branch::Development`"),
        }
    }

    pub fn has_frontend(&self) -> bool {
        *self != Branch::Development
    }
}

impl std::str::FromStr for Branch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Branch::*;

        match s {
            "stable" => Ok(Stable),
            "ptb" => Ok(Ptb),
            "canary" => Ok(Canary),
            "development" => Ok(Development),
            _ => Err(()),
        }
    }
}
