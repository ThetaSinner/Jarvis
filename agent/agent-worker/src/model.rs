use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct AgentInitialization {
    pub plugins: Option<Vec<PluginSpec>>
}

#[derive(Deserialize, Serialize)]
pub struct PluginSpec {
    pub name: String,

    pub version: String,
}
