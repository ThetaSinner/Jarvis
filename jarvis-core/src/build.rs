use crate::config::{get_project_config, ProjectConfig, ConfigError, Module, Agent, Step};
use std::collections::HashMap;
use crate::runtime::BuildRuntime;

struct BuildAgentConfig<'a> {
    name: String,

    agents: HashMap<String, &'a Agent>,

    default_agent: Option<String>,
}

pub async fn build_project(project_path: std::path::PathBuf, mut runtime: Box<dyn BuildRuntime>) -> Option<ConfigError> {
    let project_config_result = get_project_config(project_path);

    runtime.connect();

    match project_config_result {
        Err(e) => Some(e),
        Ok(project_config) => {
            build_project_with_config(project_config, &mut runtime).await;
            None
        }
    }
}

async fn build_project_with_config(project_config: ProjectConfig, runtime: &mut Box<dyn BuildRuntime>) -> Result<String, &'static str> {
    for module in &project_config.build_config.modules {
        println!("Building module: {}", module.name);

        let build_agent_config_result = configure_agents(&module);
        let agent_config = match build_agent_config_result {
            Ok(build_agent_config) => build_agent_config,
            Err(e) => return Err(e)
        };

        runtime.init_for_module(&module.name, &project_config).await;
        build_module(&module, &agent_config, runtime).await;
        runtime.tear_down_for_module(&module.name).await;
    }

    Ok("".to_string())
}

async fn build_module<'a>(module: &Module, agent_config: &'a BuildAgentConfig<'a>, runtime: &mut Box<dyn BuildRuntime>) -> Result<String, &'static str> {
    if module.steps.is_empty() {
        return Err("No build steps provided.");
    }

    for step in &module.steps {
        run_step(step, &module.name, agent_config, runtime).await;
    }

    Ok("".to_string())
}

async fn run_step<'a>(step: &Step, module_name:&String, agent_config: &'a BuildAgentConfig<'a>, runtime: &mut Box<dyn BuildRuntime>) -> Result<(), ConfigError> {
    let agent = if let Some(ref agent) = step.agent {
        if agent_config.agents.contains_key(agent) {
            agent_config.agents[agent]
        } else {
            return Err(ConfigError { msg: format!("Step [{}] attempting to use agent [{}] which isn't defined", step.name, agent) });
        }
    } else {
        if let Some(ref agent) = agent_config.default_agent {
            agent_config.agents[agent]
        } else {
            return Err(ConfigError { msg: format!("Step [{}] doesn't specify an agent and there is no default agent", step.name) });
        }
    };

    let agent_id = runtime.create_agent(module_name, agent).await
        .map_err(|e| println!("Failed to create container {}", e)).unwrap();

    runtime.execute_command(agent_id.as_str(), &step.command).await;

    runtime.destroy_agent(agent_id.as_str()).await;

    println!("{}", step.name);
    Ok(())
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
