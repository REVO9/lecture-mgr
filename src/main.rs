use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use clap::CommandFactory;
use clap::Parser;
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

fn main() -> eyre::Result<()> {
    CompleteEnv::with_factory(|| cli::Cli::command().name("lecture-mgr")).complete();

    color_eyre::install()?;

    let args = cli::Cli::parse();
    let mut config = Config::get(true)?;
    args.semester.inspect(|s| config.current_semester = *s);

    let mut app = App::new(args, config)?;
    app.run()?;

    Ok(())
}

struct App {
    args: cli::Cli,
    config: Config,
    semester_dir: PathBuf,
    repo: git2::Repository,
}

impl App {
    fn new(args: cli::Cli, config: Config) -> eyre::Result<Self> {
        let semester_dir = get_semester_dir(&config)?;
        let repo = git2::Repository::init(&config.folder).wrap_err("failed to innit git")?;

        Ok(Self {
            args: args,
            config: config,
            repo,
            semester_dir: semester_dir,
        })
    }

    fn run(&mut self) -> eyre::Result<()> {
        let no_commits = self.repo.head().is_err();

        if no_commits {
            println!("no commits yet, committing all files");
            process::Command::new("git")
                .args(["-C", self.config.folder.to_str().unwrap(), "add", "-A"])
                .output()
                .wrap_err("failed to add files to git")?;
            process::Command::new("git")
                .args([
                    "-C",
                    self.config.folder.to_str().unwrap(),
                    "commit",
                    "-m",
                    "init",
                ])
                .output()
                .wrap_err("failed init commit")?;
        }

        match self.args.command {
            Command::Commit { .. } => self.commit(),
            Command::Homepage { .. } => self.homepage(),
            Command::Script { .. } => self.script(),
            Command::Notes { .. } => self.notes(),
            Command::Path { .. } => {
                println!(
                    "{}",
                    self.lecture_dir()?
                        .to_str()
                        .expect("path is not valid Unicode")
                );
                Ok(())
            }
            Command::Lectures => {
                let lectures = get_lectures(&self.semester_dir)?;
                let string = lectures.join("\n");
                println!("{}", string);
                Ok(())
            }
            Command::Init { ref lecture } => {
                let mut lecture_dir = self.semester_dir.clone();
                lecture_dir.push(lecture);
                fs::create_dir(&lecture_dir)?;
                let lecture = lecture::Lecture::get(&lecture_dir)?;
                println!("created lecture {:?} at {:?}", lecture.name, lecture_dir);
                Ok(())
            }
        }
    }

    fn get_lecture(&self) -> Result<Lecture, eyre::Error> {
        let lecture_name = match self.args.command {
            Command::Commit { ref lecture }
            | Command::Homepage { ref lecture }
            | Command::Script { ref lecture }
            | Command::Notes { ref lecture }
            | Command::Path { ref lecture } => match lecture.clone() {
                Some(l) => l,
                None => self.prompt_lecutre_name()?,
            },
            _ => self.prompt_lecutre_name()?,
        };
        let mut lecture_dir = PathBuf::from(&self.semester_dir);
        lecture_dir.push(&lecture_name);
        eyre::ensure!(
            lecture_dir.exists(),
            "lecture '{lecture_name}' does not exist"
        );
        let lecture = Lecture::get(&lecture_dir).wrap_err("failed to get lecture")?;
        Ok(lecture)
    }

    fn script(&self) -> eyre::Result<()> {
        let lecture = self.get_lecture()?;
        open::that_detached(
            lecture
                .script_url
                .clone()
                .ok_or_eyre("lecture does not have a script url")?,
        )
        .wrap_err("failed to open script")?;

        println!("opening script for '{}'", lecture.name);

        Ok(())
    }

    fn homepage(&self) -> eyre::Result<()> {
        let lecture = self.get_lecture()?;
        open::that_detached(
            lecture
                .homepage_url
                .clone()
                .ok_or_eyre("lecture.as_ref().unwrap() does not have a homepage url")?,
        )
        .wrap_err("failed to open lecture.as_ref().unwrap() homepage")?;
        println!("opening homepage for '{}'", lecture.name);

        Ok(())
    }

    fn commit(&self) -> eyre::Result<()> {
        let lecture = self.get_lecture()?;
        let path = self.lecture_dir()?;
        let output = process::Command::new("git")
            .args(&[
                "-C",
                self.config.folder.to_str().unwrap(),
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

        let commit_msg = format!(
            "semester-{}: ,{}",
            self.config.current_semester, lecture.name,
        );
        let output = process::Command::new("git")
            .args(&[
                "-C",
                self.config.folder.to_str().unwrap(),
                "commit",
                "-m",
                &commit_msg,
            ])
            .output()
            .wrap_err("failed to add changes")?;

        if !output.status.success() {
            return Err(
                eyre::eyre!("stderr:\n{}", String::from_utf8(output.stderr)?)
                    .wrap_err("failed to commit changes"),
            );
        }

        println!("committed '{}'", lecture.name);

        Ok(())
    }

    fn notes(&self) -> Result<(), eyre::Error> {
        let lecture = self.get_lecture()?;
        const COMPILE_ERROR: &'static str = "failed to compile notes";
        let mut handle = process::Command::new("sh")
            .env("LECTURE_DIR", self.lecture_dir()?)
            .arg("-c")
            .arg(lecture.compile_notes_cmd.as_str())
            .spawn()
            .wrap_err(COMPILE_ERROR)?;

        let exit_status = handle.wait().wrap_err(COMPILE_ERROR)?;

        if !exit_status.success() {
            bail!(COMPILE_ERROR)
        }

        println!("compiled '{}'", lecture.name);

        const SHOW_ERROR: &'static str = "failed to open notes";
        let path = self.compiled_notes_path()?;

        if let Ok(Fork::Child) = fork::daemon(false, false) {
            let mut handle = process::Command::new("sh")
                .env("COMPILED_NOTES_PATH", path)
                .arg("-c")
                .arg(lecture.show_compiled_notes_cmd.as_str())
                .spawn()
                .wrap_err(SHOW_ERROR)?;

            let exit_status = handle.wait().wrap_err(SHOW_ERROR)?;

            if !exit_status.success() {
                bail!(SHOW_ERROR)
            }
        }

        Ok(())
    }

    fn lecture_dir(&self) -> eyre::Result<PathBuf> {
        let mut path = self.semester_dir.clone();
        path.push(&self.get_lecture()?.name);
        Ok(path)
    }

    fn compiled_notes_path(&self) -> eyre::Result<String> {
        let lecture = self.get_lecture()?;
        let output = process::Command::new("sh")
            .env("LECTURE_DIR", self.lecture_dir()?)
            .arg("-c")
            .arg(format!("echo {}", lecture.compiled_notes_path.clone()))
            .output()
            .unwrap();
        let path = String::from_utf8(output.stdout)?;
        Ok(path)
    }

    fn prompt_lecutre_name(&self) -> eyre::Result<String> {
        let lecture_name = inquire::Select::new(
            "select lecture",
            get_lectures(&self.semester_dir).wrap_err("failed to get lectures")?,
        )
        .prompt()
        .wrap_err("prompt failed")?;

        Ok(lecture_name)
    }
}

fn get_semester_dir(config: &Config) -> eyre::Result<PathBuf> {
    let mut dir = config.folder.clone();
    dir.extend(&PathBuf::from(format!(
        "semester-{}",
        config.current_semester
    )));
    if !dir.exists() {
        bail!("directory {dir:?} does not exist")
    }
    Ok(dir)
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
