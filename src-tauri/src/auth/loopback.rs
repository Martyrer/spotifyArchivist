use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::error::{AuthError, Result};

pub const FIXED_LOOPBACK_PORT: u16 = 4202;
pub const FIXED_REDIRECT_URI: &str = "http://127.0.0.1:4202/callback";

pub struct LoopbackListener {
    pub addr: SocketAddr,
    pub redirect_uri: String,
    receiver: oneshot::Receiver<Outcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Code {
        code: String,
        state: String,
    },
    Error {
        error: String,
        state: Option<String>,
    },
}

const SUCCESS_HTML: &str = "<!doctype html><html><body style=\"font-family:sans-serif;background:#0a0a0a;color:#f5f5f5;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;\"><div><h1>Login complete</h1><p>You can close this tab and return to Spotify Archivist.</p></div></body></html>";

const FAILURE_HTML: &str = "<!doctype html><html><body style=\"font-family:sans-serif;background:#0a0a0a;color:#f5f5f5;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;\"><div><h1>Login failed</h1><p>Return to Spotify Archivist for details.</p></div></body></html>";

impl LoopbackListener {
    pub async fn bind() -> Result<Self> {
        Self::bind_on(FIXED_LOOPBACK_PORT, FIXED_REDIRECT_URI.to_string()).await
    }

    pub async fn bind_on(port: u16, redirect_uri: String) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| AuthError::Keyring(format!("bind: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| AuthError::Keyring(format!("addr: {e}")))?;
        let redirect_uri = if port == 0 {
            format!("http://127.0.0.1:{}/callback", addr.port())
        } else {
            redirect_uri
        };

        let (tx, rx) = oneshot::channel::<Outcome>();
        let tx_holder = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let io = TokioIo::new(stream);
                let tx = tx_holder.clone();
                tokio::spawn(async move {
                    let svc = service_fn(move |req: Request<hyper::body::Incoming>| {
                        let tx = tx.clone();
                        async move { handle(req, tx).await }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .await;
                });
                if tx_holder.lock().await.is_none() {
                    break;
                }
            }
        });

        Ok(Self {
            addr,
            redirect_uri,
            receiver: rx,
        })
    }

    pub async fn wait(self, timeout: Duration) -> Result<Outcome> {
        match tokio::time::timeout(timeout, self.receiver).await {
            Ok(Ok(o)) => Ok(o),
            Ok(Err(_)) => Err(AuthError::Cancelled),
            Err(_) => Err(AuthError::Cancelled),
        }
    }
}

async fn handle(
    req: Request<hyper::body::Incoming>,
    tx: std::sync::Arc<tokio::sync::Mutex<Option<oneshot::Sender<Outcome>>>>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    if !req.uri().path().starts_with("/callback") {
        return Ok(Response::builder()
            .status(404)
            .body(Full::new(Bytes::from("not found")))
            .unwrap());
    }
    let q = req.uri().query().unwrap_or_default();
    let params: HashMap<String, String> = url::form_urlencoded::parse(q.as_bytes())
        .into_owned()
        .collect();

    let outcome = if let Some(err) = params.get("error") {
        Outcome::Error {
            error: err.clone(),
            state: params.get("state").cloned(),
        }
    } else if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
        Outcome::Code {
            code: code.clone(),
            state: state.clone(),
        }
    } else {
        Outcome::Error {
            error: "missing_params".into(),
            state: None,
        }
    };

    let html = if matches!(outcome, Outcome::Code { .. }) {
        SUCCESS_HTML
    } else {
        FAILURE_HTML
    };

    if let Some(sender) = tx.lock().await.take() {
        let _ = sender.send(outcome);
    }

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn captures_code_and_state() {
        let listener = LoopbackListener::bind_on(0, String::new()).await.unwrap();
        let url = format!("{}?code=ABC&state=XYZ", listener.redirect_uri);
        tokio::spawn(async move {
            let _ = reqwest::get(url).await;
        });
        let outcome = listener.wait(Duration::from_secs(5)).await.unwrap();
        assert_eq!(
            outcome,
            Outcome::Code {
                code: "ABC".into(),
                state: "XYZ".into()
            }
        );
    }

    #[tokio::test]
    async fn captures_error_param() {
        let listener = LoopbackListener::bind_on(0, String::new()).await.unwrap();
        let url = format!("{}?error=access_denied&state=Y", listener.redirect_uri);
        tokio::spawn(async move {
            let _ = reqwest::get(url).await;
        });
        let outcome = listener.wait(Duration::from_secs(5)).await.unwrap();
        assert!(matches!(outcome, Outcome::Error { .. }));
    }

    #[tokio::test]
    async fn missing_params_yield_error_outcome() {
        let listener = LoopbackListener::bind_on(0, String::new()).await.unwrap();
        let url = listener.redirect_uri.clone();
        tokio::spawn(async move {
            let _ = reqwest::get(url).await;
        });
        let outcome = listener.wait(Duration::from_secs(5)).await.unwrap();
        match outcome {
            Outcome::Error { error, .. } => assert_eq!(error, "missing_params"),
            _ => panic!("expected error"),
        }
    }

    #[tokio::test]
    async fn timeout_returns_cancelled() {
        let listener = LoopbackListener::bind_on(0, String::new()).await.unwrap();
        let err = listener.wait(Duration::from_millis(50)).await.unwrap_err();
        assert!(matches!(err, AuthError::Cancelled));
    }
}
