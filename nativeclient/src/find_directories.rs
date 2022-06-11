use std::path::Path;

pub const CONFIGURATION_FILE: &'static str = ".pyre_configuration";
pub const LOCAL_CONFIGURATION_FILE: &'static str = ".pyre_configuration.local";
pub const LOG_DIRECTORY: &'static str = ".pyre";

fn _find_parent_directory_containing(
    base: &str,
    target: &str,
    predicate: fn(&str) -> bool,
    stop_search_after: Option<i32>,
) -> Option<String> {
    for (i, candidate_directory) in Path::new(base).ancestors().enumerate() {
        let candidate_path = Path::new(candidate_directory).join(Path::new(target));
        // We might not have sufficient permission to read the file/directory.
        // In that case, pretend the file doesn't exist.
        // TODO: Check permissions.
        if predicate(candidate_path.as_path().to_str().expect("Not a UTF-8 path")) {
            return Some(String::from(
                candidate_directory.to_str().expect("Not a UTF-8 path"),
            ));
        }
        match stop_search_after {
            None => {}
            Some(stop) => {
                if i >= stop as usize {
                    return None;
                }
            }
        }
    }
    return None;
}

/// Walk directories upwards from `base`, until the root directory is
/// reached. At each step, check if the `target` file exist, and return
/// the closest such directory if found. Return None if the search is
/// unsuccessful.
/// We stop searching after checking `stop_search_after` parent
/// directories of `base` if provided; this is mainly for testing.
fn find_parent_directory_containing_file(
    base: &str,
    target: &str,
    stop_search_after: Option<i32>,
) -> Option<String> {
    return _find_parent_directory_containing(
        base,
        target,
        |p: &str| Path::new(p).is_file(),
        stop_search_after,
    );
}

struct FoundRoot {
    pub global_root: String,
    pub local_root: Option<String>,
}

/// Walk directories upwards from `base` and try to find both the global and local
/// pyre configurations.
/// Return `None` if no global configuration is found.
/// If a global configuration exists but no local configuration is found below it,
/// return the path to the global configuration.
/// If both global and local exist, return them as a pair.
pub fn find_global_and_local_root(base: &str) -> Option<FoundRoot> {
    let found_global_root = find_parent_directory_containing_file(base, CONFIGURATION_FILE, None)?;

    let found_local_root =
        find_parent_directory_containing_file(base, LOCAL_CONFIGURATION_FILE, None);
    match found_local_root {
        None => Some(FoundRoot {
            global_root: found_global_root,
            local_root: None,
        }),
        Some(found_local_root) => {
            // If the global configuration root is deeper than local configuration, ignore local.
            let ancestors = Path::new(&found_global_root)
                .ancestors()
                .collect::<Vec<_>>();
            if ancestors.contains(&Path::new(&found_local_root)) {
                Some(FoundRoot {
                    global_root: found_global_root,
                    local_root: None,
                })
            } else {
                Some(FoundRoot {
                    global_root: found_global_root,
                    local_root: Some(found_local_root),
                })
            }
        }
    }
}

pub fn get_relative_local_root(global_root: Path, local_root: Option<String>) -> Option<String> {
    // except ValueError:
    // This happens when `local_root` is not prefixed by `global_root`
    // return None
    local_root.map(|local_root| local_root.relative_to(global_root))
}
