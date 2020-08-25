use crate::config::ConfigError;
use crate::runtime::BuildRuntime;
use crate::runtime::docker_runtime::DockerRuntime;
use crate::runtime::k8s_runtime::KubernetesRuntime;
use futures_util::core_reexport::fmt::Formatter;
use std::fmt;
use crate::build::BuildError;

mod runtime;
mod validate;
pub mod config;
mod build;

pub trait OutputFormatter {
    fn print(&self, msg: String);

    fn success(&self, msg: String);

    fn error(&self, msg: String);

    fn background(&self, msg: String);
}

pub async fn build_project(project_path: std::path::PathBuf, runtime: RuntimeOption, output_formatter: &Box<dyn OutputFormatter>) -> Result<(), BuildError> {
    let runtime: Box<dyn BuildRuntime> = match runtime {
        RuntimeOption::Docker => Box::new(DockerRuntime::new() ),
        RuntimeOption::Kubernetes => Box::new(KubernetesRuntime {}),
        RuntimeOption::None => Box::new(DockerRuntime::new() )
    };

    build::build_project(project_path, runtime, output_formatter).await
}

pub fn validate_project(project_path: std::path::PathBuf) -> Result<validate::ValidationMessages, validate::ValidationError> {
    return validate::validate_project(project_path);
}

pub enum RuntimeOption {
    Docker,
    Kubernetes,
    None,
}

impl std::str::FromStr for RuntimeOption {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "docker" => Ok(RuntimeOption::Docker),
            "kubernetes" => Ok(RuntimeOption::Kubernetes),
            "k8s" => Ok(RuntimeOption::Kubernetes),
            _ => Ok(RuntimeOption::None)
        }
    }
}

impl std::fmt::Display for RuntimeOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeOption::Docker => write!(f, "Docker runtime"),
            RuntimeOption::Kubernetes => write!(f, "Kubernetes runtime"),
            RuntimeOption::None => write!(f, "No runtime")
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
