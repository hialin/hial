use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use mongodb::options::oidc::{CallbackContext, IdpServerInfo, IdpServerResponse};
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use tiny_http::{Response, Server, StatusCode};
use url::Url;

use super::{env::OidcEnvConfig, token_store};

const OIDC_SCOPE_OPENID: &str = "openid";
const OIDC_SCOPE_OFFLINE_ACCESS: &str = "offline_access";

#[derive(Debug, Deserialize)]
struct OpenIdConfiguration {
    authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct OidcTokenResponse {
    access_token: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
}

pub(super) fn run_human_oidc_callback(
    context: CallbackContext,
    env: &OidcEnvConfig,
) -> mongodb::error::Result<IdpServerResponse> {
    let idp_info = context
        .idp_info
        .ok_or_else(|| oidc_error("MONGODB-OIDC callback missing idp_info"))?;
    let client_id = idp_info
        .client_id
        .as_deref()
        .ok_or_else(|| oidc_error("MONGODB-OIDC callback missing client_id"))?;
    let openid = get_openid_configuration(&idp_info.issuer)?;
    let refresh_token = if context.refresh_token.as_deref().is_some() {
        context.refresh_token
    } else {
        token_store::load_refresh_token(&env.token_file, client_id).map_err(oidc_error)?
    };
    if let Some(refresh_token) = refresh_token
        && let Ok(response) =
            exchange_refresh_token(&openid.token_endpoint, client_id, &refresh_token)
    {
        if let Some(refresh_token) = response.refresh_token.as_deref() {
            let _ = token_store::save_refresh_token(&env.token_file, client_id, refresh_token);
        }
        return Ok(to_idp_response(response));
    }
    let response = run_interactive_flow(context.timeout, &idp_info, &openid, client_id, env)?;
    if let Some(refresh_token) = response.refresh_token.as_deref() {
        let _ = token_store::save_refresh_token(&env.token_file, client_id, refresh_token);
    }
    Ok(to_idp_response(response))
}

fn run_interactive_flow(
    timeout: Option<Instant>,
    idp_info: &IdpServerInfo,
    openid: &OpenIdConfiguration,
    client_id: &str,
    env: &OidcEnvConfig,
) -> mongodb::error::Result<OidcTokenResponse> {
    let callback_url = Url::parse(&env.callback_url)
        .map_err(|err| oidc_error(format!("invalid OIDC callback URL: {err}")))?;
    let redirect_uri = callback_url.to_string();
    let scopes = build_scopes(idp_info);
    let state = Alphanumeric.sample_string(&mut rand::rng(), 24);
    let auth_url = build_auth_url(
        &openid.authorization_endpoint,
        client_id,
        &redirect_uri,
        &state,
        &scopes,
    )?;
    println!("Please visit this URL to authenticate: {auth_url}");
    let auth_code = listen_for_auth_code(timeout, &callback_url, &state)?;
    exchange_auth_code(&openid.token_endpoint, client_id, &redirect_uri, &auth_code)
}

fn listen_for_auth_code(
    timeout: Option<Instant>,
    callback_url: &Url,
    state: &str,
) -> mongodb::error::Result<String> {
    let callback_host = callback_url
        .host_str()
        .ok_or_else(|| oidc_error("OIDC callback URL is missing host"))?;
    let callback_port = callback_url
        .port_or_known_default()
        .ok_or_else(|| oidc_error("OIDC callback URL is missing port"))?;
    let callback_path = callback_url.path();
    let server = Server::http(format!("{callback_host}:{callback_port}"))
        .map_err(|err| oidc_error(format!("failed to bind OIDC callback listener: {err}")))?;
    let deadline = timeout.unwrap_or_else(|| Instant::now() + Duration::from_secs(300));
    while Instant::now() < deadline {
        match server.recv_timeout(Duration::from_millis(50)) {
            Ok(Some(request)) => {
                let callback_url = Url::parse(&format!("http://localhost{}", request.url()))
                    .map_err(|err| oidc_error(format!("invalid callback path: {err}")))?;
                if callback_url.path() != callback_path {
                    let _ = request.respond(
                        Response::from_string("not found").with_status_code(StatusCode(404)),
                    );
                    continue;
                }
                let _ = request.respond(
                    Response::from_string("authorization code received, you can close.")
                        .with_status_code(StatusCode(200)),
                );
                let query = callback_url.query_pairs().collect::<BTreeMap<_, _>>();
                if query.get("state").is_none_or(|value| value != state) {
                    return Err(oidc_error("OIDC callback state mismatch"));
                }
                if let Some(code) = query.get("code") {
                    return Ok(code.to_string());
                }
                return Err(oidc_error("OIDC callback missing authorization code"));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(oidc_error(format!(
                    "failed to accept OIDC callback request: {err}"
                )));
            }
        }
    }
    Err(oidc_error("timed out waiting for OIDC callback"))
}

fn exchange_auth_code(
    token_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    auth_code: &str,
) -> mongodb::error::Result<OidcTokenResponse> {
    let response = reqwest::blocking::Client::new()
        .post(token_endpoint)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(urlencode_form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("code", auth_code),
        ]))
        .send()
        .map_err(|err| oidc_error(format!("failed to exchange auth code: {err}")))?;
    parse_token_response(response)
}

fn exchange_refresh_token(
    token_endpoint: &str,
    client_id: &str,
    refresh_token: &str,
) -> mongodb::error::Result<OidcTokenResponse> {
    let response = reqwest::blocking::Client::new()
        .post(token_endpoint)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(urlencode_form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
        ]))
        .send()
        .map_err(|err| oidc_error(format!("failed to refresh OIDC token: {err}")))?;
    parse_token_response(response)
}

fn parse_token_response(
    response: reqwest::blocking::Response,
) -> mongodb::error::Result<OidcTokenResponse> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(oidc_error(format!(
            "OIDC token endpoint returned {status}: {body}"
        )));
    }
    response
        .json::<OidcTokenResponse>()
        .map_err(|err| oidc_error(format!("failed to parse token endpoint response: {err}")))
}

fn get_openid_configuration(issuer: &str) -> mongodb::error::Result<OpenIdConfiguration> {
    let issuer = issuer.trim_end_matches('/');
    let url = format!("{issuer}/.well-known/openid-configuration");
    reqwest::blocking::Client::new()
        .get(url)
        .send()
        .map_err(|err| oidc_error(format!("failed to fetch OIDC metadata: {err}")))?
        .json::<OpenIdConfiguration>()
        .map_err(|err| oidc_error(format!("failed to parse OIDC metadata: {err}")))
}

fn build_auth_url(
    authorization_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    scopes: &str,
) -> mongodb::error::Result<String> {
    let mut url = Url::parse(authorization_endpoint)
        .map_err(|err| oidc_error(format!("invalid authorization endpoint: {err}")))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", scopes)
        .append_pair("state", state);
    Ok(url.to_string())
}

fn build_scopes(idp_info: &IdpServerInfo) -> String {
    let mut scopes = vec![
        String::from(OIDC_SCOPE_OPENID),
        String::from(OIDC_SCOPE_OFFLINE_ACCESS),
    ];
    if let Some(request_scopes) = idp_info.request_scopes.as_ref() {
        for scope in request_scopes {
            if !scopes.iter().any(|item| item == scope) {
                scopes.push(scope.clone());
            }
        }
    }
    scopes.join(" ")
}

fn to_idp_response(token: OidcTokenResponse) -> IdpServerResponse {
    IdpServerResponse::builder()
        .access_token(token.access_token)
        .expires(
            token
                .expires_in
                .map(|seconds| Instant::now() + Duration::from_secs(seconds)),
        )
        .refresh_token(token.refresh_token)
        .build()
}

fn urlencode_form(values: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in values {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

fn oidc_error(message: impl Into<String>) -> mongodb::error::Error {
    mongodb::error::Error::custom(message.into())
}
