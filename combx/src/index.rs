use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Index {
    VoteResults,
    GovProposals,
    Decrees,
    Delegates,
}

impl Index {
    pub fn as_str(&self) -> &str {
        match self {
            Index::VoteResults => "vote_results",
            Index::GovProposals => "gov_props",
            Index::Decrees => "decrees",
            Index::Delegates => "delegates",
        }
    }

    pub fn uid(&self, parliament: Parliament) -> String {
        format!("{}-{}", parliament.as_str(), self.as_str())
    }
}
impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
pub enum Parliament {
    #[default]
    At,
    Eu,
}

impl Parliament {
    pub fn as_str(&self) -> &str {
        match self {
            Parliament::At => "at",
            Parliament::Eu => "eu",
        }
    }

    pub fn from_council(council: &str) -> Parliament {
        match council {
            "ep" => Parliament::Eu,
            _ => Parliament::At,
        }
    }
}

impl std::fmt::Display for Parliament {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
