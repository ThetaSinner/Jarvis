use crate::runtime::{BuildRuntime, BuildRuntimeError};
use std::collections::HashMap;
use bollard::Docker;
use bollard::volume::{CreateVolumeOptions, RemoveVolumeOptions, ListVolumesOptions};
use async_trait::async_trait;

use crate::config::{Agent, ProjectConfig};
use bollard::container::{CreateContainerOptions, Config, StartContainerOptions, UploadToContainerOptions, RemoveContainerOptions, ListContainersOptions};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
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
use chrono::Utc;
use path_absolutize::Absolutize;
use regex::Regex;

pub struct DockerRuntime {
    docker: Option<Docker>,

    module_components: HashMap<String, Box<ModuleComponents>>,
}

struct ModuleComponents {
    build_data_volume: String,

    containers: HashMap<String, String>,

    jarvis_directory: PathBuf,
}

impl DockerRuntime {
    pub fn new() -> Self {
        DockerRuntime {
            docker: None,
            module_components: HashMap::new(),
        }
    }

    async fn create_docker_volume(&self, id: &str) -> Result<String, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let time = Utc::now().to_rfc3339();
            let mut labels = HashMap::new();
            labels.insert("created-by", "jarvis");
            labels.insert("build-time", time.as_str());

            let volume_result = docker.create_volume(CreateVolumeOptions {
                name: id,
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
        // TODO validate that a tag is provided, otherwise this will pull all tags for a repository.

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

    async fn create_container(&self,
                              module_component: &str,
                              name: &str,
                              agent: &Agent,
                              secrets_config: Vec<(String, String, String)>
    ) -> Result<String, BuildRuntimeError> {
        let mut environment: Option<Vec<String>> = None;
        if let Some(ref env) = agent.environment {
            let env_list = env.keys().map(|key| format!("{}={}", key, env[key])).collect();
            environment = Some(env_list);
        }

        if let Some(ref docker) = self.docker {
            let time = Utc::now().to_rfc3339();
            let mut labels = HashMap::new();
            labels.insert("created-by".to_string(), "jarvis".to_string());
            labels.insert("build-time".to_string(), time);

            let data_volume = self.module_components.get(module_component).unwrap().build_data_volume.as_str();

            let mut mounts = vec![Mount {
                target: Some("/build/workspace".to_string()),
                source: Some(data_volume.clone().to_string()),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            }];

            for secret_config in secrets_config {
                mounts.push(Mount {
                    target: Some(secret_config.2.clone()),
                    source: Some(format!("{}", secret_config.1)),
                    typ: Some(MountTypeEnum::BIND),
                    ..Default::default()
                });

                if let Some(environment) = &mut environment {
                    environment.push(format!("{}_FILE={}", secret_config.0, secret_config.2));
                } else {
                    environment = Some(vec![format!("{}_FILE={}", secret_config.0, secret_config.2)]);
                }
            }

            let mut user_config = None;
            let mut privileged = false;
            if let Some(container) = &agent.container {
                privileged = container.privileged.unwrap_or(false);

                if let Some(user) = &container.user {
                    if let Some(group) = &container.group {
                        user_config = Some(format!("{}:{}", user, group));
                    } else {
                        user_config = Some(user.to_string());
                    }
                }
            }

            let command_config = vec!["/bin/sh", "-c", "tail -f /dev/null"].iter().map(|x| x.to_string()).collect();

            let container_result = docker.create_container(Some(CreateContainerOptions { name }), Config {
                image: Some(agent.image.clone()),
                entrypoint: Some(command_config),
                cmd: Some(vec![]),
                env: environment,
                labels: Some(labels),
                working_dir: Some("/build/workspace".to_string()),
                user: user_config,
                host_config: Some(HostConfig {
                    mounts: Some(mounts),
                    privileged: Some(privileged),
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
                // TODO exclude .jarvis/secrets directory from the upload to prevent leaking secrets
                let tar_gz = File::create(&bundle_path).unwrap();
                let enc = GzEncoder::new(tar_gz, Compression::default());
                let mut tar = tar::Builder::new(enc);
                tar.append_dir_all(".", project_directory).unwrap();
            }

            let mut file = File::open(&bundle_path).unwrap();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            let options = Some(UploadToContainerOptions {
                path: "/build/workspace",
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
                working_dir: Some("/build/workspace"),
                ..Default::default()
            }).await
                .map(|exec| exec.id)
                .map_err(|e| BuildRuntimeError { msg: format!("Failed to create exec: {}", e) })
                ?;

            let mut exec = docker.start_exec(&exec_id, None::<StartExecOptions>);

            while let Some(exec_result) = exec.next().await {
                match exec_result {
                    Ok(result) => {
                        match result {
                            StartExecResults::Attached { log } => {
                                println!("{}", log);
                            }
                            StartExecResults::Detached => {
                                println!("Not attached");
                                // Do nothing
                            }
                        }
                    },
                    Err(e) => {
                        return Err(BuildRuntimeError { msg: format!("Error running exec: {}", e) });
                    }
                }
            }

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

    async fn delete_container(&self, agent_id: &str) -> Result<(), BuildRuntimeError> {
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

    async fn find_volumes(&self) -> Result<Vec<(String, HashMap<String, String>)>, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut filters= HashMap::new();
            filters.insert("dangling", vec!["true"]);
            filters.insert("label", vec!["created-by=jarvis"]);

            docker.list_volumes(Some(ListVolumesOptions { filters })).await
                .map(|results| {
                    if let Some(warnings) = results.warnings {
                        for warning in warnings {
                            println!("Warning during list volumes: {}", warning);
                        }
                    }

                    results.volumes.iter().map(|x| {
                        let labels_copy = if let Some(labels) = &x.labels {
                            labels.clone()
                        } else { HashMap::new() };

                        (x.name.clone(), labels_copy)
                    }).collect()
                })
                .map_err(|e| BuildRuntimeError { msg : format!("Failed to list volumes {}", e) })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn find_containers(&self) -> Result<Vec<(String, Vec<String>, HashMap<String, String>)>, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut filters= HashMap::new();
            filters.insert("label", vec!["created-by=jarvis"]);

            docker.list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            })).await
                .map(|results| {
                    results.iter().map(|x| {
                        let labels_copy = if let Some(labels) = &x.labels {
                            labels.clone()
                        } else { HashMap::new() };

                        let names_copy = if let Some(names) = &x.names {
                            names.clone()
                        } else { vec![] };

                        let id_copy = if let Some(id) = &x.id {
                            id.clone()
                        } else { "".to_string() };

                        (id_copy, names_copy, labels_copy)
                    }).collect()
                })
                .map_err(|e| BuildRuntimeError { msg : format!("Failed to list volumes {}", e) })
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
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .collect();
        let volume_name = format!("build-data-volume_{}_{}", module_name, id);
        let module_components = ModuleComponents {
            jarvis_directory: project_config.jarvis_directory.clone(),
            // TODO rename to workspace volume
            build_data_volume: volume_name.clone(),
            containers: HashMap::new(),
        };

        self.module_components.insert(module_name.to_string(), Box::new(module_components));

        self.create_docker_volume(volume_name.as_str()).await
            .map(|_| { () })?;

        let init_agent = self.create_agent(module_name, &Agent {
            name: "jarvis-init".to_string(),
            default: None,
            image: "alpine:latest".to_string(),
            environment: None,
            container: None,
        }, &None::<Vec<String>>).await?;

        self.upload_project(init_agent.as_str(), &project_config.project_directory).await?;

        self.delete_container(init_agent.as_str()).await
    }

    async fn create_agent(&mut self, module_name: &String, agent: &Agent, secrets: &Option<Vec<String>>) -> Result<String, BuildRuntimeError> {
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .collect();
        let name = format!("jarvis-agent-{}-{}-{}", module_name, agent.name, id);

        self.pull_image(agent.image.as_str()).await?;

        let secrets_config = configure_secrets(&self.module_components.get(module_name).unwrap().jarvis_directory, secrets)?;

        self.create_container(module_name, name.as_str(), agent, secrets_config).await
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

    async fn cleanup_resources(&self) -> Result<(), BuildRuntimeError> {
        let containers = self.find_containers().await?;
        if !containers.is_empty() {
            for container in containers {
                println!("Cleanup for container: {}", container.0);
                for name in container.1 {
                    println!("name: {}", name);
                }
                for label in container.2 {
                    println!("label: {}={}", label.0, label.1);
                }
                self.delete_container(container.0.as_str()).await?;
                println!();
            }
        } else {
            println!("No unused, Jarvis owned containers were found.");
        }

        let volumes = self.find_volumes().await?;
        if !volumes.is_empty() {
            for volume in volumes {
                println!("Cleanup for volume: {}", volume.0);
                for label in volume.1 {
                    println!("label: {}={}", label.0, label.1);
                }
                self.delete_volume(volume.0.as_str()).await?;
                println!();
            }
        } else {
            println!("No unused, Jarvis owned volumes were found.");
        }

        Ok(())
    }
}

fn configure_secrets(jarvis_directory: &PathBuf, secrets: &Option<Vec<String>>) -> Result<Vec<(String, String, String)>, BuildRuntimeError> {
    let mut secret_mounts = Vec::<(String, String, String)>::new();
    if let Some(secrets) = secrets {
        for secret in secrets {
            let secret_file = jarvis_directory.join("secrets").join(format!("{}.secret.txt", secret));

            if !secret_file.exists() {
                return Err(BuildRuntimeError { msg: format!("Secret [{}] not found at [{}]", secret, secret_file.to_str().unwrap()) });
            }

            let target_path = secret_file.absolutize().unwrap().to_str().unwrap().to_owned();
            secret_mounts.push((
                to_environment_variable_name(secret),
                target_path,
                format!("/build/secrets/{}", secret)
            ));
        }
    }

    Ok(secret_mounts)
}

fn to_environment_variable_name(source: &str) -> String {
    let pattern = Regex::new(r"(?P<l>.*)[^a-zA-Z0-9_](?P<r>.*)").unwrap();

    // Loop required to deal with overlapping matches, would a different regex help?

    let mut last = "".to_owned();
    let mut next = source.to_owned();
    while last != next {
        last = next.clone();
        next = pattern.replace_all(next.as_str(), "${l}_$r").into_owned();
    }

    next.to_ascii_uppercase()
}
