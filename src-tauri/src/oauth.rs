use anyhow::{anyhow, Result};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::time::timeout;
use url::Url;

const CLIENT_ID: Option<&str> = option_env!("SHADOW_GDRIVE_CLIENT_ID");
const CLIENT_SECRET: Option<&str> = option_env!("SHADOW_GDRIVE_CLIENT_SECRET");
const REDIRECT_URI: &str = "http://127.0.0.1:40003";

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

fn base64_url_encode(data: &[u8]) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(data)
}

fn generate_random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Generates the PKCE code verifier and code challenge.
/// Returns (code_verifier, code_challenge)
pub fn generate_pkce() -> (String, String) {
    let verifier_bytes = generate_random_bytes(32);
    let verifier = base64_url_encode(&verifier_bytes);
    let challenge_bytes = sha256(verifier.as_bytes());
    let challenge = base64_url_encode(&challenge_bytes);
    (verifier, challenge)
}

/// Generates a secure random state parameter for CSRF mitigation.
pub fn generate_state() -> String {
    let state_bytes = generate_random_bytes(16);
    base64_url_encode(&state_bytes)
}

pub fn get_client_credentials() -> Result<(&'static str, &'static str)> {
    let client_id = CLIENT_ID.ok_or_else(|| {
        anyhow!("Google Drive Client ID was not set at build time. Check your .env file and rebuild.")
    })?;
    let client_secret = CLIENT_SECRET.ok_or_else(|| {
        anyhow!("Google Drive Client Secret was not set at build time. Check your .env file and rebuild.")
    })?;
    Ok((client_id, client_secret))
}

/// Starts the loopback server on 127.0.0.1:40003 and waits for Google's redirect.
/// Returns the authorization code if successful.
pub async fn start_loopback_listener(expected_state: &str) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:40003").await?;

    // Set a 5-minute timeout for the user to complete the login in their browser
    let result = timeout(Duration::from_secs(300), async {
        loop {
            let (mut socket, _) = listener.accept().await?;
            let mut reader = BufReader::new(&mut socket);
            let mut request_line = String::new();
            
            if reader.read_line(&mut request_line).await.is_err() {
                continue;
            }

            // Expecting: "GET /?code=...&state=... HTTP/1.1"
            let Some(uri) = request_line.split_whitespace().nth(1) else {
                continue;
            };

            let full_url = format!("http://127.0.0.1{}", uri);
            let Ok(parsed_url) = Url::parse(&full_url) else {
                continue;
            };

            let params: HashMap<String, String> = parsed_url.query_pairs().into_owned().collect();
            let code = params.get("code");
            let state = params.get("state");

            match (code, state) {
                (Some(c), Some(s)) if s == expected_state => {
                    // Send success response page
                    let response_body = "
                        <html>
                        <head><title>Shadow Authorization Successful</title></head>
                        <body style=\"font-family: sans-serif; text-align: center; margin-top: 10%; background-color: #0f172a; color: #f8fafc;\">
                            <h1 style=\"color: #38bdf8;\">Authorization Successful!</h1>
                            <p>Shadow has connected to Google Drive.</p>
                            <p>You can close this tab and return to the application.</p>
                        </body>
                        </html>
                    ";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );
                    socket.write_all(response.as_bytes()).await?;
                    socket.flush().await?;
                    return Ok(c.clone());
                }
                _ => {
                    // Send error response page
                    let response_body = "
                        <html>
                        <head><title>Shadow Authorization Failed</title></head>
                        <body style=\"font-family: sans-serif; text-align: center; margin-top: 10%; background-color: #0f172a; color: #f8fafc;\">
                            <h1 style=\"color: #ef4444;\">Authorization Failed</h1>
                            <p>The state parameter is invalid or missing, indicating a potential CSRF attack or aborted login.</p>
                            <p>Please try connecting again from the Shadow app.</p>
                        </body>
                        </html>
                    ";
                    let response = format!(
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );
                    socket.write_all(response.as_bytes()).await?;
                    socket.flush().await?;
                }
            }
        }
    }).await;

    match result {
        Ok(code_res) => code_res,
        Err(_) => Err(anyhow!("Authentication timed out after 5 minutes")),
    }
}

/// Exchanges the authorization code and verifier for access and refresh tokens.
pub async fn exchange_code_for_tokens(code: &str, verifier: &str) -> Result<TokenResponse> {
    let (client_id, client_secret) = get_client_credentials()?;
    let client = reqwest::Client::new();

    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", REDIRECT_URI),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let err_text = response.text().await?;
        return Err(anyhow!("Failed to exchange authorization code: {}", err_text));
    }

    let token_resp: TokenResponse = response.json().await?;
    Ok(token_resp)
}

/// Refreshes the access token using the refresh token.
pub async fn refresh_access_token(refresh_token: &str) -> Result<TokenResponse> {
    let (client_id, client_secret) = get_client_credentials()?;
    let client = reqwest::Client::new();

    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let err_text = response.text().await?;
        return Err(anyhow!("Failed to refresh access token: {}", err_text));
    }

    let token_resp: TokenResponse = response.json().await?;
    Ok(token_resp)
}
