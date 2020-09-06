use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use serde::{Serialize, Deserialize};

pub mod error;
pub mod server;
pub mod client;

#[rpc]
pub trait AgentPlugin {
    #[rpc(name = "register")]
    fn register(&self) -> Result<RegistrationResponseModel>;

    #[rpc(name = "initialise")]
    fn initialize(&self, initialisation_model: InitializationModel) -> Result<InitializationResponseModel>;

    #[rpc(name = "finalize")]
    fn finalize(&self, finialisation_model: FinalizationModel) -> Result<FinalizationResponseModel>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationResponseModel {
    pub lifecycle_initialize: bool,

    pub lifecycle_finalize: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializationModel {}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializationResponseModel {}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinalizationModel {}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinalizationResponseModel {}

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
