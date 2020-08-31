use jsonrpc_ipc_server::tokio::runtime::Runtime;
use jsonrpc_core::futures::future::Future;
use crate::RegistrationModel;

pub fn thingy() {
    let mut rt = Runtime::new().unwrap();

    // TODO fixed in version 15 of the ipc lib (not released yet)
    #[allow(deprecated)]
        let reactor = rt.reactor().clone();

    let transport = jsonrpc_core_client::transports::ipc::connect::<&str, crate::gen_client::Client>(r"\\.\pipe\hello-world-plugin_0_1_0", &reactor);

    match transport {
        Ok(trans) => {
            let client = rt.block_on(trans).unwrap();

            let f = client.register().map(|out| {
                println!("{}", out.lifecycle_init)
            });

            rt.block_on(f).unwrap();
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
}

pub struct PluginClientConnection {
    conn: crate::gen_client::Client
}

impl PluginClientConnection {
    // TODO This disgustingness of passing the container back to access the runtime disappears once std::future is in use...
    pub fn register(&self, container: &mut PluginClientContainer, handler: fn (RegistrationModel) -> ()) {
        let f = self.conn.register().map(handler);

        container.borrow_runtime().block_on(f).unwrap()
    }
}

pub struct PluginClientContainer {
    runtime: Runtime
}

impl PluginClientContainer {
    pub fn new() -> Self {
        PluginClientContainer {
            runtime: Runtime::new().unwrap()
        }
    }

    pub fn create_client(&mut self, connect_to: &str) -> Option<PluginClientConnection> {
        // TODO fixed in version 15 of the ipc lib (not released yet)
        #[allow(deprecated)]
        let reactor = self.runtime.reactor().clone();

        let transport = jsonrpc_core_client::transports::ipc::connect::<String, crate::gen_client::Client>(format!(r"\\.\pipe\{}", connect_to), &reactor);

        match transport {
            Ok(trans) => {
                let client = self.runtime.block_on(trans).unwrap();

                return Some(PluginClientConnection {
                    conn: client
                });
            },
            Err(e) => {
                println!("Failed to connect: {}", e);
                None
            }
        }
    }

    fn borrow_runtime(&mut self) -> &mut Runtime {
        return &mut self.runtime;
    }
}


