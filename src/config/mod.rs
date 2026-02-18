use crate::api::*;
use serde::Deserialize;
use std::{fs, io::ErrorKind, path::PathBuf};

const MAIN_CONFIG_FILE: &str = "main.yaml";

#[derive(Copy, Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorPalette {
    #[default]
    Dark,
    Light,
    None,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct MainConfig {
    pub color_palette: Option<ColorPalette>,
}

pub(crate) fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|path| path.join("hial"))
}

pub fn load_main_config() -> Res<MainConfig> {
    let Some(path) = config_dir().map(|path| path.join(MAIN_CONFIG_FILE)) else {
        return Ok(MainConfig::default());
    };
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(MainConfig::default()),
        Err(err) => {
            return Err(caused(
                HErrKind::IO,
                format!("cannot read config file: {}", path.display()),
                err,
            ));
        }
    };
    serde_yaml::from_str(&contents).map_err(|err| {
        caused(
            HErrKind::InvalidFormat,
            format!("invalid config file: {}", path.display()),
            err,
        )
    })
}
