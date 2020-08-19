use std::env::current_dir;

use colored::Colorize;
use futures::executor::block_on;
use futures::future::Ready;
use structopt::StructOpt;
use tokio::runtime::Runtime;

use jarvis_core::{build_project, docker_things, RuntimeOption, validate_project};

#[derive(StructOpt)]
/// The Jarvis CLI
struct Cli {
    #[structopt(subcommand)]
    cmd: SubCommands,
}

#[derive(StructOpt)]
enum SubCommands {
    Validate {
        #[structopt(long, parse(from_os_str))]
        /// The project to use
        project: Option<std::path::PathBuf>
    },

    Build {
        #[structopt(long, parse(from_os_str))]
        /// The project to use
        project: Option<std::path::PathBuf>,

        #[structopt(long, default_value = "")]
        runtime: RuntimeOption,
    },

    Docker {
        #[structopt(long, parse(from_os_str))]
        /// The project to use
        project: Option<std::path::PathBuf>
    },
}

fn main() {
    let args = Cli::from_args();

    let exit_code;

    let mut rt = Runtime::new().unwrap();

    match args.cmd {
        SubCommands::Validate { project } => {
            let project_dir = match project {
                Some(project) => project,
                None => current_dir().unwrap()
            };
            exit_code = validate(project_dir);
        }
        SubCommands::Build { project, runtime } => {
            let project_dir = match project {
                Some(project) => project,
                None => current_dir().unwrap()
            };
            println!("{}", runtime);
            exit_code = block_on(rt.block_on(build(project_dir, runtime))).unwrap();
        }
        SubCommands::Docker { project } => {
            let project_dir = match project {
                Some(project) => project,
                None => current_dir().unwrap()
            };
            block_on(docker_things(project_dir));
            exit_code = 1
        }
    }

    std::process::exit(exit_code)
}

fn validate(project: std::path::PathBuf) -> i32 {
    println!("Start validation.");

    let validation_result = validate_project(project);
    if validation_result.is_ok() {
        let messages = validation_result.unwrap();
        if messages.errors.is_empty() && messages.warnings.is_empty() {
            println!("{} {}", gh_emoji::get("+1").unwrap(), "Validation succeeded with no errors or warnings!".bright_green())
        } else {
            for warning in messages.warnings {
                println!("{} {}", gh_emoji::get("warning").unwrap(), warning.yellow())
            }
            for error in messages.errors {
                println!("{} {}", gh_emoji::get("x").unwrap(), error.yellow())
            }
        }

        0
    } else {
        println!("{} {}", gh_emoji::get("-1").unwrap(), validation_result.err().unwrap().to_string().bright_red());
        1
    }
}

async fn build(project: std::path::PathBuf, runtime: RuntimeOption) -> Ready<Result<i32, ()>> {
    build_project(project, runtime).await;
    futures::future::ok(1)
}
