use jarvis_core::OutputFormatter;
use colored::Colorize;

pub struct CliOutputFormatter {}

impl OutputFormatter for CliOutputFormatter {
    fn print(&self, msg: String) {
        println!("> {}", msg.as_str().bold());
    }

    fn success(&self, msg: String) {
        println!("{} {}", gh_emoji::get("tada").unwrap(), msg.as_str().bold().green());
    }

    fn error(&self, msg: String) {
        println!("{} {}", gh_emoji::get("exclamation").unwrap(), msg.as_str().bold().red());
    }

    fn background(&self, msg: String) {
        println!("> {}", msg.as_str().dimmed());
    }
}
