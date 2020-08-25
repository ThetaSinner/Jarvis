use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use crate::runtime::BuildRuntime;
use crate::OutputFormatter;

#[derive(Debug, Clone)]
pub struct CleanupError {
    msg: String
}

impl fmt::Display for CleanupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "cleanup error: {}", self.msg)
    }
}

impl Error for CleanupError {}

pub async fn cleanup_resources(mut runtime: Box<dyn BuildRuntime>, output_formatter: &Box<dyn OutputFormatter>) -> Result<(), CleanupError> {
    runtime.connect();

    output_formatter.print("Starting cleanup".to_string());

    runtime.cleanup_resources().await
        .map_err(|e| CleanupError { msg: format!("{}", e) } )?;

    Ok(())
}