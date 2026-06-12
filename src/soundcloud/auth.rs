use oauth2::basic::{BasicClient, BasicTokenType};
use oauth2::{AccessToken, RefreshToken, StandardTokenResponse, reqwest};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, ConfigurationError, CsrfToken,
    PkceCodeChallenge, RedirectUrl, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};
use url::Url;

use crate::config;
use crate::constants;

/// Refresh the access token this many seconds before it actually expires, so
/// requests already in flight never race the expiry.
const EXPIRY_MARGIN_SECS: u64 = 60;
/// How long to wait for the user to approve access in their browser before
/// giving up on the login attempt.
const BROWSER_AUTH_TIMEOUT: Duration = Duration::from_secs(5 * 60);

type TokenResp = StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>;

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
    token_type: String,
    #[serde(default = "unix_now")]
    created_at: u64, // When the refresh token was first created
}

impl StoredToken {
    fn from_token_response(token: &TokenResp) -> Self {
        Self {
            access_token: token.access_token().secret().to_string(),
            refresh_token: token.refresh_token().map(|rt| rt.secret().to_string()),
            expires_at: token.expires_in().map(|d| unix_now() + d.as_secs()),
            token_type: "Bearer".to_string(), // SoundCloud uses Bearer tokens
            created_at: unix_now(),
        }
    }
}

#[derive(Clone)]
struct TokenStorage {
    file_path: PathBuf,
}

impl TokenStorage {
    fn new() -> Result<Self, AuthError> {
        let data_dir = config::get_data_dir();
        fs::create_dir_all(&data_dir)?;
        let file_path = data_dir.join("oauth_token.json");
        Ok(Self { file_path })
    }

    fn save_token(&self, token: &StoredToken) -> Result<(), AuthError> {
        let json = serde_json::to_string_pretty(token)?;
        fs::write(&self.file_path, json)?;
        info!("OAuth token saved to {}", self.file_path.display());
        Ok(())
    }

    fn load_token(&self) -> Result<Option<StoredToken>, AuthError> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.file_path)?;
        match serde_json::from_str::<StoredToken>(&content) {
            Ok(stored_token) => {
                info!("OAuth token loaded from {}", self.file_path.display());
                Ok(Some(stored_token))
            }
            Err(e) => {
                warn!(
                    "Failed to parse stored token: {}, clearing invalid token",
                    e
                );
                self.clear_token()?;
                Ok(None)
            }
        }
    }

    fn clear_token(&self) -> Result<(), AuthError> {
        if self.file_path.exists() {
            fs::remove_file(&self.file_path)?;
            info!("OAuth token cleared from {}", self.file_path.display());
        }
        Ok(())
    }
}

struct TokenState {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    expires_at: Option<u64>,
}

impl TokenState {
    fn needs_refresh(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| unix_now() + EXPIRY_MARGIN_SECS >= expires_at)
    }
}

/// Hands out a valid access token, transparently refreshing it shortly before
/// expiry. Clones share the same token state, so a refresh performed through
/// one clone is visible to all of them.
#[derive(Clone)]
pub struct TokenManager {
    storage: TokenStorage,
    state: Arc<Mutex<TokenState>>,
}

impl std::fmt::Debug for TokenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenManager")
            .field(
                "has_refresh_token",
                &self.state.lock().unwrap().refresh_token.is_some(),
            )
            .finish()
    }
}

impl TokenManager {
    fn from_stored(stored: StoredToken, storage: TokenStorage) -> Self {
        Self {
            storage,
            state: Arc::new(Mutex::new(TokenState {
                access_token: AccessToken::new(stored.access_token),
                refresh_token: stored.refresh_token.map(RefreshToken::new),
                expires_at: stored.expires_at,
            })),
        }
    }

    fn from_token_response(token: &TokenResp, storage: TokenStorage) -> Self {
        Self {
            storage,
            state: Arc::new(Mutex::new(TokenState {
                access_token: token.access_token().clone(),
                refresh_token: token.refresh_token().cloned(),
                expires_at: token.expires_in().map(|d| unix_now() + d.as_secs()),
            })),
        }
    }

    /// Get a valid access token, refreshing it first only when it is about to
    /// expire.
    pub async fn get_fresh_token(&mut self) -> Result<AccessToken, AuthError> {
        let refresh_token = {
            let state = self.state.lock().unwrap();
            if !state.needs_refresh() {
                return Ok(state.access_token.clone());
            }
            state.refresh_token.clone().ok_or_else(|| {
                AuthError::OAuth(
                    "Access token expired and no refresh token is available".to_string(),
                )
            })?
        };

        let new_token = refresh_access_token(&refresh_token).await?;
        info!("Refreshed OAuth token");
        self.storage
            .save_token(&StoredToken::from_token_response(&new_token))?;

        let mut state = self.state.lock().unwrap();
        state.access_token = new_token.access_token().clone();
        state.expires_at = new_token.expires_in().map(|d| unix_now() + d.as_secs());
        if let Some(refresh_token) = new_token.refresh_token() {
            state.refresh_token = Some(refresh_token.clone());
        }
        Ok(state.access_token.clone())
    }
}

/// Restore a session from a previously saved token, refreshing it when it has
/// expired. Returns `None` when a full browser login is required.
pub async fn try_cached_authentication() -> Option<TokenManager> {
    let storage = TokenStorage::new().ok()?;
    let stored = storage.load_token().ok().flatten()?;
    let mut manager = TokenManager::from_stored(stored, storage);

    match manager.get_fresh_token().await {
        Ok(_) => {
            info!("Restored session from cached OAuth token");
            Some(manager)
        }
        Err(e) => {
            warn!("Could not restore cached session: {}", e);
            None
        }
    }
}

/// Run the full OAuth2 authorization-code flow: open the user's default
/// browser on the SoundCloud consent page and wait for the redirect back to a
/// local listener.
pub async fn authenticate_in_browser() -> Result<TokenManager, AuthError> {
    let storage = TokenStorage::new()?;
    info!("Starting OAuth2 authentication flow");

    let client = BasicClient::new(ClientId::new(constants::CLIENT_ID.to_string()))
        .set_client_secret(ClientSecret::new(constants::CLIENT_SECRET.to_string()))
        .set_auth_uri(AuthUrl::new(constants::SOUNDCLOUD_AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(constants::SOUNDCLOUD_TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(constants::REDIRECT_URL.to_string())?);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Bind before opening the browser so a busy port fails fast instead of
    // leaving the user on a dead consent page.
    let listener = TcpListener::bind(redirect_listen_addr()?).await?;

    if let Err(e) = open::that_detached(auth_url.as_str()) {
        return Err(AuthError::Other(format!(
            "Could not open your browser ({}). Visit this URL to sign in: {}",
            e, auth_url
        )));
    }
    info!("Opened browser for SoundCloud authorization");

    let code = tokio::time::timeout(
        BROWSER_AUTH_TIMEOUT,
        wait_for_redirect(&listener, csrf_token.secret()),
    )
    .await
    .map_err(|_| {
        AuthError::OAuth("Timed out waiting for authorization in the browser".to_string())
    })??;

    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let token = client
        .exchange_code(code)
        .add_extra_param("client_id", constants::CLIENT_ID.as_str())
        .add_extra_param("client_secret", constants::CLIENT_SECRET.as_str())
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| AuthError::OAuth(e.to_string()))?;

    info!("Saving new OAuth token");
    storage.save_token(&StoredToken::from_token_response(&token))?;

    Ok(TokenManager::from_token_response(&token, storage))
}

/// The local address the OAuth redirect listener binds to, derived from the
/// configured redirect URL so the two can never disagree on the port.
fn redirect_listen_addr() -> Result<String, AuthError> {
    let url = Url::parse(constants::REDIRECT_URL.as_str())?;
    let port = url.port_or_known_default().unwrap_or(32857);
    Ok(format!("127.0.0.1:{}", port))
}

/// Accept connections until one carries the OAuth redirect, then validate the
/// CSRF state and extract the authorization code. Unrelated requests (e.g.
/// favicon fetches) get a 404 and the wait continues.
async fn wait_for_redirect(
    listener: &TcpListener,
    expected_state: &str,
) -> Result<AuthorizationCode, AuthError> {
    loop {
        let (mut stream, _) = listener.accept().await?;

        let mut request_line = String::new();
        {
            let mut reader = BufReader::new(&mut stream);
            reader.read_line(&mut request_line).await?;
        }

        let Some(path) = request_line.split_whitespace().nth(1) else {
            continue;
        };
        let Ok(url) = Url::parse(&format!("http://localhost{}", path)) else {
            continue;
        };

        let query_param = |key: &str| {
            url.query_pairs()
                .find(|(k, _)| k == key)
                .map(|(_, value)| value.into_owned())
        };

        if let Some(error) = query_param("error") {
            respond_html(
                &mut stream,
                "Sign-in cancelled",
                "You can close this tab and try again from Rustwave.",
            )
            .await;
            return Err(AuthError::OAuth(match error.as_str() {
                "access_denied" => "Access was denied in the browser".to_string(),
                other => format!("Authorization failed: {}", other),
            }));
        }

        let Some(code) = query_param("code") else {
            // Not the OAuth redirect (e.g. a favicon request) - keep waiting
            let _ = stream
                .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n")
                .await;
            continue;
        };

        if query_param("state").as_deref() != Some(expected_state) {
            respond_html(
                &mut stream,
                "Sign-in failed",
                "The request could not be verified. Please try again from Rustwave.",
            )
            .await;
            return Err(AuthError::OAuth(
                "CSRF state mismatch in OAuth redirect".to_string(),
            ));
        }

        respond_html(
            &mut stream,
            "You're signed in",
            "You can close this tab and return to Rustwave.",
        )
        .await;
        return Ok(AuthorizationCode::new(code));
    }
}

/// Send a small self-contained HTML page to the browser and close the
/// connection. Best-effort: the auth flow already has its result by now.
async fn respond_html(stream: &mut TcpStream, heading: &str, message: &str) {
    let body = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Rustwave</title><style>\
         body{{display:flex;align-items:center;justify-content:center;height:100vh;margin:0;\
         background:#1e1e2e;color:#cdd6f4;font-family:-apple-system,system-ui,sans-serif}}\
         div{{text-align:center}}h1{{color:#cba6f7;margin-bottom:.3em}}p{{color:#a6adc8}}\
         </style></head><body><div><h1>{}</h1><p>{}</p></div></body></html>",
        heading, message
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

async fn refresh_access_token(refresh_token: &RefreshToken) -> Result<TokenResp, AuthError> {
    let client = BasicClient::new(ClientId::new(constants::CLIENT_ID.to_string()))
        .set_client_secret(ClientSecret::new(constants::CLIENT_SECRET.to_string()))
        .set_token_uri(TokenUrl::new(constants::SOUNDCLOUD_TOKEN_URL.to_string())?);

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let token_result = client
        .exchange_refresh_token(refresh_token)
        .add_extra_param("client_id", constants::CLIENT_ID.as_str())
        .add_extra_param("client_secret", constants::CLIENT_SECRET.as_str())
        .request_async(&http_client)
        .await;

    match token_result {
        Ok(token) => Ok(token),
        Err(e) => {
            let error_string = format!("{}", e);
            if error_string.contains("invalid_grant") {
                Err(AuthError::OAuth("invalid_grant".to_string()))
            } else if error_string.contains("invalid_client") {
                Err(AuthError::OAuth(
                    "Invalid client credentials. Check CLIENT_ID and CLIENT_SECRET.".to_string(),
                ))
            } else if error_string.contains("unauthorized_client") {
                Err(AuthError::OAuth(
                    "Client not authorized to refresh tokens.".to_string(),
                ))
            } else {
                Err(AuthError::OAuth(format!("OAuth refresh failed: {}", e)))
            }
        }
    }
}

#[derive(Debug)]
pub enum AuthError {
    Io(std::io::Error),
    Json(serde_json::Error),
    OAuth(String),
    Other(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Io(e) => write!(f, "IO error: {}", e),
            AuthError::Json(e) => write!(f, "JSON error: {}", e),
            AuthError::OAuth(e) => write!(f, "OAuth error: {}", e),
            AuthError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<std::io::Error> for AuthError {
    fn from(err: std::io::Error) -> Self {
        AuthError::Io(err)
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(err: serde_json::Error) -> Self {
        AuthError::Json(err)
    }
}

impl From<url::ParseError> for AuthError {
    fn from(err: url::ParseError) -> Self {
        AuthError::OAuth(format!("URL parse error: {}", err))
    }
}

impl From<ConfigurationError> for AuthError {
    fn from(err: ConfigurationError) -> Self {
        AuthError::OAuth(format!("OAuth configuration error: {}", err))
    }
}

impl From<AuthError> for Box<dyn std::error::Error + Send + 'static> {
    fn from(err: AuthError) -> Self {
        Box::new(err)
    }
}
