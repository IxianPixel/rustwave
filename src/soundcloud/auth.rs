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
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use url::Url;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use crate::config;
use crate::constants;

#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
    token_type: String,
    #[serde(default = "default_created_at")]
    created_at: u64, // When the refresh token was first created
}

fn default_created_at() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl StoredToken {
    fn from_token_response(
        token: StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expires_at = token.expires_in().map(|duration| now + duration.as_secs());

        Self {
            access_token: token.access_token().secret().to_string(),
            refresh_token: token.refresh_token().map(|rt| rt.secret().to_string()),
            expires_at,
            token_type: "Bearer".to_string(), // SoundCloud uses Bearer tokens
            created_at: now,
        }
    }

    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now >= expires_at
        } else {
            false
        }
    }

    fn to_access_token(&self) -> AccessToken {
        AccessToken::new(self.access_token.clone())
    }

    fn to_refresh_token(&self) -> Option<RefreshToken> {
        self.refresh_token
            .as_ref()
            .map(|rt| RefreshToken::new(rt.clone()))
    }

    fn get_refresh_token_age_days(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.created_at) / (24 * 60 * 60)
    }

    fn is_refresh_token_old(&self) -> bool {
        // SoundCloud refresh tokens may expire after some time (30-90 days typically)
        self.get_refresh_token_age_days() > 30
    }
}

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

    fn save_token(
        &self,
        token: StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>,
    ) -> Result<(), AuthError> {
        let stored_token = StoredToken::from_token_response(token);
        let json = serde_json::to_string_pretty(&stored_token)?;
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

pub struct TokenManager {
    storage: TokenStorage,
    current_token: Arc<Mutex<AccessToken>>,
    refresh_token: Option<RefreshToken>,
}

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            storage: TokenStorage::new().expect("Failed to create token storage"),
            current_token: Arc::clone(&self.current_token),
            refresh_token: self.refresh_token.clone(),
        }
    }
}

impl std::fmt::Debug for TokenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenManager")
            .field("has_refresh_token", &self.refresh_token.is_some())
            .finish()
    }
}

impl TokenManager {
    pub fn new(token: AccessToken, refresh_token: Option<RefreshToken>) -> Result<Self, AuthError> {
        let storage = TokenStorage::new()?;
        Ok(Self {
            storage,
            current_token: Arc::new(Mutex::new(token)),
            refresh_token,
        })
    }

    pub fn get_access_token(&self) -> AccessToken {
        self.current_token.lock().unwrap().clone()
    }

    /// Get a fresh access token, refreshing if needed
    pub async fn get_fresh_token(&mut self) -> Result<AccessToken, AuthError> {
        match self.refresh_if_needed().await {
            Ok(_) => Ok(self.get_access_token()),
            Err(e) => Err(e),
        }
    }

    pub async fn refresh_if_needed(&mut self) -> Result<(), AuthError> {
        // Load the refresh token from storage to ensure we have the latest one
        let refresh_token = if let Ok(Some(stored_token)) = self.storage.load_token() {
            stored_token.to_refresh_token()
        } else {
            self.refresh_token.clone()
        };

        // Check if we have a refresh token and if the current token is expired
        if let Some(refresh_token) = refresh_token {
            match refresh_access_token(refresh_token.clone()).await {
                Ok(new_token) => {
                    info!("Successfully refreshed OAuth token during runtime");
                    self.storage.save_token(new_token.clone())?;

                    // Update the current token
                    let mut current_token = self.current_token.lock().unwrap();
                    *current_token = new_token.access_token().clone();

                    // Update the refresh token if a new one was provided
                    if let Some(new_refresh_token) = new_token.refresh_token() {
                        self.refresh_token = Some(new_refresh_token.clone());
                    }
                }
                Err(e) => {
                    return Err(AuthError::OAuth(format!(
                        "Failed to refresh token during runtime: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    pub async fn handle_auth_error(&mut self) -> Result<(), AuthError> {
        // When an API call fails with auth error, try to refresh the token
        if let Some(refresh_token) = &self.refresh_token {
            if let Ok(new_token) = refresh_access_token(refresh_token.clone()).await {
                info!("Successfully refreshed OAuth token after auth error");
                self.storage.save_token(new_token.clone())?;

                // Update the current token
                let mut current_token = self.current_token.lock().unwrap();
                *current_token = new_token.access_token().clone();

                // Update the refresh token if a new one was provided
                if let Some(new_refresh_token) = new_token.refresh_token() {
                    self.refresh_token = Some(new_refresh_token.clone());
                }

                return Ok(());
            } else {
                return Err(AuthError::OAuth(
                    "Failed to refresh token after auth error".to_string(),
                ));
            }
        }

        // If refresh failed, we need to re-authenticate
        warn!("Token refresh failed, need to re-authenticate");
        Err(AuthError::OAuth(
            "Token refresh failed, need to re-authenticate".to_string(),
        ))
    }
}

pub async fn authenticate() -> Result<TokenManager, AuthError> {
    let storage = TokenStorage::new()?;

    // Try to load existing token
    if let Some(stored_token) = storage.load_token()? {
        if !stored_token.is_expired() {
            info!("Using cached OAuth token");
            let refresh_token = stored_token.to_refresh_token();
            return TokenManager::new(stored_token.to_access_token(), refresh_token);
        }

        info!("Cached OAuth token expired, attempting refresh");
        // Token is expired, try to refresh it
        if let Some(refresh_token) = stored_token.to_refresh_token() {
            if let Ok(new_token) = refresh_access_token(refresh_token.clone()).await {
                info!("Successfully refreshed OAuth token");
                storage.save_token(new_token.clone())?;
                let new_refresh_token = new_token.refresh_token().cloned();
                return TokenManager::new(new_token.access_token().clone(), new_refresh_token);
            } else {
                warn!("Failed to refresh OAuth token, will perform full authentication");
            }
        } else {
            warn!("No refresh token available, will perform full authentication");
        }
    } else {
        info!("No cached OAuth token found, performing full authentication");
    }

    // No valid token found, perform full authentication
    info!("Starting OAuth2 authentication flow");
    let client = BasicClient::new(ClientId::new(constants::CLIENT_ID.to_string()))
        .set_client_secret(ClientSecret::new(constants::CLIENT_SECRET.to_string()))
        .set_auth_uri(AuthUrl::new(constants::SOUNDCLOUD_AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(constants::SOUNDCLOUD_TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(constants::REDIRECT_URL.to_string())?);

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Browse to: {}", auth_url);

    let (code, _state) = {
        // A very naive implementation of the redirect server.
        let listener = TcpListener::bind("127.0.0.1:32857").unwrap();

        // The server will terminate itself after collecting the first code.
        let Some(mut stream) = listener.incoming().flatten().next() else {
            panic!("listener terminated without accepting a connection");
        };

        let mut reader = BufReader::new(&stream);

        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();

        let redirect_url = request_line.split_whitespace().nth(1).unwrap();
        let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
            .unwrap();

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, state)| CsrfToken::new(state.into_owned()))
            .unwrap();

        let message = "Go back to your terminal :)";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );
        stream.write_all(response.as_bytes()).unwrap();

        (code, state)
    };

    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let token_result = client
        .exchange_code(code)
        .add_extra_param("client_id", constants::CLIENT_ID.as_str())
        .add_extra_param("client_secret", constants::CLIENT_SECRET.as_str())
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await;

    let token = match token_result {
        Ok(token) => token,
        Err(e) => return Err(AuthError::OAuth(e.to_string())),
    };

    // Save the new token
    info!("Saving new OAuth token");
    storage.save_token(token.clone())?;

    let refresh_token = token.refresh_token().cloned();
    TokenManager::new(token.access_token().clone(), refresh_token)
}

async fn refresh_access_token(
    refresh_token: RefreshToken,
) -> Result<StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>, AuthError> {
    let client = BasicClient::new(ClientId::new(constants::CLIENT_ID.to_string()))
        .set_client_secret(ClientSecret::new(constants::CLIENT_SECRET.to_string()))
        .set_token_uri(TokenUrl::new(constants::SOUNDCLOUD_TOKEN_URL.to_string())?);

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let token_result = client
        .exchange_refresh_token(&refresh_token)
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

pub async fn clear_stored_token() -> Result<(), AuthError> {
    let storage = TokenStorage::new()?;
    storage.clear_token()
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
/*
impl std::error::Error for TokenError {}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Something went wrong with the token request")
    }
}
*/
