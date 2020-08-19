use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, UploadToContainerOptions};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::volume::CreateVolumeOptions;
use flate2::Compression;
use flate2::write::GzEncoder;
use futures_util::stream::TryStreamExt;
use tokio::runtime::Runtime;

use crate::config::ConfigError;
use crate::runtime::BuildRuntime;
use crate::runtime::docker_runtime::DockerRuntime;
use crate::runtime::k8s_runtime::KubernetesRuntime;
use futures_util::core_reexport::fmt::Formatter;
use std::fmt;

mod runtime;
mod validate;
pub mod config;
mod build;
mod docker_image_name;

pub async fn build_project(project_path: std::path::PathBuf, runtime: RuntimeOption) -> Option<ConfigError> {
    let runtime: Box<dyn BuildRuntime> = match runtime {
        RuntimeOption::Docker => Box::new(DockerRuntime::new() ),
        RuntimeOption::Kubernetes => Box::new(KubernetesRuntime {}),
        RuntimeOption::None => Box::new(DockerRuntime::new() )
    };
    return build::build_project(project_path, runtime).await;
}

pub fn validate_project(project_path: std::path::PathBuf) -> Result<validate::ValidationMessages, validate::ValidationError> {
    return validate::validate_project(project_path);
}

fn runtime_thing(runtime: &impl BuildRuntime) {
    runtime.test();
}

pub async fn docker_things(project_path: std::path::PathBuf) {
    let mut rt = Runtime::new().unwrap();

    let runtime = KubernetesRuntime {};
    runtime_thing(&runtime);

    let docker = Docker::connect_with_local_defaults().unwrap();

    let future = async move {
        let version = docker.version().await.unwrap();
        println!("{:?}", version);

        let mut labels = HashMap::new();
        labels.insert("testing", "docker");

        let volume_result = docker.create_volume(CreateVolumeOptions {
            name: "abc",
            driver: "local",
            driver_opts: Default::default(),
            labels,
        }).await;

        match volume_result {
            Ok(volume) => println!("Volume created {}", volume.created_at),
            Err(e) => println!("{}", e)
        }

        let container_result = docker.create_container(Some(CreateContainerOptions {
            name: "abc"
        }), Config {
            image: Some("alpine:latest"),
            cmd: Some(vec!["/bin/sh", "-c", "sleep 3600"]),
            ..Default::default()
        }).await;

        let container_id = match container_result {
            Ok(result) => {
                println!("{}", result.id);
                result.id
            }
            Err(e) => {
                print!("{}", e);
                return Err("Failed to create container");
            }
        };

        let start_result = docker.start_container(container_id.as_str(), None::<StartContainerOptions<String>>).await;
        match start_result {
            Ok(_) => println!("Container started."),
            Err(e) => println!("Failed to start container {}", e)
        };


        {
            std::fs::remove_file("tarball.tar.gz");
            let tar_gz = File::create("tarball.tar.gz").unwrap();
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(".", project_path).unwrap();
        }

        let mut file = File::open("tarball.tar.gz").unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let options = Some(UploadToContainerOptions {
            path: "/opt",
            ..Default::default()
        });

        let upload_result = docker.upload_to_container(container_id.as_str(), options, contents.into()).await;

        match upload_result {
            Ok(_) => println!("Uploaded successfully."),
            Err(e) => println!("Failed to upload: {}", e)
        };

        let exec_result = docker.create_exec(container_id.as_str(), CreateExecOptions {
            cmd: Some(vec!["/bin/sh", "-c", "touch lib.txt; ls -al"]),
            attach_stdout: Some(true),
            working_dir: Some("/opt"),
            ..Default::default()
        }).await;

        let exec_id = match exec_result {
            Ok(result) => {
                println!("Created exec {}", result.id);
                result.id
            }
            Err(e) => {
                println!("Failed to create exec {}", e);
                return Err("Failed to create exec");
            }
        };

        let run_exec_result = &docker.start_exec(&exec_id, None::<StartExecOptions>).try_collect::<Vec<_>>().await;

        match run_exec_result {
            Ok(results) => {
                println!("Exec is okay");
                for result in results {
                    match result {
                        StartExecResults::Attached { log } => {
                            println!("{}", log);
                        }
                        StartExecResults::Detached => {
                            println!("Not attached");
                            // Do nothing
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to start exec {}", e)
            }
        }

        Ok(1)
    };

    rt.block_on(future);
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
