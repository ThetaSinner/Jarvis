use agent_sdk::RegistrationModel;
use agent_sdk::plugin_id;
use agent_sdk::server::JarvisAgentPluginContainer;

fn main() {
    let mut container = JarvisAgentPluginContainer::new(plugin_id!());

    container.add_register(|| {
        println!("Handling registration");
        Ok(RegistrationModel {
            lifecycle_init: true
        })
    });

    container.start();
}
