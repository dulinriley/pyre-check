use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(name = "pyre")]
#[clap(version = "1.0")]
#[clap(about = "Client for accessing pyre server", long_about = None)]
#[clap(propagate_version = true)]
pub struct CommandArguments {
    #[clap(long)]
    pub local_configuration: Option<String>,
    #[clap(long)]
    pub version: bool,
    #[clap(long)]
    pub debug: bool,
    #[clap(long)]
    pub sequential: bool,
    #[clap(long)]
    pub strict: bool,
    #[clap(long)]
    pub show_error_traces: bool,
    #[clap(long, default_value_t = String::from("text"))]
    pub output: String,
    #[clap(long)]
    pub enable_profiling: bool,
    #[clap(long)]
    pub enable_memory_profiling: bool,
    #[clap(long)]
    pub noninteractive: bool,
    #[clap(long)]
    pub logging_sections: Option<String>,
    #[clap(long)]
    pub log_identifier: Option<String>,
    #[clap(long)]
    pub logger: Option<String>,
    #[clap(long)]
    pub targets: Vec<String>,
    #[clap(long)]
    pub source_directories: Vec<String>,
    #[clap(long)]
    pub do_not_ignore_errors_in: Vec<String>,
    #[clap(long)]
    pub buck_mode: Option<String>,
    #[clap(long)]
    pub no_saved_state: bool,
    #[clap(long)]
    pub search_path: Vec<String>,
    #[clap(long)]
    pub binary: Option<String>,
    #[clap(long)]
    pub exclude: Vec<String>,
    #[clap(long)]
    pub typeshed: Option<String>,
    #[clap(long)]
    pub save_initial_state_to: Option<String>,
    #[clap(long)]
    pub load_initial_state_from: Option<String>,
    #[clap(long)]
    pub changed_files_path: Option<String>,
    #[clap(long)]
    pub saved_state_project: Option<String>,
    #[clap(long)]
    pub dot_pyre_directory: Option<String>,
    #[clap(long)]
    pub isolation_prefix: Option<String>,
    #[clap(long)]
    pub python_version: Option<String>,
    #[clap(long)]
    pub shared_memory_heap_size: Option<i32>,
    #[clap(long)]
    pub shared_memory_dependency_table_power: Option<i32>,
    #[clap(long)]
    pub shared_memory_hash_table_power: Option<i32>,
    #[clap(long)]
    pub number_of_workers: Option<i32>,
    #[clap(long)]
    pub enable_hover: Option<bool>,
    #[clap(long)]
    pub enable_go_to_definition: Option<bool>,
    #[clap(long)]
    pub enable_find_symbols: Option<bool>,
    #[clap(long)]
    pub use_buck2: Option<bool>,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug)]
pub struct AnalysisArgs;

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Set debug mode
    #[clap(long)]
    debug: bool,
    #[clap(long)]
    enable_memory_profiling: bool,
    #[clap(long)]
    enable_profiling: bool,
    log_identifier: Option<String>,
    logging_sections: Option<String>,
    #[clap(long)]
    noninteractive: bool,
    #[clap(long)]
    output: Option<String>,
    #[clap(long)]
    sequential: bool,
    #[clap(long)]
    show_error_traces: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Analysis(AnalysisArgs),
    /// Runs check stuff
    Check(CheckArgs),
}

pub fn get_args() -> CommandArguments {
    CommandArguments::parse()
}
