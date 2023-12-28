use std::fmt;

pub struct AppError {
    details: String,
}

impl AppError {
    pub fn new(msg: &str) -> AppError {
        AppError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AppError {{ {} }}", self.details)
    }
}