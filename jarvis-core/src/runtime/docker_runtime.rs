use crate::runtime::{BuildRuntime, BuildRuntimeError};
use std::collections::HashMap;
use bollard::Docker;
use bollard::volume::{CreateVolumeOptions, RemoveVolumeOptions, ListVolumesOptions};
use async_trait::async_trait;

use crate::config::{Agent, ProjectConfig, CacheRule, ArchiveRule, ShellConfig, PluginSpecification, Step};
use bollard::container::{CreateContainerOptions, Config, StartContainerOptions, UploadToContainerOptions, RemoveContainerOptions, ListContainersOptions, DownloadFromContainerOptions};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use bollard::image::{CreateImageOptions, ListImagesOptions};
use tokio::stream::StreamExt;
use std::{io, env};
use std::io::{Write, Read};
use bollard::models::{HostConfig, Mount, MountTypeEnum, PortMap, PortBinding};
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

    identifier_base: String,

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

    async fn create_docker_volume(&self, id: &str, extra_labels: Option<HashMap<String, String>>) -> Result<String, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let time = Utc::now().to_rfc3339();
            let mut labels = HashMap::new();
            labels.insert("created-by", "jarvis");
            labels.insert("build-time", time.as_str());
            if let Some(extras) = &extra_labels {
                for extra in extras {
                    labels.insert(extra.0, extra.1);
                }
            }

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

    async fn image_available(&self, image: &str) -> Result<bool, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut filters = HashMap::new();
            filters.insert("reference", vec![image]);

            docker.list_images(Some(ListImagesOptions {
                filters,
                ..Default::default()
            })).await
                .map_err(|e| {
                    BuildRuntimeError { msg: format!("{}", format_docker_api_error(e)) }
                })
                .map(|r| {
                    r.len() > 0
                })
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
                        if let Some(err) = create_result.error {
                            print!("\n{}", ansi_escapes::CursorShow);
                            println!("{}", err);
                            return Err(BuildRuntimeError { msg: format!("Image pull error: {}", err) });
                        } else {
                            let status = create_result.status.unwrap();
                            let progress = create_result.progress;

                            if let Some(layer_id) = create_result.id {
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
                              secrets_config: Vec<(String, String, String)>,
                              using_plugins: bool
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

            if let Some(cache_list) = &agent.cache {
                let cache_volumes = ensure_caches_created(&self, cache_list, self.module_components.get(module_component).unwrap().identifier_base.as_str()).await?;

                for cache in cache_list {
                    mounts.push(Mount {
                        target: Some(cache.location.clone()),
                        source: Some(cache_volumes.get(cache.name.as_str()).unwrap().clone()),
                        typ: Some(MountTypeEnum::VOLUME),
                        ..Default::default()
                    });
                }
            }

            if using_plugins {
                mounts.push(Mount {
                    target: Some("/build/agent".to_string()),
                    source: Some("jarvis-plugins".to_string()),
                    typ: Some(MountTypeEnum::VOLUME),
                    ..Default::default()
                });
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

            let port_config = if using_plugins {
                let mut ports = HashMap::new();
                ports.insert("1438/tcp".to_string(), HashMap::new());

                let mut port_bindings = HashMap::new() as PortMap;
                port_bindings.insert("1438/tcp".to_string(), Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: Some("1438".to_string())
                }]));

                (Some(ports), Some(port_bindings))
            } else {
                (None, None)
            };

            let container_result = docker.create_container(Some(CreateContainerOptions { name }), Config {
                image: Some(agent.image.clone()),
                entrypoint: Some(command_config),
                cmd: Some(vec![]),
                env: environment,
                tty: Some(true),
                attach_stdin: Some(true),
                attach_stderr: Some(true),
                attach_stdout: Some(true),
                labels: Some(labels),
                working_dir: Some("/build/workspace".to_string()),
                user: user_config,
                exposed_ports: port_config.0,
                host_config: Some(HostConfig {
                    mounts: Some(mounts),
                    privileged: Some(privileged),
                    port_bindings: port_config.1,
                    ..Default::default()
                }),
                ..Default::default()
            }).await;

            container_result.map(|x| {
                for warning in x.warnings {
                    println!("docker container create warning: {}", warning);
                }
                x.id
            }).map_err(|e| BuildRuntimeError { msg: format!("Failed to create container: {}", format_docker_api_error(e)) })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn start_container(&self, name: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            docker.start_container(name, None::<StartContainerOptions<String>>).await
                .map_err(|e| {
                    BuildRuntimeError { msg: format!("Failed to start container: {}", format_docker_api_error(e)) }
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
                BuildRuntimeError { msg: format!("Error uploading build bundle: {}", format_docker_api_error(e)) }
            })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn get_archive_internal(&mut self, agent_id: &str, archive_rule: &ArchiveRule) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut download_stream = docker.download_from_container(agent_id, Some(DownloadFromContainerOptions {
                path: archive_rule.location.clone()
            }));

            let output_file_name = match &archive_rule.output {
                Some(output) => output.clone(),
                None => format!("{}.tar", archive_rule.name)
            };

            let mut f = File::create(output_file_name)
                .map_err(|e| {
                    BuildRuntimeError { msg: format!("Failed to create file for archive {}, due to {}", archive_rule.name, e) }
                })?;

            while let Some(download_item) = download_stream.next().await {
                let bytes = match download_item {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        return Err(BuildRuntimeError { msg: format!("Download error {}", format_docker_api_error(e)) });
                    }
                };

                f.write_all(&bytes)
                    .map_err(|e| {
                        BuildRuntimeError { msg: format!("Failed to write to archive {}, due to {}", archive_rule.name, e) }
                    })?;
            }

            Ok(())
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn execute_command_internal(&mut self, agent_id: &str, shell_config: &ShellConfig, working_directory: &str, command: &str) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let exec_id = docker.create_exec(agent_id, CreateExecOptions {
                cmd: Some(vec![shell_config.executable.as_str(), "-c", command]),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                attach_stdin: Some(true),
                tty: Some(true),
                working_dir: Some(working_directory),
                ..Default::default()
            }).await
                .map(|exec| exec.id)
                .map_err(|e| BuildRuntimeError { msg: format!("Failed to create exec: {}", format_docker_api_error(e)) })
                ?;

            let mut exec = docker.start_exec(&exec_id, None::<StartExecOptions>);

            while let Some(exec_result) = exec.next().await {
                match exec_result {
                    Ok(result) => {
                        match result {
                            StartExecResults::Attached { log } => {
                                print!("{}", log);
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
                    BuildRuntimeError { msg: format!("Failed to check command status {}", format_docker_api_error(e)) }
                })
                .and_then(|result| {
                    if let Some(running) = result.running {
                        if running {
                            return Err(BuildRuntimeError { msg: "Command has not exited.".to_string() });
                        }
                    } else {
                        return Err(BuildRuntimeError { msg: "Command status is not available.".to_string() });
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
                .map_err(|e| BuildRuntimeError { msg: format!("Failed to remove container [{}]: {}", agent_id, format_docker_api_error(e)) })
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
                .map_err(|e| BuildRuntimeError { msg: format!("Error removing volume [{}]: {}", volume_id, format_docker_api_error(e)) })
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
                    for warning in &results.warnings {
                        println!("Warning during list volumes: {}", warning);
                    }

                    results.volumes.iter().map(|x| {
                        let labels_copy = x.labels.clone();

                        (x.name.clone(), labels_copy)
                    }).collect()
                })
                .map_err(|e| BuildRuntimeError { msg : format!("Failed to list volumes {}", format_docker_api_error(e)) })
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
                .map_err(|e| BuildRuntimeError { msg : format!("Failed to list volumes {}", format_docker_api_error(e)) })
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn ensure_plugin_disk(&self) -> Result<String, BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let mut labels = HashMap::new();
            labels.insert("created-by".to_string(), "jarvis".to_string());
            labels.insert("used-for".to_string(), "plugins".to_string());

            let label_list = labels.iter().map(|x| format!("{}={}", x.0, x.1)).collect();

            let mut filters= HashMap::new();
            filters.insert("label".to_string(), label_list);

            let matches = docker.list_volumes(Some(ListVolumesOptions { filters })).await
                .map_err(|e| BuildRuntimeError { msg : format!("Failed to list volumes {}", format_docker_api_error(e)) })?;

            if matches.warnings.len() > 0 {
                for warning in &matches.warnings {
                    println!("Warning during list volumes: {}", warning);
                }
            }
            let matches_count = matches.volumes.len();

            if matches_count == 1 {
                // This is what we want, a single disk which matches the filter.
                return Ok(matches.volumes.get(0).unwrap().name.clone())
            }
            else if matches_count == 0 {
                let create_result = docker.create_volume(CreateVolumeOptions {
                    name: "jarvis-plugins".to_string(),
                    labels,
                    ..Default::default()
                }).await;

                Ok(create_result.map(|x| {
                    x.name
                }).map_err(|e| {
                    BuildRuntimeError { msg : format!("Failed to create plugin volume {}", format_docker_api_error(e)) }
                })?)
            } else {
                Err(BuildRuntimeError { msg : format!("Multiple plugin volumes found. Please remove unwanted disks until only 1 of {} remains", matches_count) })
            }
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn sync_plugins(&mut self, source_directory: String, target_disk: String) -> Result<(), BuildRuntimeError> {
        if let Some(ref docker) = self.docker {
            let id: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(30)
                .collect();

            let mut labels = HashMap::new();
            labels.insert("created-by".to_string(), "jarvis".to_string());
            labels.insert("used-for".to_string(), "plugin-sync".to_string());

            let mounts = vec![Mount {
                target: Some("/plugins".to_string()),
                source: Some(target_disk),
                typ: Some(MountTypeEnum::VOLUME),
                ..Default::default()
            },  Mount {
                target: Some("/input".to_string()),
                source: Some(source_directory),
                typ: Some(MountTypeEnum::BIND),
                ..Default::default()
            }];

            let command_config = vec!["/bin/sh", "-c", "tail -f /dev/null"].iter().map(|x| x.to_string()).collect();

            let image = "alpine:latest";
            let image_available = self.image_available(image).await?;
            if !image_available {
                self.pull_image(image).await?;
            }

            let container_result = docker.create_container(Some(CreateContainerOptions { name: id.clone() }), Config {
                image: Some(image.to_string()),
                entrypoint: Some(command_config),
                cmd: Some(vec![]),
                attach_stdin: Some(true),
                attach_stderr: Some(true),
                attach_stdout: Some(true),
                labels: Some(labels),
                working_dir: Some("/input".to_string()),
                host_config: Some(HostConfig {
                    mounts: Some(mounts),
                    ..Default::default()
                }),
                ..Default::default()
            }).await;

            let container = container_result.map(|x| {
                for warning in x.warnings {
                    println!("docker container create warning: {}", warning);
                }
                x.id
            }).map_err(|e| BuildRuntimeError { msg: format!("Failed to create plugin sync container: {}", format_docker_api_error(e)) })?;

            self.start_container(container.as_str()).await?;

            let shell_config = ShellConfig {
                executable: "/bin/sh".to_string()
            };

            self.execute_command_internal(container.as_str(), &shell_config, "/input", "cp -pR agent-worker agent-plugins /plugins").await?;

            self.delete_container(container.as_str()).await?;

            Ok(())
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
        let data_volume_name = format!("build-data-volume_{}_{}", module_name, id);
        let module_components = ModuleComponents {
            jarvis_directory: project_config.jarvis_directory.clone(),
            // TODO rename to workspace volume
            build_data_volume: data_volume_name.clone(),
            containers: HashMap::new(),
            // TODO identify the project more specifically to allow duplicate module names.
            identifier_base: format!("{}", module_name),
        };

        self.module_components.insert(module_name.to_string(), Box::new(module_components));

        self.create_docker_volume(data_volume_name.as_str(), None).await
            .map(|_| { () })?;

        let init_agent = self.create_agent(module_name, &Agent {
            name: "jarvis-init".to_string(),
            default: None,
            image: "alpine:latest".to_string(),
            environment: None,
            cache: None,
            container: None,
        }, None).await?;

        self.upload_project(init_agent.as_str(), &project_config.project_directory).await?;

        self.delete_container(init_agent.as_str()).await
    }

    async fn create_agent(&mut self, module_name: &String, agent: &Agent, step: Option<&Step>) -> Result<String, BuildRuntimeError> {
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .collect();
        let name = format!("jarvis-agent-{}-{}-{}", module_name, agent.name, id);

        if !self.image_available(agent.image.as_str()).await? {
            self.pull_image(agent.image.as_str()).await?;
        }

        let secrets = match &step {
            Some(step) => &step.secrets,
            None => &None
        };

        let using_plugins = match &step {
            Some(step) => step.plugins.is_some(),
            None => false
        };

        let secrets_config = configure_secrets(&self.module_components.get(module_name).unwrap().jarvis_directory, secrets)?;

        self.create_container(module_name, name.as_str(), agent, secrets_config, using_plugins).await
            .map(|x| {
                let component: &mut Box<ModuleComponents> = self.module_components.get_mut(module_name).unwrap();
                component.containers.insert(agent.name.clone(), x);
                ()
            })?;

        self.start_container(self.module_components.get(module_name).unwrap().containers.get(agent.name.as_str()).unwrap().as_str()).await?;

        Ok(name.clone())
    }

    async fn execute_command(&mut self, agent_id: &str, shell_config: &ShellConfig, command: &str) -> Result<(), BuildRuntimeError> {
        self.execute_command_internal(agent_id, shell_config, "/build/workspace", command).await
    }

    async fn get_archive(&mut self, agent_id: &str, archive_rule: &ArchiveRule) -> Result<(), BuildRuntimeError> {
        self.get_archive_internal(agent_id, archive_rule).await
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

    async fn ensure_plugins_loaded(&mut self, _plugins: Vec<&PluginSpecification>) -> Result<(), BuildRuntimeError> {
        // TODO do not unwrap me
        let agent_home = std::env::var("JARVIS_AGENT_HOME").unwrap();

        let plugin_disk_name = self.ensure_plugin_disk().await?;

        // TODO use the input plugins to ensure that the sync actually captures the correct plugins.

        self.sync_plugins(agent_home, plugin_disk_name).await?;

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

fn format_docker_api_error(e: bollard::errors::Error) -> String {
    // TDOO remove and replace with proper handling below.
    println!("{:?}", e);
    match e {
        _ => "Driver error".to_string()
    }
}

async fn ensure_caches_created(runtime: &DockerRuntime, cache_rules: &Vec<CacheRule>, identifier_base: &str) -> Result<HashMap<String, String>, BuildRuntimeError> {
    let volumes: HashMap<String, String> = cache_rules.iter().map(|rule| {
        let cache_volume_name = format!("cache_{}_{}", identifier_base, rule.name);
        (rule.name.clone(), cache_volume_name)
    }).collect();

    let mut extra_labels = HashMap::new();
    extra_labels.insert("used-for".to_string(), "caching".to_string());

    for volume in &volumes {
        runtime.create_docker_volume(volume.1, Some(extra_labels.clone())).await?;
    }

    Ok(volumes)
}
