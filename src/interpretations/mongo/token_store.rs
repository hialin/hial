use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
struct TokenConfig {
    #[serde(default)]
    tokens: BTreeMap<String, String>,
}

pub(super) fn load_refresh_token(path: &Path, client_id: &str) -> Result<Option<String>, String> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read token config: {err}")),
    };
    let config: TokenConfig =
        serde_yaml::from_str(&data).map_err(|err| format!("failed to parse token config: {err}"))?;
    Ok(config.tokens.get(client_id).cloned())
}

pub(super) fn save_refresh_token(path: &Path, client_id: &str, refresh_token: &str) -> Result<(), String> {
    let mut config = match fs::read_to_string(path) {
        Ok(data) => serde_yaml::from_str::<TokenConfig>(&data)
            .map_err(|err| format!("failed to parse existing token config: {err}"))?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => TokenConfig::default(),
        Err(err) => return Err(format!("failed to read token config: {err}")),
    };
    config
        .tokens
        .insert(client_id.to_string(), refresh_token.to_string());
    let data = serde_yaml::to_string(&config)
        .map_err(|err| format!("failed to serialize token config: {err}"))?;
    fs::write(path, data).map_err(|err| format!("failed to write token config: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{load_refresh_token, save_refresh_token};

    #[test]
    fn load_missing_file_returns_none() {
        let path = std::env::temp_dir().join(format!(
            "hial-mongo-token-store-missing-{}.yaml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let token = load_refresh_token(&path, "client-id").expect("load should not fail");
        assert!(token.is_none());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = std::env::temp_dir().join(format!(
            "hial-mongo-token-store-roundtrip-{}.yaml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        save_refresh_token(&path, "client-id", "refresh-token").expect("save should succeed");
        let token = load_refresh_token(&path, "client-id").expect("load should succeed");
        assert_eq!(token.as_deref(), Some("refresh-token"));
        let _ = std::fs::remove_file(&path);
    }
}
