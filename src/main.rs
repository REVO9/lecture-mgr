use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use clap::CommandFactory;
use clap::Parser;
use clap::ValueEnum;
use clap_complete::CompleteEnv;
use color_eyre::eyre;
use eyre::Context;
use eyre::OptionExt;
use eyre::bail;
use fork::Fork;

use crate::cli::Command;
use crate::config::Config;
use crate::lecture::Lecture;

mod cli;
mod config;
mod lecture;

type LectureName = String;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    CompleteEnv::with_factory(|| cli::Cli::command().name("lecture-mgr"))  
        .complete();  

    let args = cli::Cli::parse();
    if let Command::Generate { shell } = args.command {
        let mut cmd = cli::Cli::command();
        eprintln!("Generating completion file for {shell:?}...");
        cli::print_completions(shell, &mut cmd);

        return Ok(());
    }

    let config = Config::get(true)?;

    let mut app = App::new(args, config)?;
    app.run()?;

    Ok(())
}

struct App {
    args: cli::Cli,
    config: Config,
    lecture: Lecture,
    semester_dir: PathBuf,
}

impl App {
    fn new(args: cli::Cli, config: Config) -> eyre::Result<Self> {
        let semester_dir = get_semester_dir(&config)?;

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
            Some(ref name) => name,
            None => &inquire::Select::new(
                "select lecture",
                get_lectures(&semester_dir).wrap_err("failed to get lectures")?,
            )
            .prompt()
            .wrap_err("prompt failed")?,
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
            Command::Notes => self.notes(),
            Command::Generate { .. } => unreachable!(),
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
        println!("opening script for '{}'", self.lecture.name);

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
        println!("opening homepage for '{}'", self.lecture.name);

        Ok(())
    }

    fn commit(&self) -> eyre::Result<()> {
        let path = self.lecture_dir();
        let output = process::Command::new("git")
            .args(&[
                "-C",
                self.semester_dir.to_str().unwrap(),
                "add",
                path.to_str().unwrap(),
            ])
            .output()
            .wrap_err("failed to add changes")?;

        if !output.status.success() {
            return Err(
                eyre::eyre!("stderr:\n{}", String::from_utf8(output.stderr)?)
                    .wrap_err("failed to add changes"),
            );
        }

        let commit_msg = format!("{}", self.lecture.name);
        let output = process::Command::new("git")
            .args(&["-C", path.to_str().unwrap(), "commit", "-m", &commit_msg])
            .output()
            .wrap_err("failed to add changes")?;

        if !output.status.success() {
            return Err(
                eyre::eyre!("stderr:\n{}", String::from_utf8(output.stderr)?)
                    .wrap_err("failed to commit changes"),
            );
        }

        println!("committed '{}'", self.lecture.name);

        Ok(())
    }

    fn notes(&self) -> Result<(), eyre::Error> {
        const COMPILE_ERROR: &'static str = "failed to compile notes";
        let mut handle = process::Command::new("sh")
            .env("LECTURE_DIR", self.lecture_dir())
            .arg("-c")
            .arg(self.lecture.compile_notes_cmd.as_str())
            .spawn()
            .wrap_err(COMPILE_ERROR)?;

        let exit_status = handle.wait().wrap_err(COMPILE_ERROR)?;

        if !exit_status.success() {
            bail!(COMPILE_ERROR)
        }

        println!("compiled '{}'", self.lecture.name);

        const SHOW_ERROR: &'static str = "failed to open notes";
        let path = self.compiled_notes_path()?;

        if let Ok(Fork::Child) = fork::daemon(false, false) {
            let mut handle = process::Command::new("sh")
                .env("COMPILED_NOTES_PATH", path)
                .arg("-c")
                .arg(self.lecture.show_compiled_notes_cmd.as_str())
                .spawn()
                .wrap_err(SHOW_ERROR)?;

            let exit_status = handle.wait().wrap_err(SHOW_ERROR)?;

            if !exit_status.success() {
                bail!(SHOW_ERROR)
            }
        }

        Ok(())
    }

    fn lecture_dir(&self) -> PathBuf {
        let mut path = self.semester_dir.clone();
        path.push(&self.lecture.name);
        path
    }

    fn compiled_notes_path(&self) -> eyre::Result<String> {
        let output = process::Command::new("sh")
            .env("LECTURE_DIR", self.lecture_dir())
            .arg("-c")
            .arg(format!("echo {}", self.lecture.compiled_notes_path.clone()))
            .output()
            .unwrap();
        let path = String::from_utf8(output.stdout)?;
        Ok(path)
    }
}

fn get_semester_dir(config: &Config) -> eyre::Result<PathBuf> {
    let home_dir: PathBuf = std::env::var("HOME").expect("$HOME not set").into();
    let mut semester_dir = home_dir.clone();
    semester_dir.extend(&PathBuf::from(format!(
        "Documents/semester-{}",
        config.current_semester
    )));
    if !semester_dir.exists() {
        bail!("directory {semester_dir:?} does not exist")
    }
    Ok(semester_dir)
}

fn get_lectures(semester_dir: &Path) -> eyre::Result<Vec<String>> {
    let mut vec = Vec::new();

    for entry in fs::read_dir(semester_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let mut lecture_config_path = entry.path();
        lecture_config_path.push("lecture.toml");
        if !lecture_config_path.exists() {
            continue;
        }

        vec.push(
            entry
                .file_name()
                .to_str()
                .ok_or_eyre("failed to get file name")?
                .to_string(),
        )
    }

    Ok(vec)
}
