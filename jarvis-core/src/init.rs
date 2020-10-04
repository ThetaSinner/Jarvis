use std::fmt::Formatter;
use std::fmt;
use std::error::Error;
use crate::OutputFormatter;
use crate::runtime::BuildRuntime;
use crate::config::{get_project_config, ProjectConfig};

#[derive(Debug, Clone)]
pub struct InitError {
    msg: String
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "init project error: {}", self.msg)
    }
}

impl Error for InitError {}

pub async fn init_project(project_path: std::path::PathBuf, mut runtime: Box<dyn BuildRuntime>, output_formatter: &Box<dyn OutputFormatter>) -> Result<(), InitError> {
    let project_config = get_project_config(project_path)
        .map_err(|e| InitError { msg: format!("Project configuration error: {}", e) })?;

    runtime.connect();

    init_project_with_config(project_config, &mut runtime, output_formatter).await
}

async fn init_project_with_config(project_config: ProjectConfig, runtime: &mut Box<dyn BuildRuntime>, _output_formatter: &Box<dyn OutputFormatter>) -> Result<(), InitError> {
    let project_id = project_config.build_config.project_id.as_str();

    println!("project id {}", project_id);

    let mut all_plugins = vec![];
    for module in &project_config.build_config.modules {
        for step in &module.steps {
            if let Some(plugins) = &step.plugins {
                for plugin in plugins {
                    all_plugins.push(plugin)
                }
            }
        }
    }

    runtime.ensure_plugins_loaded(all_plugins).await.unwrap();

    Ok(())
}
