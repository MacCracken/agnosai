//! Simple shared-secret auth middleware.
//!
//! Full JWT validation (RS256, claims, expiry) is future work.
//! For now this checks `Authorization: Bearer <token>` against a configured secret.

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

/// JWT/auth configuration.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AuthConfig {
    pub enabled: bool,
    pub secret: String,
}

/// Middleware that checks for a valid Bearer token.
///
/// If auth is disabled, passes through unconditionally.
/// If enabled, expects `Authorization: Bearer <token>` where token matches
/// `config.secret`.
pub async fn auth_middleware(
    config: AuthConfig,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !config.enabled {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header["Bearer ".len()..];
            if token.is_empty() || token != config.secret {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        _ => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware;
    use axum::routing::get;
    use tower::ServiceExt;

    async fn handler() -> &'static str {
        "ok"
    }

    fn test_router(config: AuthConfig) -> Router {
        Router::new()
            .route("/test", get(handler))
            .layer(middleware::from_fn(
                move |req: Request<Body>, next: Next| {
                    let cfg = config.clone();
                    async move { auth_middleware(cfg, req, next).await }
                },
            ))
    }

    #[tokio::test]
    async fn passes_when_disabled() {
        let app = test_router(AuthConfig {
            enabled: false,
            secret: String::new(),
        });
        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_missing_token_when_enabled() {
        let app = test_router(AuthConfig {
            enabled: true,
            secret: "my-secret".into(),
        });
        let resp = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_wrong_token_when_enabled() {
        let app = test_router(AuthConfig {
            enabled: true,
            secret: "my-secret".into(),
        });
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", "Bearer wrong-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn passes_with_correct_token() {
        let app = test_router(AuthConfig {
            enabled: true,
            secret: "my-secret".into(),
        });
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", "Bearer my-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_empty_bearer_token() {
        let app = test_router(AuthConfig {
            enabled: true,
            secret: "my-secret".into(),
        });
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", "Bearer ")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
