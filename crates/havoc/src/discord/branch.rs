use serde::{Deserialize, Serialize};
use url::Url;

/// A Discord branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Branch {
    Stable,
    Ptb,
    Canary,
    Development,
}

impl std::fmt::Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Branch::*;

        match self {
            Stable => write!(f, "Stable"),
            Ptb => write!(f, "PTB"),
            Canary => write!(f, "Canary"),
            Development => write!(f, "Development"),
        }
    }
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

    /// Returns whether the branch has an accessible frontend.
    pub fn has_frontend(&self) -> bool {
        *self != Branch::Development
    }

    /// Returns a suitable color representative of this branch.
    pub fn color(&self) -> u32 {
        use Branch::*;

        match self {
            Stable => 0x7289da,
            Ptb => 0x99aab5,
            Canary => 0xf1c40f,
            Development => 0x4d4d4d,
        }
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
