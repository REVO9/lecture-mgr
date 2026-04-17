use clap::Parser;
use clap::Subcommand;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

use crate::get_lectures;
use crate::get_semester_dir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,

    #[arg(long, short, add = ArgValueCompleter::new(lecture_completer))]
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
}

fn lecture_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let completions = vec![];
    let Some(current) = current.to_str() else {
        return completions;
    };

    let Ok(semester_dir) = crate::Config::get(false).and_then(|config| get_semester_dir(&config))
    else {
        return completions;
    };

    let Ok(lectures) = get_lectures(&semester_dir) else {
        return completions;
    };

    lectures
        .into_iter()
        .filter(|lecture_name| lecture_name.starts_with(current))
        .map(|lecture_name| CompletionCandidate::new(lecture_name))
        .collect()
}
