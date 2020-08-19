use crate::runtime::{BuildRuntime, BuildRuntimeError};
use std::collections::HashMap;
use crypto::sha2::Sha256;
use bollard::Docker;
use crypto::digest::Digest;
use bollard::volume::CreateVolumeOptions;
use async_trait::async_trait;

use crate::config::Agent;
use bollard::container::{CreateContainerOptions, Config};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use futures::TryStreamExt;
use bollard::image::{CreateImageOptions, CreateImageResults};
use tokio::stream::StreamExt;
use std::io;
use std::io::Write;

pub struct DockerRuntime {
    docker: Option<Docker>,

    module_components: HashMap<String, Box<ModuleComponents>>,
}

struct ModuleComponents {
    build_data_volume: String,

    containers: HashMap<String, String>
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

    // async fn pull_image(&self, image: &str) -> Result<(), BuildRuntimeError> {
    //     if let Some(ref docker) = self.docker {
    //         let pull_results = docker.create_image(Some(CreateImageOptions {
    //             from_image: image,
    //             ..Default::default()
    //         }), None, None).try_collect::<Vec<_>>().await.map_err(|e| {
    //             BuildRuntimeError { msg: format!("Failed to pull image: {}", e) }
    //         })?;
    //
    //         for result in pull_results {
    //             match result {
    //                 CreateImageResults::CreateImageProgressResponse { status, progress_detail, id, progress } => {
    //                     println!("{}", status)
    //                 },
    //                 CreateImageResults::CreateImageError { error, error_detail} => {
    //                     return Err(BuildRuntimeError { msg: format!("Image pull error: {}", error) })
    //                 }
    //             }
    //         }
    //
    //         Ok(())
    //     } else {
    //         Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
    //     }
    // }

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
                            CreateImageResults::CreateImageProgressResponse { status, progress_detail, id, progress } => {
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

                                io::stdout().flush();
                            },
                            CreateImageResults::CreateImageError { error_detail, error } => {
                                print!("\n{}", ansi_escapes::CursorShow);
                                println!("{}", error);
                                return Err(BuildRuntimeError { msg: format!("Image pull error: {}", error) })
                            }
                        }
                    },
                    Err(e) => {
                        print!("\n{}", ansi_escapes::CursorShow);
                        return Err(BuildRuntimeError { msg: format!("Image pull error: {}", e) })
                    }
                }
            }

            print!("\n{}", ansi_escapes::CursorShow);
            Ok(())
        } else {
            Err(BuildRuntimeError { msg: "Runtime has not been initialised".to_string() })
        }
    }

    async fn create_container(&self, name: String, agent: &Agent) -> Result<String, BuildRuntimeError> {
        let mut environment = None;
        if let Some(ref env) = agent.environment {
            let env_list = env.keys().map(|key| format!("{}={}", key, env[key])).collect();
            environment = Some(env_list);
        }

        if let Some(ref docker) = self.docker {
            let container_result = docker.create_container(Some(CreateContainerOptions { name }), Config {
                image: Some(agent.image.clone()),
                cmd: Some(vec!["/bin/sh", "-c", "tail -f /dev/null"].iter().map(|x| x.to_string()).collect()),
                env: environment,
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
}

#[async_trait]
impl BuildRuntime for DockerRuntime {
    fn test(&self) {
        println!("I'm the docker runtime");
    }

    fn connect(&mut self) {
        self.docker = Some(Docker::connect_with_local_defaults().unwrap())
    }

    async fn init_for_module(&mut self, module_name: &String) -> Result<(), BuildRuntimeError> {
        let mut hasher = Sha256::new();
        hasher.input_str("build_data_volume-");
        hasher.input_str(module_name);
        let module_components = ModuleComponents {
            build_data_volume: hasher.result_str(),
            containers: HashMap::new()
        };

        self.module_components.insert(module_name.to_string(), Box::new(module_components));

        return self.create_docker_volume(&hasher.result_str()).await
            .map(|x| {
                println!("Created build data volume: {}", x);
                ()
            });
    }

    async fn create_agent(&mut self, module_name: &String, agent: &Agent) -> Result<(), BuildRuntimeError> {
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .collect();
        let name = format!("jarvis-agent-{}-{}-{}", module_name, agent.name, id);

        println!("Create agent {}", name);

        self.pull_image(agent.image.as_str()).await?;

        self.create_container(name, agent).await
            .map(|x| {
                let component: &mut Box<ModuleComponents> = self.module_components.get_mut(module_name).unwrap();
                component.containers.insert(agent.name.clone(), x);
                ()
            })
    }
}
