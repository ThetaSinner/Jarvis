use crate::runtime::{BuildRuntime, BuildRuntimeError};
use async_trait::async_trait;
use crate::config::{Agent, ProjectConfig};

pub struct KubernetesRuntime {

}

#[async_trait]
impl BuildRuntime for KubernetesRuntime {
    fn test(&self) {
        println!("I'm the kubernetes runtime");
    }

    fn connect(&mut self) {
        unimplemented!()
    }

    async fn init_for_module(&mut self, _module_name: &String, _project_config: &ProjectConfig) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn create_agent(&mut self, _module_name: &String, _agent: &Agent) -> Result<String, BuildRuntimeError> {
        unimplemented!()
    }

    async fn execute_command(&mut self, _agent_id: &str, _command: &str) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn destroy_agent(&mut self, _agent_id: &str) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn tear_down_for_module(&self, _module_name: &String) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }
}
