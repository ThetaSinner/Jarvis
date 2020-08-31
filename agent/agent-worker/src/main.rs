use tokio::runtime::Runtime;
use futures::executor::block_on;
use jsonrpc_core::Params;
use serde_json::map::Map;

fn main() {
    let mut rt = Runtime::new().unwrap();

    // TODO fixed in version 15 of the ipc lib (not released yet)
    #[allow(deprecated)]
    let reactor = rt.reactor().clone();

    let transport = jsonrpc_core_client::transports::ipc::connect::<&str, jsonrpc_core_client::RawClient>(r"\\.\pipe\hello-world-plugin_0_1_0", &reactor);

    match transport {
        Ok(trans) => {
            let client = rt.block_on(trans).unwrap();

            let fut = client.call_method("register", Params::None);

            // FIXME: it seems that IPC server on Windows won't be polled with
            // default I/O reactor, work around with sending stop signal which polls
            // the server (https://github.com/paritytech/jsonrpc/pull/459)
            //server.close();

            match rt.block_on(fut) {
                Ok(_) => println!("done!"),
                Err(err) => panic!("IPC RPC call failed: {}", err),
            }
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
}
