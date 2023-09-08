use std::{error, fmt, result};

pub type Result<T> = result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct InvalidInput(pub String);

#[derive(Debug)]
pub struct InvalidArgument(pub String);

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid input: {}", self.0)
    }
}

impl fmt::Display for InvalidArgument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid argument: {}", self.0)
    }
}

impl error::Error for InvalidInput {}

impl error::Error for InvalidArgument {}

pub fn invalid_input(s: String) -> Box<dyn error::Error> {
    InvalidInput(s).into()
}

pub fn invalid_input_ref(s: &str) -> Box<dyn error::Error> {
    InvalidInput(s.to_owned()).into()
}

pub fn invalid_argument(s: String) -> Box<dyn error::Error> {
    InvalidArgument(s).into()
}

pub fn invalid_argument_ref(s: &str) -> Box<dyn error::Error> {
    InvalidArgument(s.to_owned()).into()
}
