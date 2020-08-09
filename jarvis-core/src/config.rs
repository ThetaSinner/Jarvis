use std::{fs, fmt};
use std::path::PathBuf;
use std::fs::read_to_string;
use crate::config;
use serde::{Serialize, Deserialize};
use std::error::Error;
use serde::export::Formatter;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
    pub api_version: f32,

    pub modules: Vec<Module>,
}

pub struct ProjectConfig {
    pub project_directory: PathBuf,

    pub build_config: BuildConfig
}

#[derive(Debug, Clone)]
pub struct ConfigError {
    msg: String
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "config error: {}", self.msg)
    }
}

impl Error for ConfigError {}

pub fn get_project_config(project_directory: std::path::PathBuf) -> Result<ProjectConfig, ConfigError> {
    let project_dir = find_project_dir(&project_directory);
    if project_dir.is_none() {
        return Err(ConfigError { msg: "No .jarvis directory found in the project root".to_string() });
    }

    let build_file = project_dir.unwrap().join("build.yaml");
    if !build_file.exists() {
        return Err(ConfigError { msg: "build.yaml file not found in .jarvis directory".to_string() });
    }

    let build_config_string = read_to_string(build_file);
    if !build_config_string.is_ok() {
        return Err(ConfigError { msg: "Cannot read build.yaml".to_string() });
    }

    let build_config_result: serde_yaml::Result<config::BuildConfig> = serde_yaml::from_str(build_config_string.unwrap().as_str());
    if !build_config_result.is_ok() {
        return Err(ConfigError { msg: format!("build.yaml is not valid: {}", build_config_result.err().unwrap().to_string()) })
    }

    let build_config = build_config_result.unwrap();

    if build_config.api_version < 0.1 {
        return Err(ConfigError { msg: format!("Invalid value for api_version, should be >0.1 but was {}", build_config.api_version) })
    }

    return Ok(ProjectConfig {
        project_directory,
        build_config
    });
}

fn find_project_dir(project_path: &std::path::PathBuf) -> Option<PathBuf> {
    let dir = fs::read_dir(project_path);
    match dir {
        Ok(paths) => {
            for path in paths {
                if path.is_ok() {
                    let entry = path.unwrap();
                    let entry_type = entry.file_type();
                    if entry_type.is_ok() && entry_type.unwrap().is_dir() && ".jarvis" == entry.file_name() {
                        return Option::Some(entry.path())
                    }
                }
            }
        },
        Err(_error) => {
            return Option::None;
        }
    }

    return Option::None;
}

