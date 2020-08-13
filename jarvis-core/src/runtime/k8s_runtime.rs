use crate::runtime::{BuildRuntime, BuildRuntimeError};
use async_trait::async_trait;
use crate::config::Agent;

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

    async fn init_for_module(&mut self, module_name: &String) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }

    async fn create_agent(&mut self, module_name: &String, agent: &Agent) -> Result<(), BuildRuntimeError> {
        unimplemented!()
    }
}
