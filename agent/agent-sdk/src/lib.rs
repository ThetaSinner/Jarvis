use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use serde::{Serialize, Deserialize};
use jsonrpc_core::futures::future::Future;

pub mod error;
pub mod server;
pub mod client;

#[rpc]
pub trait AgentPlugin {
    #[rpc(name = "register")]
    fn register(&self) -> Result<RegistrationModel>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationModel {
    pub lifecycle_init: bool,
}

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
