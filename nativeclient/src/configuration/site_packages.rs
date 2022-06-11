
pub enum SearchStrategy {
    NONE,
    ALL,
    PEP561,
}

impl From<&str> for SearchStrategy {
    fn from(s: &str) -> Self {
        match s {
            "none" => SearchStrategy::NONE,
            "all" => SearchStrategy::ALL,
            "pep561" => SearchStrategy::PEP561,
        }
    }
}

impl Default for SearchStrategy {
    fn default() -> Self {
        SearchStrategy::NONE
    }
}