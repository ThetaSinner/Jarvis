use std::fmt::Formatter;
use std::fmt;
use std::error::Error;
use async_trait::async_trait;
use crate::config::{Agent, ProjectConfig, ArchiveRule, ShellConfig, PluginSpecification};

pub mod docker_runtime;
pub mod k8s_runtime;

#[async_trait]
pub trait BuildRuntime {
    fn connect(&mut self);

    async fn init_for_module(&mut self, module_name: &String, project_config: &ProjectConfig) -> Result<(), BuildRuntimeError>;

    async fn create_agent(&mut self, module_name: &String, agent: &Agent, secrets: &Option<Vec<String>>) -> Result<String, BuildRuntimeError>;

    async fn execute_command(&mut self, agent_id: &str, shell_config: &ShellConfig, command: &str) -> Result<(), BuildRuntimeError>;

    async fn get_archive(&mut self, agent_id: &str, archive_rule: &ArchiveRule) -> Result<(), BuildRuntimeError>;

    async fn destroy_agent(&mut self, agent_id: &str) -> Result<(), BuildRuntimeError>;

    async fn tear_down_for_module(&self, module_name: &String) -> Result<(), BuildRuntimeError>;

    async fn cleanup_resources(&self) -> Result<(), BuildRuntimeError>;

    async fn ensure_plugins_loaded(&mut self, plugins: Vec<&PluginSpecification>) -> Result<(), BuildRuntimeError>;
}

#[derive(Debug, Clone)]
pub struct BuildRuntimeError {
    msg: String
}

impl fmt::Display for BuildRuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "build runtime error: {}", self.msg)
    }
}

impl Error for BuildRuntimeError {}

