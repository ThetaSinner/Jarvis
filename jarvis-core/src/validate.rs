use std::fmt;
use crate::config;
use std::error::Error;
use std::fmt::Formatter;
use crate::config::ProjectConfig;

#[derive(Debug, Clone)]
pub struct ValidationError {
    msg: String
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "config error: {}", self.msg)
    }
}

impl Error for ValidationError {}

pub struct ValidationMessages {
    pub errors: Vec<String>,

    pub warnings: Vec<String>
}

pub fn validate_project(project_path: std::path::PathBuf) -> Result<ValidationMessages, ValidationError> {
    let project_config = config::get_project_config(project_path);

    match project_config {
        Err(e) => Err(ValidationError { msg: format!("Could not load project config: {}", e).to_string() }),
        Ok(project_config) => Ok(validate_project_config(project_config))
    }
}

fn validate_project_config(project_config: ProjectConfig) -> ValidationMessages {
    let mut messages = ValidationMessages { errors: vec![], warnings: vec![] };

    if project_config.build_config.modules.is_empty() {
        messages.warnings.push("No build modules defined".to_string());
    }

    messages
}
