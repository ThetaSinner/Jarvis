use agent_sdk::{InitializationModel, FinalizationModel};
use warp::Filter;
use crate::model::AgentInitialization;
use warp::hyper::StatusCode;
use std::path::PathBuf;
use std::process::{Stdio, Child};
use std::sync::Arc;
use tokio::sync::Mutex;
use regex::Regex;
use std::convert::Infallible;
use std::io::{Write, Read};

mod model;

type PluginExecutions = Arc<Mutex<Vec<PluginExecution>>>;

pub struct PluginExecution {
    client_name: String,

    handle: Option<Child>,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let data = Arc::new(Mutex::new(std::vec::Vec::<PluginExecution>::new()));

    let inspect_data = data.clone();
    let inspect = warp::path!("inspect")
        .and(warp::get())
        // Only accept bodies smaller than 16kb...
        .and(warp::any().map(move || inspect_data.clone()))
        .and_then(inspect);

    let plugins_data = data.clone();
    let promote = warp::path!("plugins")
        .and(warp::post())
        // Only accept bodies smaller than 16kb...
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(warp::any().map(move || plugins_data.clone()))
        .and_then(initialise_agent);

    let initialize_data = data.clone();
    let initialise = warp::path!("initialize")
        .and(warp::post())
        .and(warp::any().map(move || initialize_data.clone()))
        .and_then(initialize_plugins);

    let finalize_data = data.clone();
    let finalize = warp::path!("finalize")
        .and(warp::post())
        .and(warp::any().map(move || finalize_data.clone()))
        .and_then(finalize_plugins);

    let routes = promote
        .or(initialise)
        .or(inspect)
        .or(finalize);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 30311))
        .await
}

async fn initialise_agent(agent_initialization: AgentInitialization, plugin_executions: PluginExecutions) -> Result<impl warp::Reply, Infallible> {
    let agent_home = std::env::var("JARVIS_AGENT_HOME").unwrap();

    let mut path = PathBuf::new();
    path.push(agent_home);

    if let Some(plugins) = &agent_initialization.plugins {
        for plugin in plugins {
            let mut plugin_path = path.clone();
            plugin_path.push(plugin.name.as_str());
            plugin_path.push(plugin.version.as_str());
            plugin_path.push(format!("{}.exe", plugin.name));

            println!("Plugin found? {}", plugin_path.exists());

            let handle = std::process::Command::new(plugin_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect(format!("plugin execution failed {}", plugin.name).as_str());

            let mut pe = plugin_executions.lock().await;
            pe.push(PluginExecution {
                client_name: format!("{}_{}", plugin.name, format_version(plugin.version.as_str())),
                handle: Some(handle)
            });

            log::info!("{}", pe.last().unwrap().client_name);
        }
    }

    Ok(warp::reply::with_status("", StatusCode::ACCEPTED))
}

fn format_version(version: &str) -> String {
    // TODO regex in a loop
    let re = Regex::new(r"(\d+)\.(\d+)\.(\d+)").unwrap();
    let captures = re.captures_iter(version).next().unwrap();

    format!("{}_{}_{}", &captures[1], &captures[2], &captures[3])
}

async fn initialize_plugins(plugin_executions: PluginExecutions) -> Result<impl warp::Reply, Infallible> {
    let mut container = agent_sdk::client::PluginClientContainer::new();

    let mut pe = plugin_executions.lock().await;

    for plugin_execution in pe.iter_mut() {
        let client = container.create_client(plugin_execution.client_name.as_str());

        if let Some(c) = client {
            let registration = c.register(&mut container).unwrap();

            if registration.lifecycle_initialize {
                c.initialize(&mut container, InitializationModel {}).unwrap();
            }
        }
    }

    Ok(warp::reply::with_status("", StatusCode::ACCEPTED))
}

async fn inspect(plugin_executions: PluginExecutions) -> Result<impl warp::Reply, Infallible> {
    let pe = plugin_executions.lock().await;

    let res = pe.iter().map(|x| x.client_name.as_str()).collect::<Vec<&str>>();
    Ok(warp::reply::json(&res))
}

async fn finalize_plugins(plugin_executions: PluginExecutions) -> Result<impl warp::Reply, Infallible> {
    let mut container = agent_sdk::client::PluginClientContainer::new();

    let mut pe = plugin_executions.lock().await;

    for plugin_execution in pe.iter_mut() {
        let client = container.create_client(plugin_execution.client_name.as_str());

        if let Some(c) = client {
            let registration = c.register(&mut container).unwrap();

            if registration.lifecycle_finalize {
                c.finalize(&mut container, FinalizationModel {}).unwrap();
            }

            // Close the client before terminating the server.
            drop(c);

            if let Some(handle) = &mut plugin_execution.handle {
                handle.kill().expect("Failed to kill plugin");

                if let Some(out) = &mut handle.stdout {
                    let mut buf: Vec<u8> = vec![];
                    out.read_to_end(&mut buf).unwrap();
                    println!("---");
                    std::io::stdout().write_all(buf.as_slice()).unwrap();
                    println!("---");
                }
            }
        }
    }

    Ok(warp::reply::with_status("", StatusCode::ACCEPTED))
}
