use crate::runtime::{BuildRuntime, BuildRuntimeError};
use std::collections::HashMap;
use crypto::sha2::Sha256;
use bollard::Docker;
use crypto::digest::Digest;
use bollard::volume::{CreateVolumeOptions, RemoveVolumeOptions};
use async_trait::async_trait;

use crate::config::{Agent, ProjectConfig};
use bollard::container::{CreateContainerOptions, Config, StartContainerOptions, UploadToContainerOptions, RemoveContainerOptions};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use futures::TryStreamExt;
use bollard::image::{CreateImageOptions, CreateImageResults};
use tokio::stream::StreamExt;
use std::{io, env};
use std::io::{Write, Read};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use std::path::PathBuf;
use std::fs::File;
use flate2::write::GzEncoder;
use flate2::Compression;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};

pub struct DockerRuntime {
    docker: Option<Docker>,

    module_components: HashMap<String, Box<ModuleComponents>>,
}

struct ModuleComponents {
    build_data_volume: String,

    containers: HashMap<String, String>,
}

impl DockerRuntime {
    pub fn new() -> Self {
        DockerRuntime {
            docker: None,
            module_components: HashMap::new(),
        }
    }

    async fn create_docker_volume(&self, id: &String) -> Result<String, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut labels = HashMap::new();
            labels.insert("created-by", "jarvis");

            let volume_result = docker.create_volume(CreateVolumeOptions {
                name: id.as_str(),
                driver: "local",
                driver_opts: Default::default(),
                labels,
            }).await;

            return volume_result
                .map(|x| x.name)
                .map_err(|e| BuildRuntimeError { msg: e.to_string() });
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn pull_image(&self, image: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut pull_results = docker.create_image(Some(CreateImageOptions {
                from_image: image,
                ..Default::default()
            }), None, None);

            print!("{}", ansi_escapes::CursorHide);
            let mut layer_id_line_numbers = HashMap::<String, usize>::new();
            while let Some(pull_result) = pull_results.next().await {
                match pull_result {
                    Ok(create_result) => {
                        match create_result {
                            CreateImageResults::CreateImageProgressResponse { status, progress_detail: _progress_detail, id, progress } => {
                                if let Some(layer_id) = id {
                                    if !layer_id_line_numbers.contains_key(layer_id.as_str()) {
                                        layer_id_line_numbers.insert(layer_id.clone(), layer_id_line_numbers.len() + 1);

                                        let msg = match status.as_str() {
                                            "Downloading" => progress.unwrap(),
                                            _ => status
                                        };

                                        match layer_id_line_numbers.len() {
                                            1 => print!("{} {}", layer_id, msg),
                                            _ => print!("\n{} {}", layer_id, msg)
                                        }
                                    } else {
                                        let msg = match status.as_str() {
                                            "Downloading" => progress.unwrap(),
                                            _ => status
                                        };

                                        let move_lines = layer_id_line_numbers.len() - layer_id_line_numbers.get(layer_id.as_str()).unwrap();
                                        match move_lines {
                                            0 => print!("\r{}{} {}", ansi_escapes::EraseEndLine, layer_id, msg),
                                            _ => print!("{}\r{}{} {}{}", ansi_escapes::CursorUp(move_lines as u16), ansi_escapes::EraseEndLine, layer_id, msg, ansi_escapes::CursorDown(move_lines as u16))
                                        }
                                    }
                                } else if let Some(progress_msg) = progress {
                                    // Relies on these kinds of messages being written last, after all the layers have been handled.
                                    print!("\n{}", progress_msg)
                                }

                                io::stdout().flush().unwrap();
                            }
                            CreateImageResults::CreateImageError { error_detail: _error_detail, error } => {
                                print!("\n{}", ansi_escapes::CursorShow);
                                println!("{}", error);
                                return Err(BuildRuntimeError { msg: format!("Image pull error: {}", error) });
                            }
                        }
                    }
                    Err(e) => {
                        print!("\n{}", ansi_escapes::CursorShow);
                        return Err(BuildRuntimeError { msg: format!("Image pull error: {}", e) });
                    }
                }
            }

            print!("\n{}", ansi_escapes::CursorShow);
            Ok(())
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn create_container(&self, module_component: &str, name: &str, agent: &Agent) -> Result<String, BuildRuntimeError> {
        let mut environment = None;
        if let Some(ref env) = agent.environment {
            let env_list = env.keys().map(|key| format!("{}={}", key, env[key])).collect();
            environment = Some(env_list);
        }

        if let Some(ref docker) = self.docker {
            let data_volume = self.module_components.get(module_component).unwrap().build_data_volume.as_str();

            let container_result = docker.create_container(Some(CreateContainerOptions { name }), Config {
                image: Some(agent.image.clone()),
                cmd: Some(vec!["/bin/sh", "-c", "tail -f /dev/null"].iter().map(|x| x.to_string()).collect()),
                env: environment,
                working_dir: Some("/build".to_string()),
                host_config: Some(HostConfig {
                    mounts: Some(vec![Mount {
                        target: Some("/build".to_string()),
                        source: Some(data_volume.clone().to_string()),
                        _type: Some(MountTypeEnum::VOLUME),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            }).await;

            container_result.map(|x| {
                for warning in x.warnings {
                    println!("docker container create warning: {}", warning);
                }
                x.id
            }).map_err(|e| BuildRuntimeError { msg: format!("Failed to create container: {}", e) })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn start_container(&self, name: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            docker.start_container(name, None::<StartContainerOptions<String>>).await
                .map_err(|e| {
                    BuildRuntimeError { msg: format!("Failed to start container: {}", e) }
                })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn upload_project(&self, container_id: &str, project_directory: &PathBuf) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let id: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(30)
                .collect();

            let mut bundle_path = env::temp_dir();
            bundle_path.push(format!("jarvis-bundle-{}.tgz", id));

            {
                let tar_gz = File::create(&bundle_path).unwrap();
                let enc = GzEncoder::new(tar_gz, Compression::default());
                let mut tar = tar::Builder::new(enc);
                tar.append_dir_all(".", project_directory).unwrap();
            }

            let mut file = File::open(&bundle_path).unwrap();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let options = Some(UploadToContainerOptions {
                path: "/build",
                ..Default::default()
            });

            let upload_result = docker.upload_to_container(container_id, options, contents.into()).await;

            std::fs::remove_file(bundle_path).unwrap();

            upload_result.map_err(|e| {
                BuildRuntimeError { msg: format!("Error uploading build bundle: {}", e) }
            })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn execute_command_internal(&mut self, agent_id: &str, command: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let exec_id = docker.create_exec(agent_id, CreateExecOptions {
                cmd: Some(vec!["/bin/sh", "-c", command]),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                working_dir: Some("/build"),
                ..Default::default()
            }).await
                .map(|exec| exec.id)
                .map_err(|e| BuildRuntimeError { msg: format!("Failed to create exec: {}", e) })
                ?;

            docker.start_exec(&exec_id, None::<StartExecOptions>).try_collect::<Vec<_>>().await
                .map(|results| {
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
                })
                .map_err(|e| BuildRuntimeError { msg: format!("Error running command: {}", e) })?;

            docker.inspect_exec(&exec_id).await
                .map_err(|e| {
                    BuildRuntimeError { msg: format!("Failed to check command status {}", e) }
                })
                .and_then(|result| {
                    if result.running {
                        return Err(BuildRuntimeError { msg: "Command has not exited.".to_string() });
                    }

                    if result.exit_code != Some(0) {
                        return Err(BuildRuntimeError { msg: format!("Command has non-zero exit status [{}]", result.exit_code.unwrap()) });
                    }

                    Ok(())
                })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn delete_container(&mut self, agent_id: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let options = Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            });

            docker.remove_container(agent_id, options).await
                .map_err(|e| BuildRuntimeError { msg: format!("Failed to remove container [{}]: {}", agent_id, e) })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn delete_volume(&self, volume_id: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let options = RemoveVolumeOptions {
                force: false,
            };

            docker.remove_volume(volume_id, Some(options)).await
                .map_err(|e| BuildRuntimeError { msg: format!("Error removing volume [{}]: {}", volume_id, e) })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }
}

#[async_trait]
impl BuildRuntime for DockerRuntime {
    fn connect(&mut self) {
        self.docker = Some(Docker::connect_with_local_defaults().unwrap())
    }

    async fn init_for_module(&mut self, module_name: &String, project_config: &ProjectConfig) -> Result<(), BuildRuntimeError> {
        let mut hasher = Sha256::new();
        hasher.input_str("build_data_volume-");
        hasher.input_str(module_name);
        let module_components = ModuleComponents {
            build_data_volume: hasher.result_str(),
            containers: HashMap::new(),
        };

        self.module_components.insert(module_name.to_string(), Box::new(module_components));

        self.create_docker_volume(&hasher.result_str()).await
            .map(|_| { () })?;

        let init_agent = self.create_agent(module_name, &Agent {
            name: "jarvis-init".to_string(),
            default: None,
            image: "alpine:latest".to_string(),
            environment: None,
        }).await?;

        self.upload_project(init_agent.as_str(), &project_config.project_directory).await?;

        self.delete_container(init_agent.as_str()).await
    }

    async fn create_agent(&mut self, module_name: &String, agent: &Agent) -> Result<String, BuildRuntimeError> {
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .collect();
        let name = format!("jarvis-agent-{}-{}-{}", module_name, agent.name, id);

        self.pull_image(agent.image.as_str()).await?;

        self.create_container(module_name, name.as_str(), agent).await
            .map(|x| {
                let component: &mut Box<ModuleComponents> = self.module_components.get_mut(module_name).unwrap();
                component.containers.insert(agent.name.clone(), x);
                ()
            })?;

        self.start_container(self.module_components.get(module_name).unwrap().containers.get(agent.name.as_str()).unwrap().as_str()).await?;

        Ok(name.clone())
    }

    async fn execute_command(&mut self, agent_id: &str, command: &str) -> Result<(), BuildRuntimeError> {
        self.execute_command_internal(agent_id, command).await
    }

    async fn destroy_agent(&mut self, agent_id: &str) -> Result<(), BuildRuntimeError> {
        self.delete_container(agent_id).await
    }

    async fn tear_down_for_module(&self, module_name: &String) -> Result<(), BuildRuntimeError> {
        self.delete_volume(self.module_components.get(module_name).unwrap().build_data_volume.as_str()).await
    }
}
