mod cli_output_formatter;

use std::env::current_dir;

use colored::Colorize;
use futures::executor::block_on;
use futures::future::Ready;
use structopt::StructOpt;
use tokio::runtime::Runtime;

use jarvis_core::{build_project, RuntimeOption, validate_project, OutputFormatter, cleanup_resources, init_project, core_test};
use crate::cli_output_formatter::CliOutputFormatter;

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

    Init {
        #[structopt(long, parse(from_os_str))]
        /// The project to use
        project: Option<std::path::PathBuf>,

        #[structopt(long, default_value = "")]
        runtime: RuntimeOption,
    },

    Build {
        #[structopt(long, parse(from_os_str))]
        /// The project to use
        project: Option<std::path::PathBuf>,

        #[structopt(long, default_value = "")]
        runtime: RuntimeOption,
    },

    Cleanup {
        #[structopt(long, default_value = "")]
        runtime: RuntimeOption,
    },

    Test {},
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
        SubCommands::Init { project, runtime } => {
            let cli_output_formatter = Box::new(CliOutputFormatter {});
            let project_dir = match project {
                Some(project) => project,
                None => current_dir().unwrap()
            };
            exit_code = block_on(rt.block_on(init(project_dir, runtime, cli_output_formatter))).unwrap();
        }
        SubCommands::Build { project, runtime } => {
            let cli_output_formatter = Box::new(CliOutputFormatter {});
            let project_dir = match project {
                Some(project) => project,
                None => current_dir().unwrap()
            };
            exit_code = block_on(rt.block_on(build(project_dir, runtime, cli_output_formatter))).unwrap();
        }
        SubCommands::Cleanup { runtime } => {
            let cli_output_formatter = Box::new(CliOutputFormatter {});
            exit_code = block_on(rt.block_on(cleanup(runtime, cli_output_formatter))).unwrap();
        }
        SubCommands::Test {} => {
            let cli_output_formatter = Box::new(CliOutputFormatter {});
            exit_code = block_on(rt.block_on(test(cli_output_formatter))).unwrap();
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

async fn init(project: std::path::PathBuf, runtime: RuntimeOption, output_formatter: Box<dyn OutputFormatter>) -> Ready<Result<i32, ()>> {
    let result = init_project(project, runtime, &output_formatter).await;

    match result {
        Ok(_) => {
            output_formatter.success("Project init succeeded".to_string());
            futures::future::ok(1)
        }
        Err(e) => {
            output_formatter.error(format!("Project init failed: {}", e));
            futures::future::ok(0)
        }
    }
}

async fn build(project: std::path::PathBuf, runtime: RuntimeOption, output_formatter: Box<dyn OutputFormatter>) -> Ready<Result<i32, ()>> {
    let result = build_project(project, runtime, &output_formatter).await;

    match result {
        Ok(_) => {
            output_formatter.success("Project build succeeded".to_string());
            futures::future::ok(1)
        }
        Err(e) => {
            output_formatter.error(format!("Project build failed: {}", e));
            futures::future::ok(0)
        }
    }
}

async fn cleanup(runtime: RuntimeOption, output_formatter: Box<dyn OutputFormatter>) -> Ready<Result<i32, ()>> {
    let result = cleanup_resources(runtime, &output_formatter).await;

    match result {
        Ok(_) => {
            output_formatter.success("Cleanup succeeded".to_string());
            futures::future::ok(1)
        }
        Err(e) => {
            output_formatter.error(format!("Cleanup failed: {}", e));
            futures::future::ok(0)
        }
    }
}

async fn test(output_formatter: Box<dyn OutputFormatter>) -> Ready<Result<i32, ()>> {
    let result = core_test().await;

    match result {
        Ok(_) => {
            output_formatter.success("Test succeeded".to_string());
            futures::future::ok(1)
        }
        Err(e) => {
            output_formatter.error(format!("Test failed: {}", e));
            futures::future::ok(0)
        }
    }
}
