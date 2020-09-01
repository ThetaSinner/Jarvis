use agent_sdk::{InitializationResponseModel, FinalizationResponseModel};
use agent_sdk::plugin_id;
use agent_sdk::server::JarvisAgentPluginContainer;

fn main() {
    let mut container = JarvisAgentPluginContainer::new(plugin_id!());

    container.add_initialize(|_req| {
        println!("Initialise");
        Ok(InitializationResponseModel {})
    });

    container.add_finalize(|_req| {
        println!("Finalise");
        Ok(FinalizationResponseModel {})
    });

    container.start();
}
