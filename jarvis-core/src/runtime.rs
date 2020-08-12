pub mod docker_runtime;
pub mod k8s_runtime;

pub trait BuildRuntime {
    fn test(&self);
}
