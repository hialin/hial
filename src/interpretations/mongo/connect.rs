use futures::FutureExt;
use mongodb::{
    options::{AuthMechanism, ClientOptions, oidc::Callback},
    sync::Client,
};

use crate::api::{HErrKind, Res, caused};

use super::{env, oidc};

pub(super) fn connect_client(conn_str: &str) -> Res<Client> {
    let mut options = ClientOptions::parse(conn_str)
        .run()
        .map_err(|err| caused(HErrKind::Net, "mongo: cannot parse options", err))?;
    let oidc_env = env::oidc_env_config();
    if should_attach_human_oidc_callback(&options, oidc_env.enabled)
        && let Some(credential) = options.credential.as_mut()
    {
        let callback_env = oidc_env.clone();
        credential.oidc_callback = Callback::human(move |context| {
            let callback_env = callback_env.clone();
            async move { oidc::run_human_oidc_callback(context, &callback_env) }.boxed()
        });
    }
    Client::with_options(options).map_err(|err| caused(HErrKind::Net, "mongo: cannot connect", err))
}

fn should_attach_human_oidc_callback(options: &ClientOptions, oidc_human_enabled: bool) -> bool {
    oidc_human_enabled
        && matches!(
            options.credential.as_ref().and_then(|credential| credential.mechanism.as_ref()),
            Some(AuthMechanism::MongoDbOidc)
        )
}

#[cfg(test)]
mod tests {
    use mongodb::options::{AuthMechanism, Credential};

    use super::should_attach_human_oidc_callback;

    #[test]
    fn oidc_callback_enabled_for_oidc_mechanism_when_env_enabled() {
        let options = mongodb::options::ClientOptions::builder()
            .credential(Credential::builder().mechanism(AuthMechanism::MongoDbOidc).build())
            .build();
        assert!(should_attach_human_oidc_callback(&options, true));
    }

    #[test]
    fn oidc_callback_disabled_for_oidc_mechanism_when_env_disabled() {
        let options = mongodb::options::ClientOptions::builder()
            .credential(Credential::builder().mechanism(AuthMechanism::MongoDbOidc).build())
            .build();
        assert!(!should_attach_human_oidc_callback(&options, false));
    }

    #[test]
    fn oidc_callback_disabled_for_non_oidc_mechanism() {
        let options = mongodb::options::ClientOptions::builder()
            .credential(Credential::builder().mechanism(AuthMechanism::ScramSha256).build())
            .build();
        assert!(!should_attach_human_oidc_callback(&options, true));
    }
}
