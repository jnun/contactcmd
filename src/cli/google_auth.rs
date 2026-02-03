//! Google OAuth2 authentication for Gmail SMTP

use anyhow::{anyhow, Result};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
    basic::BasicClient,
    reqwest::http_client,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::Database;

// Google OAuth2 endpoints
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// Gmail send scope - minimal permissions
const GMAIL_SEND_SCOPE: &str = "https://www.googleapis.com/auth/gmail.send";

// Build-time embedded credentials (set via environment variables during compilation)
// These are read from .env at build time and embedded in the binary
const BUILD_CLIENT_ID: Option<&str> = option_env!("GOOGLE_CLIENT_ID");
const BUILD_CLIENT_SECRET: Option<&str> = option_env!("GOOGLE_CLIENT_SECRET");

/// Provider name for database storage
pub const PROVIDER_GOOGLE: &str = "google";

/// Load Google OAuth credentials
/// Priority: 1) Database settings, 2) Runtime env vars, 3) Build-time embedded
fn get_google_credentials(db: &Database) -> Result<(String, String)> {
    let client_id = db.get_setting("google_client_id")?
        .or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok())
        .or_else(|| BUILD_CLIENT_ID.map(String::from))
        .ok_or_else(|| anyhow!("Google client ID not configured. Set GOOGLE_CLIENT_ID env var or use 'contact config set google_client_id <value>'"))?;

    let client_secret = db.get_setting("google_client_secret")?
        .or_else(|| std::env::var("GOOGLE_CLIENT_SECRET").ok())
        .or_else(|| BUILD_CLIENT_SECRET.map(String::from))
        .ok_or_else(|| anyhow!("Google client secret not configured. Set GOOGLE_CLIENT_SECRET env var or use 'contact config set google_client_secret <value>'"))?;

    Ok((client_id, client_secret))
}

/// Check if Google OAuth is configured (has refresh token)
pub fn is_google_auth_configured(db: &Database) -> bool {
    db.get_oauth_token(PROVIDER_GOOGLE)
        .ok()
        .flatten()
        .is_some()
}

/// Get the authenticated Google email address
pub fn get_google_email(db: &Database) -> Option<String> {
    db.get_oauth_token(PROVIDER_GOOGLE)
        .ok()
        .flatten()
        .map(|t| t.email)
}

/// Run the Google OAuth2 authorization flow
/// Opens browser for user consent and captures the callback
pub fn run_google_auth_flow(db: &Database) -> Result<String> {
    let (client_id, client_secret) = get_google_credentials(db)?;

    // Find an available port for the callback server
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{}", port);

    // Create OAuth2 client
    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(AUTH_URL.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_uri.clone())?);

    // Generate authorization URL
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(GMAIL_SEND_SCOPE.to_string()))
        .add_scope(Scope::new("email".to_string())) // To get user's email
        .url();

    println!("\nOpening browser for Google sign-in...\n");

    // Open browser
    if webbrowser::open(auth_url.as_str()).is_err() {
        println!("Could not open browser automatically.");
        println!("Please open this URL manually:\n");
        println!("{}\n", auth_url);
    }

    println!("Waiting for authorization...");

    // Wait for the callback
    let (code, received_state) = wait_for_callback(listener)?;

    // Verify CSRF token
    if received_state != *csrf_token.secret() {
        return Err(anyhow!("CSRF token mismatch - possible security issue"));
    }

    println!("Authorization received, exchanging for tokens...");

    // Exchange code for tokens
    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .request(http_client)
        .map_err(|e| anyhow!("Token exchange failed: {}", e))?;

    let access_token = token_result.access_token().secret().to_string();
    let refresh_token = token_result
        .refresh_token()
        .ok_or_else(|| anyhow!("No refresh token received - try revoking access and re-authorizing"))?
        .secret()
        .to_string();

    // Calculate expiry time
    let expires_at = token_result.expires_in().map(|d| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + d.as_secs() as i64
    });

    // Get user's email address from the token info
    let email = get_email_from_token(&access_token)?;

    // Save tokens to database
    db.save_oauth_token(
        PROVIDER_GOOGLE,
        &email,
        &refresh_token,
        Some(&access_token),
        expires_at,
    )?;

    Ok(email)
}

/// Wait for OAuth callback on local server
fn wait_for_callback(listener: TcpListener) -> Result<(String, String)> {
    // Set a timeout (60 seconds)
    listener.set_nonblocking(false)?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut reader = BufReader::new(&stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line)?;

                // Parse the request to get the code and state
                let redirect_url = request_line
                    .split_whitespace()
                    .nth(1)
                    .ok_or_else(|| anyhow!("Invalid callback request"))?;

                // Extract query parameters
                let url = url::Url::parse(&format!("http://localhost{}", redirect_url))?;
                let params: std::collections::HashMap<_, _> = url.query_pairs().collect();

                let code = params
                    .get("code")
                    .ok_or_else(|| anyhow!("No authorization code received"))?
                    .to_string();

                let state = params
                    .get("state")
                    .ok_or_else(|| anyhow!("No state parameter received"))?
                    .to_string();

                // Send success response to browser
                let response = "HTTP/1.1 200 OK\r\n\
                    Content-Type: text/html\r\n\
                    Connection: close\r\n\r\n\
                    <html><body style=\"font-family: system-ui; text-align: center; padding: 50px;\">\
                    <h1>Authentication Successful</h1>\
                    <p>You can close this window and return to contactcmd.</p>\
                    </body></html>";

                stream.write_all(response.as_bytes())?;
                stream.flush()?;

                return Ok((code, state));
            }
            Err(e) => return Err(anyhow!("Callback server error: {}", e)),
        }
    }

    Err(anyhow!("No callback received"))
}

/// Get user's email address from access token using Google's userinfo endpoint
fn get_email_from_token(access_token: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to get user info: {}", response.status()));
    }

    let json: serde_json::Value = response.json()?;
    json["email"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Email not found in response"))
}

/// Refresh the access token if expired
pub fn refresh_access_token_if_needed(db: &Database) -> Result<String> {
    let token = db.get_oauth_token(PROVIDER_GOOGLE)?
        .ok_or_else(|| anyhow!("Google not authenticated"))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as i64;

    // If we have a valid access token that's not expired (with 60s buffer), use it
    if let (Some(access_token), Some(expires_at)) = (&token.access_token, token.expires_at) {
        if now < expires_at - 60 {
            return Ok(access_token.clone());
        }
    }

    // Need to refresh
    let (client_id, client_secret) = get_google_credentials(db)?;

    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(AUTH_URL.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string())?),
    );

    let token_result = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(token.refresh_token))
        .request(http_client)
        .map_err(|e| anyhow!("Token refresh failed: {}", e))?;

    let new_access_token = token_result.access_token().secret().to_string();

    let new_expires_at = token_result.expires_in().map(|d| {
        now + d.as_secs() as i64
    }).unwrap_or(now + 3600); // Default to 1 hour

    // Update stored token
    db.update_oauth_access_token(PROVIDER_GOOGLE, &new_access_token, new_expires_at)?;

    Ok(new_access_token)
}

/// Disconnect Google account (revoke and delete tokens)
pub fn disconnect_google(db: &Database) -> Result<()> {
    // Try to revoke the token with Google (best effort)
    if let Ok(Some(token)) = db.get_oauth_token(PROVIDER_GOOGLE) {
        let _ = revoke_token(&token.refresh_token);
    }

    // Delete from database
    db.delete_oauth_token(PROVIDER_GOOGLE)?;
    Ok(())
}

/// Revoke a token with Google (best effort, don't fail if this fails)
fn revoke_token(token: &str) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let _ = client
        .post("https://oauth2.googleapis.com/revoke")
        .form(&[("token", token)])
        .send();
    Ok(())
}
