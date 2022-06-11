use serde::{Deserialize, Serialize};
use std::convert::From;
use std::process::{Command, Stdio};
use std::string::String;

#[derive(Debug)]
pub struct CheckError {
    pub msg: &'static str,
}

impl From<std::io::Error> for CheckError {
    fn from(_e: std::io::Error) -> Self {
        CheckError { msg: "I/O error" }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TypeError {
    line: i32,
    column: i32,
    stop_line: i32,
    stop_column: i32,
    path: String,
    code: i32,
    name: String,
    description: String,
    long_description: String,
    concise_description: String,
}

fn parse_type_error_response(val: &str) -> Vec<TypeError> {
    let type_errors: Vec<TypeError> =
        serde_json::from_str(val).expect("Cannot parse response as JSON");
    type_errors
}

fn display_type_errors(errors: &Vec<TypeError>) {
    for e in errors {
        println!("{:#?}", e);
    }
}

pub fn check_command(binary_location: &str, argument_file_path: &str) -> Result<(), CheckError> {
    let child = Command::new(binary_location)
        .arg("newcheck")
        .arg(argument_file_path)
        .stdout(Stdio::piped())
        .spawn()?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(CheckError {
            msg: "Command failed",
        });
    }
    println!("{:#?}", output.status);
    let stdout = String::from_utf8(output.stdout).expect("Cannot decode as utf-8");
    let type_errors = parse_type_error_response(&stdout);
    display_type_errors(&type_errors);
    if type_errors.len() == 0 {
        Ok(())
    } else {
        Err(CheckError {
            msg: "Had some type errors",
        })
    }
}
