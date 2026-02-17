use std::{env, path::PathBuf};

const OIDC_HUMAN_ENV: &str = "HIAL_MONGO_OIDC_HUMAN";
const OIDC_CALLBACK_URL_ENV: &str = "HIAL_MONGO_OIDC_CALLBACK_URL";

const DEFAULT_TOKEN_FILE: &str = "mongo/oidc-token.yaml";
const DEFAULT_CALLBACK_URL: &str = "http://127.0.0.1:27097/redirect";

#[derive(Clone, Debug)]
pub(super) struct OidcEnvConfig {
    pub(super) enabled: bool,
    pub(super) token_file: PathBuf,
    pub(super) callback_url: String,
}

pub(super) fn oidc_env_config() -> OidcEnvConfig {
    let callback_url = env::var(OIDC_CALLBACK_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| String::from(DEFAULT_CALLBACK_URL));
    OidcEnvConfig {
        enabled: env::var(OIDC_HUMAN_ENV).is_ok_and(|value| is_truthy(&value)),
        token_file: default_token_file().unwrap_or_else(|| PathBuf::from(DEFAULT_TOKEN_FILE)),
        callback_url,
    }
}

fn default_token_file() -> Option<PathBuf> {
    crate::config::config_dir().map(|mut path| {
        path.push(DEFAULT_TOKEN_FILE);
        path
    })
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use std::{env, sync::Mutex};

    use super::{DEFAULT_CALLBACK_URL, DEFAULT_TOKEN_FILE, OIDC_CALLBACK_URL_ENV, oidc_env_config};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn uses_default_callback_url_when_missing() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let previous = env::var(OIDC_CALLBACK_URL_ENV).ok();
        unsafe { env::remove_var(OIDC_CALLBACK_URL_ENV) };
        let config = oidc_env_config();
        assert_eq!(config.callback_url, DEFAULT_CALLBACK_URL);
        if let Some(value) = previous {
            unsafe { env::set_var(OIDC_CALLBACK_URL_ENV, value) };
        } else {
            unsafe { env::remove_var(OIDC_CALLBACK_URL_ENV) };
        }
    }

    #[test]
    fn reads_callback_url_from_env() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let previous = env::var(OIDC_CALLBACK_URL_ENV).ok();
        unsafe { env::set_var(OIDC_CALLBACK_URL_ENV, "http://127.0.0.1:3000/callback") };
        let config = oidc_env_config();
        assert_eq!(config.callback_url, "http://127.0.0.1:3000/callback");
        if let Some(value) = previous {
            unsafe { env::set_var(OIDC_CALLBACK_URL_ENV, value) };
        } else {
            unsafe { env::remove_var(OIDC_CALLBACK_URL_ENV) };
        }
    }

    #[test]
    fn uses_default_config_dir_for_default_token_file() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let config = oidc_env_config();
        if let Some(mut expected) = crate::config::config_dir() {
            expected.push(DEFAULT_TOKEN_FILE);
            assert_eq!(config.token_file, expected);
        } else {
            assert_eq!(config.token_file, std::path::PathBuf::from(DEFAULT_TOKEN_FILE));
        }
    }
}
