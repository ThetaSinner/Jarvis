use crate::config::{get_project_config, ProjectConfig, ConfigError, Module, Agent, Step};
use std::collections::HashMap;
use std::borrow::Borrow;

struct BuildAgentConfig<'a> {
    name: String,

    agents: HashMap<String, &'a Agent>,

    default_agent: Option<String>,
}

pub fn build_project(project_path: std::path::PathBuf) -> Option<ConfigError> {
    let project_config_result = get_project_config(project_path);

    match project_config_result {
        Err(e) => Some(e),
        Ok(project_config) => {
            build_project_with_config(project_config);
            None
        }
    }
}

fn build_project_with_config(project_config: ProjectConfig) -> Result<String, &'static str> {
    for module in project_config.build_config.modules {
        println!("Building module: {}", module.name);

        let build_agent_config_result = configure_agents(&module);
        let agent_config = match build_agent_config_result {
            Ok(build_agent_config) => build_agent_config,
            Err(e) => return Err(e)
        };

        build_module(&module, agent_config);
    }

    Ok("".to_string())
}

fn build_module(module: &Module, agent_config: BuildAgentConfig) -> Result<String, &'static str> {
    if module.steps.is_empty() {
        return Err("No build steps provided.");
    }

    // for step in module.steps.borrow() {
    //
    // }

    Ok("".to_string())
}

fn configure_agents(module: &Module) -> Result<BuildAgentConfig, &'static str> {
    let mut build_model = BuildAgentConfig {
        name: module.name.clone(),
        agents: HashMap::new(),
        default_agent: None,
    };

    match module.agents {
        None => {}
        Some(ref agent_list) => {
            let default_agent_result = get_default_agent(agent_list);

            if default_agent_result.is_err() {
                return Err(default_agent_result.err().unwrap());
            }

            let default_agent = default_agent_result.unwrap();
            build_model.default_agent = default_agent;

            build_model.agents = agent_list.iter().map(|agent| (agent.name.clone(), agent)).collect();
        },
    }

    Ok(build_model)
}

fn get_default_agent(agents: &Vec<Agent>) -> Result<Option<String>, &'static str> {
    let mut matches = agents.iter().filter(|agent| agent.default.is_some() && agent.default.unwrap() == true);

    let first = matches.next();
    if matches.next().is_some() {
        Err("Only one agent can be specified as default!")
    } else {
        match first {
            Some(agent) => Ok(Some(agent.name.clone())),
            None => Ok(None)
        }
    }
}
