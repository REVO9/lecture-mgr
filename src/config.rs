use color_eyre::eyre;
use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub current_semester: u8,
    pub browser_cmd: String,
}

impl Config {
    pub fn get() -> eyre::Result<Self> {
        let default_config = include_str!("../.config/default_config.toml");
        let config_file_path = config_file_path();

        assert_file_exist(config_file_path.as_ref(), default_config)?;

        let config = config::Config::builder()
            .add_source(config::File::from_str(
                default_config,
                config::FileFormat::Toml,
            ))
            .add_source(config::File::with_name(config_file_path.to_str().unwrap()))
            .build()
            .wrap_err("failed to build config")?;

        let config: Self = config
            .try_deserialize()
            .wrap_err("failed to deserialize config")?;

        Ok(config)
    }
}

fn config_file_path() -> PathBuf {
    let mut config_file = PathBuf::from(
        std::env::var("XDG_CONFIG_HOME")
            .or(std::env::var("HOME").map(|home| format!("{}/.config", home)))
            .expect("$neither $XDG_CONFIG_HOME nor $HOME are set"),
    );
    config_file.push("lecture-mgr/config.toml");
    config_file
}

fn assert_file_exist(config_file: &Path, default_config: &str) -> eyre::Result<()> {
    if !config_file.exists() {
        println!("no config found");
        println!("creating new config file at {:?}", config_file);
        if let Some(config_dir) = config_file.parent() {
            std::fs::create_dir_all(&config_dir)?;
        }
        let mut file = File::create(&config_file).wrap_err(eyre::eyre!(
            "Failed to create config file at {:?}",
            config_file,
        ))?;

        write!(file, "{}", default_config).map_err(|err| {
            eyre::eyre!("Failed to write default config to new config. Reason: {err}")
        })?;
    }

    Ok(())
}
