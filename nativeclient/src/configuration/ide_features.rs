const DEFAULT_HOVER_ENABLED: bool = false;
const DEFAULT_GO_TO_DEFINITION_ENABLED: bool = false;
const DEFAULT_FIND_SYMBOLS_ENABLED: bool = false;
const DEFAULT_FIND_ALL_REFERENCES_ENABLED: bool = false;

pub struct IdeFeatures {
    hover_enabled: Option<bool>,
    go_to_definition_enabled: Option<bool>,
    find_symbols_enabled: Option<bool>,
    find_all_references_enabled: Option<bool>,
}