use std::fmt;

#[derive(Debug, Clone)]
pub struct RhizoCLIError {
    pub message: String
}

impl fmt::Display for RhizoCLIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.message)
    }
}

impl std::error::Error for RhizoCLIError { }

impl RhizoCLIError {
    pub fn new(msg: &str) -> RhizoCLIError { RhizoCLIError { message: msg.to_string() } } 
}

