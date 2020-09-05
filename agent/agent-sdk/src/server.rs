use crate::{RegistrationResponseModel, error, AgentPlugin, InitializationModel, InitializationResponseModel, FinalizationModel, FinalizationResponseModel};
use jsonrpc_ipc_server::jsonrpc_core::{IoHandler, ErrorCode};
use jsonrpc_ipc_server::ServerBuilder;
use jsonrpc_core::Result;
use crate::error::PluginError;

pub struct JarvisAgentPluginContainer {
    id: String,
    plugin_impl: AgentPluginImpl,
}

impl JarvisAgentPluginContainer {
    pub fn new(id: String) -> Self {
        JarvisAgentPluginContainer {
            id,
            plugin_impl: AgentPluginImpl { initialization: None, finalization: None }
        }
    }

    pub fn add_initialize(&mut self, f: fn(InitializationModel) -> std::result::Result<InitializationResponseModel, error::PluginError>) {
        self.plugin_impl.initialization = Some(f);
    }

    pub fn add_finalize(&mut self, f: fn(FinalizationModel) -> std::result::Result<FinalizationResponseModel, error::PluginError>) {
        self.plugin_impl.finalization = Some(f);
    }

    pub fn start(self) {
        let mut io = IoHandler::new();
        io.extend_with(self.plugin_impl.to_delegate());

        let builder = ServerBuilder::new(io);

        let server = builder.start(format!(r"\\.\pipe\{}", self.id).as_str()).expect("Couldn't open socket");
        server.wait();
    }
}

struct AgentPluginImpl {
    initialization: Option<fn(InitializationModel) -> std::result::Result<InitializationResponseModel, error::PluginError>>,

    finalization: Option<fn(FinalizationModel) -> std::result::Result<FinalizationResponseModel, error::PluginError>>,
}

impl AgentPlugin for AgentPluginImpl {
    fn register(&self) -> Result<RegistrationResponseModel> {
        let result = Ok(RegistrationResponseModel {
            lifecycle_initialize: self.initialization.is_some(),
            lifecycle_finalize: self.finalization.is_some()
        });
        map_result_for_json_rpc(result)
    }

    fn initialize(&self, initialization_model: InitializationModel) -> Result<InitializationResponseModel> {
        if let Some(initialisation) = &self.initialization {
            let result = initialisation(initialization_model);
            map_result_for_json_rpc(result)
        } else {
            make_not_implemented_error::<InitializationResponseModel>()
        }
    }

    fn finalize(&self, finialization_model: FinalizationModel) -> Result<FinalizationResponseModel> {
        if let Some(finalisation) = &self.finalization {
            let result = finalisation(finialization_model);
            map_result_for_json_rpc(result)
        } else {
            make_not_implemented_error::<FinalizationResponseModel>()
        }
    }
}

fn map_result_for_json_rpc<T>(result: std::result::Result<T, PluginError>) -> Result<T> {
    match result {
        Ok(response) => {
            Result::Ok(response)
        },
        Err(e) => {
            Result::Err(jsonrpc_core::Error {
                code: ErrorCode::ServerError(0),
                message: format!("{}", e),
                data: None
            })
        }
    }
}

fn make_not_implemented_error<T>() -> Result<T> {
    Result::Err(jsonrpc_core::Error {
        code: ErrorCode::ServerError(1),
        message: format!("Not implemented"),
        data: None
    })
}
