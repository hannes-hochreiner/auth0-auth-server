use std::fmt;
use std::error::Error;
use std::env::VarError;

#[derive(Debug)]
pub enum AuthServerError {
    EnvVarError(String, VarError),
}

impl fmt::Display for AuthServerError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            AuthServerError::EnvVarError(s, e) => write!(fmt, "environment variable error \"{}\": {}", s, e),
        }
    }
}

impl Error for AuthServerError {}

impl From<(&str, VarError)> for AuthServerError {
    fn from(t: (&str, VarError)) -> Self {
        let e = AuthServerError::EnvVarError(String::from(t.0), t.1);
        error!("{}", e);
        e
    }
}
