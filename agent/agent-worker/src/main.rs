use agent_sdk::{InitzalisationModel, FinalizationModel};

fn main() {
    let mut container = agent_sdk::client::PluginClientContainer::new();

    let client = container.create_client("hello-world-plugin_0_1_0");

    if let Some(c) = client {
        let registration = c.register(&mut container).unwrap();

        if registration.lifecycle_initialize {
            c.initialize(&mut container, InitzalisationModel {}).unwrap();
        }

        if registration.lifecycle_finalize {
            c.finalize(&mut container, FinalizationModel {}).unwrap();
        }
    }
}
