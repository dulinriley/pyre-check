use std::fmt::{Display, Formatter};

pub struct PythonVersion {
    major: i32,
    minor: i32,
    micro: i32,
}

struct InvalidPythonVersionError {
    msg: String,
}

impl From<std::num::ParseIntError> for InvalidPythonVersionError {
    fn from(e: std::num::ParseIntError) -> Self {
        InvalidPythonVersionError {
            msg: "Int parse error".to_string(),
        }
    }
}

impl PythonVersion {
    pub fn new(s: &str) -> Result<Self, InvalidPythonVersionError> {
        let splits = s.split(".").collect::<Vec<_>>();
        match splits.len() {
            1 => Ok(PythonVersion {
                major: splits[0].parse::<i32>()?,
                minor: 0,
                micro: 0,
            }),
            2 => Ok(PythonVersion {
                major: splits[0].parse::<i32>()?,
                minor: splits[1].parse::<i32>()?,
                micro: 0,
            }),
            3 => Ok(PythonVersion {
                major: splits[0].parse::<i32>()?,
                minor: splits[1].parse::<i32>()?,
                micro: splits[2].parse::<i32>()?,
            }),
            _ => Err(InvalidPythonVersionError {
                msg: "Version string is expected to have the form of 'X.Y.Z' but got ".to_string()
                    + s,
            }),
        }
    }
}

impl Display for PythonVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}", self.major, self.minor, self.micro))
    }
}
