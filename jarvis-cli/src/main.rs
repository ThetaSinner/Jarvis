use jarvis_core::{validate_project};
use structopt::StructOpt;
use colored::Colorize;

#[derive(StructOpt)]
/// The Jarvis CLI
struct Cli {
    #[structopt(long)]
    /// Validate the project configuration
    validate: bool,

    #[structopt(long, parse(from_os_str))]
    /// The project to use
    project: std::path::PathBuf,
}

fn main() {
    let args = Cli::from_args();

    let mut exit_code = 0;

    if args.validate {
        println!("Start validation.");

        let (msg, validation_success) = validate_project(args.project);
        if validation_success {
            println!("{} {}", gh_emoji::get("+1").unwrap(), msg.bright_green())
        } else {
            println!("{} {}", gh_emoji::get("-1").unwrap(), msg.bright_red());
            exit_code = 1
        }
    }

    std::process::exit(exit_code)
}
