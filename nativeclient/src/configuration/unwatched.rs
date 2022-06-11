use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct UnwatchedFiles {
    root: String,
    checksum_path: String,
}

struct InvalidConfigurationError {
    msg: String,
}

impl UnwatchedFiles {
    fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to send to JSON")
    }
}

#[derive(Serialize, Deserialize)]
pub struct UnwatchedDependency {
    change_indicator: String,
    files: UnwatchedFiles,
}

impl UnwatchedDependency {
    fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to send to JSON")
    }
}
