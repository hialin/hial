use crate::api::*;
use serde::Deserialize;
use std::{fs, io::ErrorKind, path::PathBuf};

const MAIN_CONFIG_FILE: &str = "hial.yaml";
const PRELUDE_FILE: &str = "prelude.hial";

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
    pub mongo_oidc_human: Option<bool>,
}

pub(crate) fn config_dir() -> Res<PathBuf> {
    let config_dir = dirs::home_dir()
        .ok_or_else(|| ioerr("home dir not found"))?
        .join(".config");
    Ok(config_dir.join("hial"))
}

pub fn load_main_config() -> Res<MainConfig> {
    let path = config_dir()?.join(MAIN_CONFIG_FILE);
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

pub fn load_prelude_text() -> Res<Option<String>> {
    let path = config_dir()?.join(PRELUDE_FILE);
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(Some(contents)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(caused(
            HErrKind::IO,
            format!("cannot read prelude file: {}", path.display()),
            err,
        )),
    }
}
