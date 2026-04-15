use std::path::Path;
use std::path::PathBuf;
use std::process;

use clap::Parser;
use clap::Subcommand;
use clap::builder::TypedValueParser;
use color_eyre::Section;
use color_eyre::eyre;
use eyre::Context;
use eyre::OptionExt;
use eyre::bail;

use crate::config::Config;
use crate::lecture::Lecture;

mod config;
mod lecture;

type LectureName = String;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(long, short)]
    lecture: Option<LectureName>,
}

#[derive(Debug, Subcommand, Clone, Copy)]
enum Command {
    /// Commit your changes. These changes will be marked with your current lecture
    Commit,
    /// Open this lectures homepage
    Homepage,
    /// Open this lectures script
    Script,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let config = Config::get()?;

    let mut app = App::new(args, config)?;
    app.run()?;

    Ok(())
}

struct App {
    args: Args,
    config: Config,
    lecture: Lecture,
    semester_dir: PathBuf,
}

impl App {
    fn new(args: Args, config: Config) -> eyre::Result<Self> {
        let home_dir: PathBuf = std::env::var("HOME").expect("$HOME not set").into();
        let mut semester_dir = home_dir.clone();
        semester_dir.extend(&PathBuf::from(format!(
            "Documents/semester-{}",
            config.current_semester
        )));
        if !semester_dir.exists() {
            bail!("directory {semester_dir:?} does not exist")
        }

        let repo = git2::Repository::init(&semester_dir).wrap_err("failed to innit git")?;
        let no_commits = repo.head().is_err();

        if no_commits {
            println!("no commits yet, committing all files");
            process::Command::new("git")
                .args(["-C", semester_dir.to_str().unwrap(), "add", "-A"])
                .output()
                .wrap_err("failed to add files to git")?;
            process::Command::new("git")
                .args(["-C", semester_dir.to_str().unwrap(), "commit", "-m", "init"])
                .output()
                .wrap_err("failed init commit")?;
        }

        // let lecture_name = args
        //     .lecture
        //     .clone()
        //     .ok_or_eyre("lecture is currently unknown")
        //     .suggestion("use '--lecture' to set the lecture")?;

        let lecture_name = match args.lecture {
            Some(name) => name,
            None => {}
        };

        let mut lecture_dir = PathBuf::from(&semester_dir);
        lecture_dir.push(&lecture_name);

        eyre::ensure!(
            lecture_dir.exists(),
            "lecture '{lecture_name}' does not exist"
        );
        let lecture = Lecture::get(&lecture_dir).wrap_err("failed to get lecture")?;

        Ok(Self {
            args: args,
            config: config,
            lecture,
            semester_dir: semester_dir,
        })
    }

    fn run(&mut self) -> eyre::Result<()> {
        match self.args.command {
            Command::Commit => self.commit(),
            Command::Homepage => self.homepage(),
            Command::Script => self.script(),
        }
    }

    fn script(&self) -> eyre::Result<()> {
        process::Command::new("sh")
            .env(
                "URL",
                self.lecture
                    .script_url
                    .clone()
                    .ok_or_eyre("lecture does not have a script url")?,
            )
            .arg("-c")
            .arg(self.config.browser_cmd.as_str())
            .spawn()
            .wrap_err("failed to open script")?;

        Ok(())
    }

    fn homepage(&self) -> eyre::Result<()> {
        process::Command::new("sh")
            .env(
                "URL",
                self.lecture
                    .homepage_url
                    .clone()
                    .ok_or_eyre("lecture does not have a homepage url")?,
            )
            .arg("-c")
            .arg(self.config.browser_cmd.as_str())
            .spawn()
            .wrap_err("failed to open lecture homepage")?;

        Ok(())
    }

    fn commit(&self) -> eyre::Result<()> {
        process::Command::new("git")
            .args(&["add", "-C", self.semester_dir.to_str().unwrap(), "-A"])
            .output()
            .wrap_err("failed to add changes")?;

        let commit_msg = format!("{}", self.lecture.name);
        process::Command::new("git")
            .args(&["commit", "-m", &commit_msg])
            .output()
            .wrap_err("failed to add changes")?;

        Ok(())
    }
}

fn get_lectures(semster_dir: &Path) -> Vec<String> {}
