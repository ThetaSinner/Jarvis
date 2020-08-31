use crate::{RegistrationModel, error, AgentPlugin};
use jsonrpc_ipc_server::jsonrpc_core::{IoHandler, ErrorCode};
use jsonrpc_ipc_server::ServerBuilder;
use jsonrpc_core::Result;

pub struct JarvisAgentPluginContainer {
    id: String,
    plugin_impl: AgentPluginImpl,
}

impl JarvisAgentPluginContainer {
    pub fn new(id: String) -> Self {
        JarvisAgentPluginContainer {
            id,
            plugin_impl: AgentPluginImpl { registration: None }
        }
    }

    pub fn add_register(&mut self, f: fn() -> std::result::Result<RegistrationModel, error::PluginError>) {
        self.plugin_impl.registration = Some(f);
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
    registration: Option<fn() -> std::result::Result<RegistrationModel, error::PluginError>>
}

impl AgentPlugin for AgentPluginImpl {
    fn register(&self) -> Result<RegistrationModel> {
        if let Some(registration) = &self.registration {
            match registration() {
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
        } else {
            Result::Err(jsonrpc_core::Error {
                code: ErrorCode::ServerError(1),
                message: format!("Not implemented"),
                data: None
            })
        }
    }
}
