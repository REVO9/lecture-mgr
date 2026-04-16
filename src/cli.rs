use clap::Parser;
use clap::Subcommand;
use clap_complete::Generator;
use clap_complete::generate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,

    #[arg(long, short)]
    pub(crate) lecture: Option<LectureName>,
}

pub type LectureName = String;

#[derive(Debug, Subcommand, Clone, Copy)]
pub enum Command {
    /// Commit your changes. These changes will be marked with your current lecture
    Commit,
    /// Open this lectures homepage
    Homepage,
    /// Open this lectures script
    Script,
    /// Compile and show notes
    Notes,

    /// Outputs the completion file for given shell
    Generate{
        shell: clap_complete::Shell,
    },
}

pub fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut std::io::stdout(),
    );
}
