use std::{fs, fmt};
use std::path::PathBuf;
use std::fs::read_to_string;
use crate::config;
use serde::{Serialize, Deserialize};
use std::error::Error;
use serde::export::Formatter;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,

    pub default: Option<bool>,

    pub image: String,

    pub environment: Option<HashMap<String, String>>,

    pub cache: Option<Vec<CacheRule>>,

    pub container: Option<ContainerConfiguration>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CacheRule {
    pub name: String,

    pub location: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub name: String,

    pub command: String,

    pub agent: Option<String>,

    pub secrets: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,

    pub path: Option<String>,

    pub agents: Option<Vec<Agent>>,

    pub steps: Vec<Step>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
    pub api_version: f32,

    pub modules: Vec<Module>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ContainerConfiguration {
    pub user: Option<String>,

    pub group: Option<String>,

    pub privileged: Option<bool>,
}

pub struct ProjectConfig {
    pub project_directory: PathBuf,

    pub jarvis_directory: PathBuf,

    pub build_config: BuildConfig,
}

#[derive(Debug, Clone)]
pub struct ConfigError {
    pub(crate) msg: String
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "config error: {}", self.msg)
    }
}

impl Error for ConfigError {}

pub fn get_project_config(project_directory: std::path::PathBuf) -> Result<ProjectConfig, ConfigError> {
    let project_dir = if let Some(project_dir) = find_project_dir(&project_directory) {
        project_dir
    } else {
        return Err(ConfigError { msg: "No .jarvis directory found in the project root".to_string() });
    };

    let build_file = project_dir.join("build.yaml");
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
        jarvis_directory: project_dir,
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
