use jarvis_core::{validate_project};
use structopt::StructOpt;
use colored::Colorize;

#[derive(StructOpt)]
/// The Jarvis CLI
struct Cli {
    #[structopt(subcommand)]
    cmd: SubCommands
}

#[derive(StructOpt)]
enum SubCommands {
    Validate {
        #[structopt(long, parse(from_os_str))]
        /// The project to use
        project: std::path::PathBuf,
    }
}

fn main() {
    let args = Cli::from_args();

    let exit_code ;

    match args.cmd {
        SubCommands::Validate { project} => {
            exit_code = validate(project);
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
