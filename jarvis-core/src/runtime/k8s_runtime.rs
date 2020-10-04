use crate::runtime::{BuildRuntime, BuildRuntimeError};
use async_trait::async_trait;
use crate::config::{Agent, ProjectConfig, ArchiveRule, ShellConfig, PluginSpecification};

pub struct KubernetesRuntime {

}

#[async_trait]
impl BuildRuntime for KubernetesRuntime {
    fn connect(&mut self) {
        unimplemented!()
    }

    async fn init_for_module(&mut self, _module_name: &String, _project_config: &ProjectConfig) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn create_agent(&mut self, _module_name: &String, _agent: &Agent, _secrets: &Option<Vec<String>>) -> Result<String, BuildRuntimeError> {
        unimplemented!()
    }

    async fn execute_command(&mut self, _agent_id: &str, _shell_config: &ShellConfig, _command: &str) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn get_archive(&mut self, _agent_id: &str, _archive_rule: &ArchiveRule) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn destroy_agent(&mut self, _agent_id: &str) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn tear_down_for_module(&self, _module_name: &String) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn cleanup_resources(&self) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn ensure_plugins_loaded(&mut self, _plugins: Vec<&PluginSpecification>) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }
}
