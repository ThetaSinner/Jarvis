use std::fmt::Formatter;
use std::fmt;
use std::error::Error;
use futures::Future;
use async_trait::async_trait;

pub mod docker_runtime;
pub mod k8s_runtime;

#[async_trait]
pub trait BuildRuntime {
    fn test(&self);

    fn connect(&mut self);

    async fn init_for_module(&mut self, module_name: &String) -> Result<(), BuildRuntimeError>;
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

