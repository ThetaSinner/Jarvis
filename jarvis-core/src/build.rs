use crate::config::{get_project_config, ProjectConfig, Module, Agent, Step};
use std::collections::HashMap;
use crate::runtime::{BuildRuntime, BuildRuntimeError};
use std::fmt;
use std::fmt::Formatter;
use std::error::Error;

struct BuildAgentConfig<'a> {
    agents: HashMap<String, &'a Agent>,

    default_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuildError {
    msg: String
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "build runtime error: {}", self.msg)
    }
}

impl Error for BuildError {}

pub async fn build_project(project_path: std::path::PathBuf, mut runtime: Box<dyn BuildRuntime>) -> Result<(), BuildError> {
    let project_config = get_project_config(project_path)
        .map_err(|e| BuildError { msg: format!("Project configuration error: {}", e) })?;

    runtime.connect();

    build_project_with_config(project_config, &mut runtime).await
}

async fn build_project_with_config(project_config: ProjectConfig, runtime: &mut Box<dyn BuildRuntime>) -> Result<(), BuildError> {
    for module in &project_config.build_config.modules {
        println!("Building module: {}", module.name);

        let agent_config = configure_agents(&module)
            .map_err(|e| {
                BuildError { msg: format!("Error configuring agents: {}", e) }
            })?;

        runtime.init_for_module(&module.name, &project_config).await.map_err(build_project_error)?;
        let module_build_result = build_module(&module, &agent_config, runtime).await;
        runtime.tear_down_for_module(&module.name).await.map_err(build_project_error)?;

        if module_build_result.is_err() {
            return module_build_result;
        }
    }

    Ok(())
}

fn build_project_error(bre: BuildRuntimeError) -> BuildError {
    BuildError { msg: format!("Failed to build project: {}", bre) }
}

async fn build_module<'a>(module: &Module, agent_config: &'a BuildAgentConfig<'a>, runtime: &mut Box<dyn BuildRuntime>) -> Result<(), BuildError> {
    if module.steps.is_empty() {
        return Err(BuildError { msg: "No build steps provided.".to_string() });
    }

    for step in &module.steps {
        run_step(step, &module.name, agent_config, runtime).await?;
    }

    Ok(())
}

async fn run_step<'a>(step: &Step, module_name:&String, agent_config: &'a BuildAgentConfig<'a>, runtime: &mut Box<dyn BuildRuntime>) -> Result<(), BuildError> {
    let agent = if let Some(ref agent) = step.agent {
        if agent_config.agents.contains_key(agent) {
            agent_config.agents[agent]
        } else {
            return Err(BuildError { msg: format!("Step [{}] attempting to use agent [{}] which isn't defined", step.name, agent) });
        }
    } else {
        if let Some(ref agent) = agent_config.default_agent {
            agent_config.agents[agent]
        } else {
            return Err(BuildError { msg: format!("Step [{}] doesn't specify an agent and there is no default agent", step.name) });
        }
    };

    let agent_id = runtime.create_agent(module_name, agent).await
        .map_err(|e| run_step_error(step.name.as_str(), e))?;

    let command_result = runtime.execute_command(agent_id.as_str(), &step.command).await
        .map_err(|e| run_step_error(step.name.as_str(), e));

    runtime.destroy_agent(agent_id.as_str()).await
        .map_err(|e| run_step_error(step.name.as_str(), e))?;

    command_result
}

fn run_step_error(step_name: &str, bre: BuildRuntimeError) -> BuildError {
    BuildError { msg: format!("Failed to run step [{}]: {}", step_name, bre) }
}

fn configure_agents(module: &Module) -> Result<BuildAgentConfig, &'static str> {
    let mut build_model = BuildAgentConfig {
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
