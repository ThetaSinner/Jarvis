use jsonrpc_core::{Result, ErrorCode, IoHandler};
use jsonrpc_derive::rpc;
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use jsonrpc_ipc_server::ServerBuilder;

#[rpc]
pub trait AgentPlugin {
    #[rpc(name = "register")]
    fn register(&self) -> Result<RegistrationModel>;
}

#[derive(Serialize, Deserialize)]
pub struct RegistrationModel {
    pub lifecycle_init: bool,
}

struct AgentPluginImpl {
    registration: Option<fn() -> std::result::Result<RegistrationModel, PluginError>>
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

    pub fn add_register(&mut self, f: fn() -> std::result::Result<RegistrationModel, PluginError>) {
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

#[derive(Debug)]
pub struct PluginError {
    pub msg: String,
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "config error: {}", self.msg)
    }
}

impl Error for PluginError {}

#[macro_export]
macro_rules! plugin_id {
    () => {
        {
            format!("{}_{}_{}_{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION_MAJOR"), env!("CARGO_PKG_VERSION_MINOR"), env!("CARGO_PKG_VERSION_PATCH"))
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
