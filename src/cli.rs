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
}

pub type LectureName = String;

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Commit your changes. These changes will be marked with your current lecture
    Commit {
        #[arg(add = ArgValueCompleter::new(lecture_completer))]
        lecture: Option<LectureName>,
    },
    /// Open this lectures homepage
    Homepage {
        #[arg(add = ArgValueCompleter::new(lecture_completer))]
        lecture: Option<LectureName>,
    },
    /// Open this lectures script
    Script {
        #[arg(add = ArgValueCompleter::new(lecture_completer))]
        lecture: Option<LectureName>,
    },
    /// Compile and show notes for this lecture
    Notes {
        #[arg(add = ArgValueCompleter::new(lecture_completer))]
        lecture: Option<LectureName>,
    },
    /// Show the path of the lecture
    Path {
        #[arg(add = ArgValueCompleter::new(lecture_completer))]
        lecture: Option<LectureName>,
    },
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
