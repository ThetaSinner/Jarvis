use std::fmt;
use std::fmt::Formatter;
use std::error::Error;

#[derive(Debug)]
pub struct PluginError {
    pub msg: String,
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "plugin error: {}", self.msg)
    }
}

impl Error for PluginError {}
