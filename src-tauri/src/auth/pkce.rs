use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use super::error::{AuthError, Result};

const SPOTIFY_AUTH_URL: &str = "https://accounts.spotify.com/authorize";
pub const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";

pub const SCOPES: &[&str] = &[
    "user-library-read",
    "playlist-read-private",
    "playlist-read-collaborative",
];

#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

pub fn build_pkce() -> PkceChallenge {
    let mut buf = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut buf);
    let verifier = URL_SAFE_NO_PAD.encode(buf);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let mut state_buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut state_buf);
    let state = URL_SAFE_NO_PAD.encode(state_buf);

    PkceChallenge {
        verifier,
        challenge,
        state,
    }
}

pub fn authorize_url(
    client_id: &str,
    redirect_uri: &str,
    challenge: &PkceChallenge,
) -> Result<Url> {
    let mut url = Url::parse(SPOTIFY_AUTH_URL)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &SCOPES.join(" "))
        .append_pair("state", &challenge.state)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &challenge.challenge);
    Ok(url)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    token_type: String,
    scope: Option<String>,
}

pub async fn exchange_code(
    http: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    redirect_uri: &str,
    code: &str,
    verifier: &str,
) -> Result<TokenSet> {
    let res = http
        .post(token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(AuthError::TokenEndpoint {
            status: status.as_u16(),
            body,
        });
    }
    let parsed: TokenResponse = res.json().await?;
    Ok(TokenSet {
        access_token: parsed.access_token,
        refresh_token: parsed
            .refresh_token
            .ok_or(AuthError::MissingField("refresh_token"))?,
        expires_in: parsed.expires_in,
        token_type: parsed.token_type,
        scope: parsed.scope.unwrap_or_default(),
    })
}

pub async fn refresh_token(
    http: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    refresh: &str,
) -> Result<TokenSet> {
    let res = http
        .post(token_url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
            ("client_id", client_id),
        ])
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(AuthError::TokenEndpoint {
            status: status.as_u16(),
            body,
        });
    }
    let parsed: TokenResponse = res.json().await?;
    Ok(TokenSet {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token.unwrap_or_else(|| refresh.to_string()),
        expires_in: parsed.expires_in,
        token_type: parsed.token_type,
        scope: parsed.scope.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use sha2::{Digest, Sha256};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn build_pkce_produces_valid_s256_pair() {
        let p = build_pkce();
        assert!(!p.verifier.is_empty());
        assert!(!p.challenge.is_empty());
        assert!(!p.state.is_empty());

        let mut h = Sha256::new();
        h.update(p.verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(h.finalize());
        assert_eq!(p.challenge, expected);
    }

    #[test]
    fn build_pkce_produces_unique_values() {
        let a = build_pkce();
        let b = build_pkce();
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.state, b.state);
    }

    #[test]
    fn authorize_url_includes_pkce_and_scopes() {
        let p = build_pkce();
        let url = authorize_url("CID", "http://127.0.0.1:1234/cb", &p).unwrap();
        let s = url.as_str();
        assert!(s.contains("response_type=code"));
        assert!(s.contains("client_id=CID"));
        assert!(s.contains("code_challenge_method=S256"));
        assert!(s.contains(&format!("code_challenge={}", p.challenge)));
        assert!(s.contains(&format!("state={}", p.state)));
        assert!(s.contains("scope="));
        assert!(s.contains("user-library-read"));
    }

    #[tokio::test]
    async fn exchange_code_parses_token_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "AT",
                "refresh_token": "RT",
                "expires_in": 3600,
                "token_type": "Bearer",
                "scope": "user-library-read"
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let url = format!("{}/api/token", server.uri());
        let t = exchange_code(&http, &url, "CID", "http://127.0.0.1/cb", "CODE", "VER")
            .await
            .unwrap();
        assert_eq!(t.access_token, "AT");
        assert_eq!(t.refresh_token, "RT");
        assert_eq!(t.expires_in, 3600);
    }

    #[tokio::test]
    async fn exchange_code_returns_error_on_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad_request"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let url = format!("{}/api/token", server.uri());
        let err = exchange_code(&http, &url, "CID", "http://127.0.0.1/cb", "CODE", "VER")
            .await
            .unwrap_err();
        match err {
            AuthError::TokenEndpoint { status, .. } => assert_eq!(status, 400),
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[tokio::test]
    async fn exchange_code_missing_refresh_token_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "AT",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let url = format!("{}/api/token", server.uri());
        let err = exchange_code(&http, &url, "CID", "http://127.0.0.1/cb", "CODE", "VER")
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::MissingField("refresh_token")));
    }

    #[tokio::test]
    async fn refresh_token_keeps_old_refresh_when_omitted() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "NEW",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let url = format!("{}/api/token", server.uri());
        let t = refresh_token(&http, &url, "CID", "OLD_REFRESH")
            .await
            .unwrap();
        assert_eq!(t.access_token, "NEW");
        assert_eq!(t.refresh_token, "OLD_REFRESH");
    }

    #[tokio::test]
    async fn refresh_token_uses_new_refresh_when_present() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "NEW",
                "refresh_token": "NEW_REFRESH",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let url = format!("{}/api/token", server.uri());
        let t = refresh_token(&http, &url, "CID", "OLD_REFRESH")
            .await
            .unwrap();
        assert_eq!(t.refresh_token, "NEW_REFRESH");
    }

    #[tokio::test]
    async fn refresh_token_returns_error_on_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid_grant"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let url = format!("{}/api/token", server.uri());
        let err = refresh_token(&http, &url, "CID", "OLD_REFRESH")
            .await
            .unwrap_err();
        match err {
            AuthError::TokenEndpoint { status, .. } => assert_eq!(status, 401),
            other => panic!("unexpected: {other:?}"),
        }
    }
}
