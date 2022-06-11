use std::path::Path;

pub(crate) fn expand_relative_path<'a>(root: &'a str, path: &'a str) -> &'a str {
    let expanded_path = Path::from(path).canonicalize().expect("Cannot canonicalize");
    if expanded_path.is_absolute() {
        expanded_path.as_path().to_str().expect("cannot convert to string")
    }
    else {
        Path::from(path).join(expanded_path).to_str().unwrap()
    }
}


pub(crate) fn expand_global_root<'a>(path: &'a str, global_root: &'a str) -> &'a str {
    if path.startswith("//") {
        expand_relative_path(global_root, &path[2..])
    } else {
        path
    }
}