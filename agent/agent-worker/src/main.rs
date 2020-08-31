fn main() {
    let mut container = agent_sdk::client::PluginClientContainer::new();

    let client = container.create_client("hello-world-plugin_0_1_0");

    if let Some(c) = client {
        c.register(&mut container, |out| {println!("response {}", out.lifecycle_init)})
    }
}
