use crate::runtime::{BuildRuntime, BuildRuntimeError};
use std::collections::HashMap;
use crypto::sha2::Sha256;
use futures_util::AsyncWriteExt;
use bollard::Docker;
use crypto::digest::Digest;
use bollard::volume::{CreateVolumeOptions, VolumeAPI};
use std::mem::MaybeUninit;
use std::future::Future;
use bollard::errors::Error;
use async_trait::async_trait;

use crate::config::BuildConfig;

pub struct DockerRuntime {
    docker: Option<Docker>,

    module_components: HashMap<String, ModuleComponents>,
}

struct ModuleComponents {
    build_data_volume: String,
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
            build_data_volume: hasher.result_str()
        };

        self.module_components.insert(module_name.to_string(), module_components);

        return self.create_docker_volume(&hasher.result_str()).await
            .map(|x| {
                println!("Created build data volume: {}", x);
                ()
            });
    }
}
