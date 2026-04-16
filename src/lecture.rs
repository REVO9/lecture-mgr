use std::io::{Read, Write};
use std::{fs, process};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

use color_eyre::eyre;
use eyre::Context;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Lecture {
    #[serde(skip_deserializing)]
    pub name: String,

    pub homepage_url: Option<String>,
    pub script_url: Option<String>,

    pub compile_notes_cmd: String,
    pub compiled_notes_path: String,
    pub show_compiled_notes_cmd: String,
}

impl Lecture {
    pub fn get(path: &Path) -> eyre::Result<Self> {
        let lecture_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let mut lecture_config = PathBuf::from(path);
        lecture_config.push("lecture.toml");

        let default_lecture_config = include_str!("../.config/default_lecture.toml");

        if !lecture_config.exists() {
            let mut file =
                File::create_new(&lecture_config).wrap_err("failed to create lecture.toml")?;
            write!(&mut file, "{}", default_lecture_config)
                .wrap_err("failed to write default_lecture_config")?;
        }

        let lecture: Lecture = config::Config::builder()
            .add_source(config::File::from_str(
                default_lecture_config,
                config::FileFormat::Toml,
            ))
            .add_source(config::File::with_name(lecture_config.to_str().unwrap()))
            .build()
            .wrap_err("reading lecture.toml failed")
            .and_then(|config| {
                config
                    .try_deserialize()
                    .wrap_err("deserializing lecture.toml failed")
            })?;

        Ok(Lecture {
            name: lecture_name,
            ..lecture
        })
    }
}
