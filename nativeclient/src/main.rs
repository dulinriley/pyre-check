mod check;
mod command_arguments;
mod configuration;
mod filesystem;
mod find_directories;

use command_arguments::Commands;

fn main() {
    let args = command_arguments::get_args();
    let res = match &args.command {
        Commands::Analysis(_analysis_args) => Ok("analysis"),
        Commands::Check(_check_args) => {
            check::check_command("/bin/ls", "/tmp/foo.json").and_then(|()| Ok("check complete"))
        }
    };
    match res {
        Ok(s) => println!("No errors: {}", s),
        Err(e) => println!("{}", e.msg),
    }
}
