use crate::command_arguments::CommandArguments;
use crate::configuration::ide_features::IdeFeatures;
use crate::configuration::platform_aware;
use crate::configuration::platform_aware::from_json;
use crate::configuration::python_version::PythonVersion;
use crate::configuration::search_path::SimpleRawElement;
use crate::configuration::shared_memory::SharedMemory;
use crate::configuration::site_packages::SearchStrategy;
use crate::configuration::unwatched::UnwatchedDependency;
use crate::find_directories::{
    find_global_and_local_root, get_relative_local_root, CONFIGURATION_FILE,
    LOCAL_CONFIGURATION_FILE, LOG_DIRECTORY,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter::Map;
use std::path::{Path, PathBuf};

struct ExtensionElement {
    suffix: String,
    include_suffix_in_module_qualifier: bool,
}

#[derive(Default)]
struct Configuration {
    project_root: String,
    dot_pyre_directory: String,
    binary: Option<String>,
    buck_mode: Option<String>,
    do_not_ignore_errors_in: Vec<String>,
    excludes: Vec<String>,
    extensions: Vec<ExtensionElement>,
    ide_features: Option<IdeFeatures>,
    ignore_all_errors: Vec<String>,
    isolation_prefix: Option<String>,
    logger: Option<String>,
    number_of_workers: Option<i32>,
    oncall: Option<String>,
    other_critical_files: Vec<String>,
    pysa_version_hash: Option<String>,
    python_version: Option<PythonVersion>,
    relative_local_root: Option<String>,
    search_path: Vec<String>,
    shared_memory: SharedMemory,
    site_package_search_strategy: SearchStrategy,
    site_roots: Option<Vec<String>>,
    source_directories: Option<Vec<String>>,
    strict: bool,
    taint_models_path: Vec<String>,
    targets: Option<Vec<String>>,
    typeshed: Option<String>,
    unwatched_dependency: Option<UnwatchedDependency>,
    use_buck2: bool,
    version_hash: Option<String>,
}

impl Configuration {
    fn from_partial_configuration(
        project_root: &str,
        relative_local_root: Option<&str>,
        partial_configuration: PartialConfiguration,
    ) -> Self {
        let search_path = partial_configuration.search_path;

        return Self {
            project_root: String::from(project_root),
            dot_pyre_directory: partial_configuration
                .dot_pyre_directory
                .unwrap_or(project_root.join(LOG_DIRECTORY)),
            binary: partial_configuration.binary,
            buck_mode: partial_configuration.buck_mode,
            do_not_ignore_errors_in: partial_configuration.do_not_ignore_errors_in,
            excludes: partial_configuration.excludes,
            extensions: partial_configuration.extensions,
            ide_features: partial_configuration.ide_features,
            ignore_all_errors: partial_configuration.ignore_all_errors,
            isolation_prefix: partial_configuration.isolation_prefix,
            logger: partial_configuration.logger,
            number_of_workers: partial_configuration.number_of_workers,
            oncall: partial_configuration.oncall,
            other_critical_files: partial_configuration.other_critical_files,
            pysa_version_hash: partial_configuration.pysa_version_hash,
            python_version: partial_configuration.python_version,
            relative_local_root: relative_local_root,
            search_path: search_path
                .into_iter()
                .map(|path| path.expand_global_root(project_root))
                .collect::<Vec<String>>(),
            shared_memory: partial_configuration.shared_memory,
            site_package_search_strategy: partial_configuration
                .site_package_search_strategy
                .unwrap_or(SearchStrategy::NONE),
            site_roots: partial_configuration.site_roots,
            source_directories: partial_configuration.source_directories,
            strict: partial_configuration.strict.unwrap_or(false),
            taint_models_path: partial_configuration.taint_models_path,
            targets: partial_configuration.targets,
            typeshed: partial_configuration.typeshed,
            unwatched_dependency: partial_configuration.unwatched_dependency,
            use_buck2: partial_configuration.use_buck2.unwrap_or(false),
            version_hash: partial_configuration.version_hash,
        };
    }
}

#[derive(Default, Serialize, Deserialize)]
struct PartialConfiguration {
    binary: Option<String>,
    buck_mode: Option<String>,
    do_not_ignore_errors_in: Vec<String>,
    dot_pyre_directory: Option<String>,
    excludes: Vec<String>,
    extensions: Vec<String>,
    ide_features: Option<IdeFeatures>,
    ignore_all_errors: Vec<String>,
    isolation_prefix: Option<String>,
    logger: Option<String>,
    number_of_workers: Option<i32>,
    oncall: Option<String>,
    other_critical_files: Vec<String>,
    pysa_version_hash: Option<String>,
    python_version: Option<PythonVersion>,
    search_path: Vec<String>,
    shared_memory: SharedMemory,
    site_package_search_strategy: Option<SearchStrategy>,
    site_roots: Option<Vec<String>>,
    source_directories: Option<Vec<String>>,
    strict: Option<bool>,
    taint_models_path: Vec<String>,
    targets: Option<Vec<String>>,
    typeshed: Option<String>,
    unwatched_dependency: Option<UnwatchedDependency>,
    use_buck2: Option<bool>,
    version_hash: Option<String>,
}

impl PartialConfiguration {
    fn _get_depreacted_map() -> HashMap<String, String> {
        HashMap::from(("do_not_check", "ignore_all_errors"))
    }

    fn _get_extra_keys() -> HashSet<String, String> {
        [
            "create_open_source_configuration",
            "saved_state",
            "stable_client",
            "taint_models_path",
            "unstable_client",
        ]
    }

    fn from_command_arguments(arguments: CommandArguments) -> Self {
        let strict: Option<bool> = match arguments.strict {
            true => Some(true),
            false => None,
        };
        let source_directories = arguments
            .source_directories
            .into_iter()
            .map(|element| SimpleRawElement { root: element })
            .collect::<Vec<_>>();
        let targets = if arguments.targets.len() > 0 {
            Some(arguments.targets)
        } else {
            None
        };
        let python_version_string = arguments.python_version;
        let ide_features = if arguments.enable_hover.is_some()
            || arguments.enable_go_to_definition.is_some()
            || arguments.enable_find_symbols.is_some()
            || arguments.enable_find_all_references.is_some()
        {
            Some(IdeFeatures {
                hover_enabled: arguments.enable_hover,
                go_to_definition_enabled: arguments.enable_go_to_definition,
                find_symbols_enabled: arguments.enable_find_symbols,
                find_all_references_enabled: arguments.enable_find_all_references,
            })
        } else {
            None
        };
        PartialConfiguration {
            binary: arguments.binary,
            buck_mode: Some(from_json(arguments.buck_mode, "buck_mode")),
            do_not_ignore_errors_in: arguments.do_not_ignore_errors_in,
            dot_pyre_directory: arguments.dot_pyre_directory,
            excludes: arguments.exclude,
            extensions: [],
            ide_features: ide_features,
            ignore_all_errors: [],
            isolation_prefix: arguments.isolation_prefix,
            logger: arguments.logger,
            number_of_workers: arguments.number_of_workers,
            oncall: None,
            other_critical_files: [],
            pysa_version_hash: None,
            python_version: python_version_string.map(|pvs| PythonVersion::from_string(pvs)),
            search_path: arguments
                .search_path
                .into_iter()
                .map(|element| SimpleRawElement { root: element })
                .collect::<Vec<_>>(),
            shared_memory: SharedMemory {
                heap_size: arguments.shared_memory_heap_size,
                dependency_table_power: arguments.shared_memory_dependency_table_power,
                hash_table_power: arguments.shared_memory_hash_table_power,
            },
            site_package_search_strategy: None,
            site_roots: None,
            source_directories: Some(source_directories),
            strict: strict,
            taint_models_path: [],
            targets: targets,
            typeshed: arguments.typeshed,
            unwatched_dependency: None,
            use_buck2: arguments.use_buck2,
            version_hash: None,
        }
    }

    fn from_string(contents: &str) -> Self {
        fn is_list_of_string(elements: i32) -> bool {
            // isinstance(elements, list) && all(
            //     isinstance(element, str) for element in elements
            // )
            false
        }

        fn ensure_option_type(
            json: Map<String, i32>,
            name: str,
            expected_type: Type<T>,
        ) -> Result<Option<T>> {
            let result = json.pop(name, None);
            if result.is_none() {
                return None;
            } else if isinstance(result, expected_type) {
                return Some(result);
            } else {
                return Err(format!(
                    "Configuration field `{}` is expected to have type "
                    "{} but got: `{}`.", name, expected_type, result
                ));
            }
        }

        fn ensure_optional_string_or_string_dict(
            json: Dict<str, Any>,
            name: str,
        ) -> Option<Union<Dict<str, str>, str>> {
            let result = json.pop(name, None);
            if result.is_none() {
                return None;
            } else if isinstance(result, str) {
                return result;
            } else if isinstance(result, Dict) {
                for value in result.values() {
                    if !isinstance(value, str) {
                        format!(
                            "Configuration field `{}` is expected to be a dict of strings but got `{}`.", name, result
                        )
                    }
                }
                return result;
            } else {
                format!(
                    "Configuration field `{}` is expected to be a string or a dict of strings but got `{}`.", name, result
                );
            }
        }

        fn ensure_optional_string_list(json: Dict<str, Any>, name: str) -> Option<List<str>> {
            let result = json.pop(name, None);
            if result.is_none() {
                return None;
            } else if is_list_of_string(result) {
                return result;
            } else {
                format!(
                    "Configuration field `{}` is expected to be a list of strings but got `{}`.",
                    name, result
                );
            }
        }

        fn ensure_string_list(
            json: Dict<str, Any>,
            name: str,
            allow_single_string: bool,
        ) -> Vec<str> {
            let mut result = json.pop(name, []);
            if allow_single_string && isinstance(result, str) {
                result = [result];
            }
            if is_list_of_string(result) {
                return result;
            }
            format!(
                "Configuration field `{}` is expected to be a list of strings but got `{}`.",
                name, result
            )
        }

        fn ensure_list(json: Dict<str, object>, name: &str) -> Vec<object> {
            let result = json.pop(name, []);
            if isinstance(result, list) {
                return result;
            }
            format!(
                "Configuration field `{}` is expected to be a list but got `{}`.",
                name, result
            )
        }

        let configuration_json: PartialConfiguration = serde_json::from_str(contents);

        let partial_configuration = PartialConfiguration {
            binary: ensure_option_type(configuration_json, "binary", str),
            buck_mode: from_json(
                ensure_optional_string_or_string_dict(configuration_json, "buck_mode"),
                "buck_mode",
            ),
            do_not_ignore_errors_in: ensure_string_list(
                configuration_json,
                "do_not_ignore_errors_in",
            ),
            dot_pyre_directory: dot_pyre_directory.map(|x| Path(x)),
            excludes: ensure_string_list(configuration_json, "exclude", allow_single_string = True),
            extensions: vec![
                ExtensionElement.from_json(json)
                for json in ensure_list(configuration_json, "extensions")
            ],
            ide_features: ide_features,
            ignore_all_errors: ensure_string_list(configuration_json, "ignore_all_errors"),
            isolation_prefix: ensure_option_type(configuration_json, "isolation_prefix", str),
            logger: ensure_option_type(configuration_json, "logger", str),
            number_of_workers: ensure_option_type(configuration_json, "workers", int),
            oncall: ensure_option_type(configuration_json, "oncall", str),
            other_critical_files: ensure_string_list(configuration_json, "critical_files"),
            pysa_version_hash: ensure_option_type(configuration_json, "pysa_version", str),
            python_version: python_version,
            search_path: search_path,
            shared_memory: shared_memory,
            site_package_search_strategy: site_package_search_strategy,
            site_roots: ensure_optional_string_list(configuration_json, "site_roots"),
            source_directories: source_directories,
            strict: ensure_option_type(configuration_json, "strict", bool),
            taint_models_path: ensure_string_list(
                configuration_json,
                "taint_models_path",
                allow_single_string = True,
            ),
            targets: ensure_optional_string_list(configuration_json, "targets"),
            typeshed: ensure_option_type(configuration_json, "typeshed", str),
            unwatched_dependency: unwatched_dependency,
            use_buck2: ensure_option_type(configuration_json, "use_buck2", bool),
            version_hash: ensure_option_type(configuration_json, "version", str),
        };

        // Check for deprecated and unused keys
        for (deprecated_key, replacement_key) in PartialConfiguration::_get_depreacted_map().items()
        {
            if configuration_json.contains(deprecated_key) {
                configuration_json.pop(deprecated_key);
                // warning
                println!(
                    "Configuration file uses deprecated item `{}`. Please migrate to its replacement `{}`", deprecated_key, replacement_key
                )
            }
        }
        let extra_keys = PartialConfiguration::_get_extra_keys();
        for unrecognized_key in configuration_json {
            if !extra_keys.contains(unrecognized_key) {
                // warning
                println!("Unrecognized configuration item: {}", unrecognized_key)
            }
        }

        partial_configuration
    }

    fn from_file(path: &str) -> Self {
        Self::from_string(fs::open(path).read_text()?)
    }

    fn expand_relative_paths(&self, root: &str) -> Self {
        let binary = self.binary;
        if binary.is_some() {
            binary = expand_relative_path(root, binary);
        }
        let logger = self.logger;
        if logger.is_some() {
            logger = expand_relative_path(root, logger);
        }
        let mut source_directories = self.source_directories;
        if source_directories.is_some() {
            source_directories = vec![
                path.expand_relative_root(root) for path in source_directories
            ];
        }
        let typeshed = self.typeshed;
        if typeshed.is_some() {
            typeshed = expand_relative_path(root, typeshed);
        }
        let unwatched_dependency = self.unwatched_dependency;
        if unwatched_dependency.is_some() {
            files = unwatched_dependency.files;
            unwatched_dependency = unwatched.UnwatchedDependency(
                change_indicator = unwatched_dependency.change_indicator,
                files = unwatched.UnwatchedFiles(
                    root = expand_relative_path(root, files.root),
                    checksum_path = files.checksum_path,
                ),
            )
        }
        return Self {
            binary,
            buck_mode: self.buck_mode,
            do_not_ignore_errors_in: self
                .do_not_ignore_errors_in
                .into_iter()
                .map(|path| expand_relative_path(root, path)),
            dot_pyre_directory: self.dot_pyre_directory,
            excludes: self.excludes,
            extensions: self.extensions,
            ide_features: self.ide_features,
            ignore_all_errors: self
                .ignore_all_errors
                .into_iter()
                .map(|path| expand_relative_path(root, path)),
            isolation_prefix: self.isolation_prefix,
            logger: logger,
            number_of_workers: self.number_of_workers,
            oncall: self.oncall,
            other_critical_files: self
                .other_critical_files
                .into_iter()
                .map(|path| expand_relative_path(root, path)),
            pysa_version_hash: self.pysa_version_hash,
            python_version: self.python_version,
            search_path: self
                .search_path
                .into_iter()
                .map(|path| path.expand_relative_root(root)),
            shared_memory: self.shared_memory,
            site_package_search_strategy: self.site_package_search_strategy,
            site_roots: self.site_roots,
            source_directories: source_directories,
            strict: self.strict,
            taint_models_path: self
                .taint_models_path
                .into_iter()
                .map(|path| expand_relative_path(root, path)),
            targets: self.targets,
            typeshed: typeshed,
            unwatched_dependency: unwatched_dependency,
            use_buck2: self.use_buck2,
            version_hash: self.version_hash,
            targets: todo!(),
            typeshed,
            unwatched_dependency,
            use_buck2: todo!(),
            version_hash: todo!(),
        };
    }
}

fn merge_partial_configurations(
    base: PartialConfiguration,
    overwrite: PartialConfiguration,
) -> PartialConfiguration {
    PartialConfiguration::merge(base, overwrite)
}

fn create_configuration(
    arguments: CommandArguments,
    base_directory: &str,
) -> Result<Configuration, String> {
    let local_root_argument = arguments.local_configuration;
    let search_base = match local_root_argument {
        None => PathBuf::from(base_directory),
        Some(local_root_argument) => [base_directory, &local_root_argument]
            .iter()
            .collect::<PathBuf>(),
    }
    .into_os_string()
    .into_string()
    .expect("Invalid UTF-8");
    let found_root = find_global_and_local_root(&search_base);

    // If the local root was explicitly specified but does not exist, return an
    // error instead of falling back to current directory.
    match local_root_argument {
        Some(local_root_argument) => {
            match found_root {
                None => Err(format!("A local configuration path was explicitly specified, but no {} file was found in {} or its parents.", CONFIGURATION_FILE, search_base)),
                Some(found_root) => {
                    match found_root.local_root {
                        None => Err(format!(
                            "A local configuration path was explicitly specified, but no {} file was found in {} or its parents.", LOCAL_CONFIGURATION_FILE, search_base
                        )),
                        _ => Ok(()),
                    }
                },
            }
        },
        None => Ok(())
    }?;
    let cwd = std::env::current_dir()
        .expect("Cannot get current_dir")
        .into_os_string()
        .into_string()
        .expect("cannot convert");

    let command_argument_configuration =
        PartialConfiguration::from_command_arguments(arguments).expand_relative_paths(&cwd);
    match found_root {
        None => {
            let project_root = &cwd;
            let relative_local_root = None;
            let partial_configuration = command_argument_configuration;
        }
        Some(found_root) => {
            let project_root = found_root.global_root;
            let config_file = Path::new(&project_root)
                .join(CONFIGURATION_FILE)
                .into_os_string()
                .into_string()
                .expect("cannot convert");
            let relative_local_root = None;
            let partial_configuration =
                PartialConfiguration::from_file(&config_file).expand_relative_paths(&project_root);
            let local_root = found_root.local_root;
            match local_root {
                Some(local_root) => {
                    let relative_local_root = get_relative_local_root(project_root, local_root);
                    let partial_configuration = merge_partial_configurations(
                        partial_configuration,
                        PartialConfiguration::from_file(local_root / LOCAL_CONFIGURATION_FILE)
                            .expand_relative_paths(str(local_root)),
                    );
                }
                None => (),
            }
            let partial_configuration =
                merge_partial_configurations(partial_configuration, command_argument_configuration);
        }
    }

    Ok(Configuration::from_partial_configuration(
        project_root,
        relative_local_root,
        partial_configuration,
    ))
}
