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
}
impl std::fmt::Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
