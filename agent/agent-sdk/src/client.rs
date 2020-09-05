use jsonrpc_ipc_server::tokio::runtime::Runtime;
use jsonrpc_core::futures::future::Future;
use crate::{RegistrationResponseModel, InitializationModel, InitializationResponseModel, FinalizationModel, FinalizationResponseModel};
use crate::error::PluginError;

pub struct PluginClientConnection {
    conn: crate::gen_client::Client
}

impl PluginClientConnection {
    // TODO This disgustingness of passing the container back to access the runtime disappears once std::future is in use...
    pub fn register(&self, container: &mut PluginClientContainer) -> Result<RegistrationResponseModel, PluginError> {
        let f = self.conn.register().map_err(|e| { PluginError { msg: format!("{}", e) } });

        container.borrow_runtime().block_on(f)
    }

    pub fn initialize(&self, container: &mut PluginClientContainer, initialization_model: InitializationModel) -> Result<InitializationResponseModel, PluginError> {
        let f = self.conn.initialize(initialization_model).map_err(|e| { PluginError { msg: format!("{}", e) } });

        container.borrow_runtime().block_on(f)
    }

    pub fn finalize(&self, container: &mut PluginClientContainer, finalization_model: FinalizationModel) -> Result<FinalizationResponseModel, PluginError> {
        let f = self.conn.finalize(finalization_model).map_err(|e| { PluginError { msg: format!("{}", e) } });

        container.borrow_runtime().block_on(f)
    }
}

pub struct PluginClientContainer {
    runtime: Runtime
}

impl PluginClientContainer {
    pub fn new() -> Self {
        PluginClientContainer {
            runtime: Runtime::new().unwrap()
        }
    }

    pub fn create_client(&mut self, connect_to: &str) -> Option<PluginClientConnection> {
        // TODO fixed in version 15 of the ipc lib (not released yet)
        #[allow(deprecated)]
        let reactor = self.runtime.reactor().clone();

        let transport = jsonrpc_core_client::transports::ipc::connect::<String, crate::gen_client::Client>(format!(r"\\.\pipe\{}", connect_to), &reactor);

        match transport {
            Ok(trans) => {
                let client = self.runtime.block_on(trans).unwrap();

                return Some(PluginClientConnection {
                    conn: client
                });
            },
            Err(e) => {
                println!("Failed to connect: {}", e);
                None
            }
        }
    }

    fn borrow_runtime(&mut self) -> &mut Runtime {
        return &mut self.runtime;
    }
}


