//! JWT authentication middleware with RS256 + shared-secret fallback.
//!
//! Supports two modes:
//! - **JWT (RS256)**: Validates signature, issuer, expiry, and custom claims.
//! - **Shared secret**: Legacy `Bearer <token>` exact-match (when no JWKS configured).

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};

/// Constant-time byte comparison to prevent timing attacks on secret comparison.
///
/// Both length and content are compared in constant time — no early return
/// on length mismatch that would leak secret length.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    // XOR the lengths to avoid timing leak on length mismatch.
    let mut result = (a.len() ^ b.len()) as u8;
    // Compare up to the shorter length, then pad comparison for the remainder.
    for i in 0..a.len().max(b.len()) {
        let x = if i < a.len() { a[i] } else { 0 };
        let y = if i < b.len() { b[i] } else { 0 };
        result |= x ^ y;
    }
    result == 0
}

/// JWT/auth configuration.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AuthConfig {
    /// Whether auth is enabled at all.
    pub enabled: bool,
    /// Shared secret for legacy Bearer token validation (used when `jwt` is `None`).
    pub secret: String,
    /// JWT configuration. When set, tokens are validated as RS256 JWTs.
    pub jwt: Option<JwtConfig>,
}

impl AuthConfig {
    /// Create an auth config with a shared secret.
    pub fn with_secret(secret: impl Into<String>) -> Self {
        Self {
            enabled: true,
            secret: secret.into(),
            jwt: None,
        }
    }

    /// Create an auth config with JWT validation.
    pub fn with_jwt(jwt: JwtConfig) -> Self {
        Self {
            enabled: true,
            secret: String::new(),
            jwt: Some(jwt),
        }
    }
}

/// RS256 JWT validation configuration.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JwtConfig {
    /// PEM-encoded RSA public key for RS256 signature verification.
    pub public_key_pem: String,
    /// Expected `iss` claim. If set, tokens without a matching issuer are rejected.
    pub issuer: Option<String>,
    /// Expected `aud` claim. If set, tokens without a matching audience are rejected.
    pub audience: Option<String>,
}

impl JwtConfig {
    /// Create a JWT config with just the public key.
    pub fn new(public_key_pem: impl Into<String>) -> Self {
        Self {
            public_key_pem: public_key_pem.into(),
            issuer: None,
            audience: None,
        }
    }

    /// Set the expected issuer.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set the expected audience.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }
}

/// Standard JWT claims we validate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (typically a user or service ID).
    pub sub: Option<String>,
    /// Issuer.
    pub iss: Option<String>,
    /// Audience.
    pub aud: Option<String>,
    /// Expiration (UNIX timestamp).
    pub exp: Option<u64>,
    /// Issued-at (UNIX timestamp).
    pub iat: Option<u64>,
    /// Custom scope or role.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Validate a JWT string against the given config. Returns parsed claims on success.
fn validate_jwt(token: &str, config: &JwtConfig) -> Result<TokenData<Claims>, StatusCode> {
    let key = DecodingKey::from_rsa_pem(config.public_key_pem.as_bytes()).map_err(|e| {
        tracing::error!(error = %e, "JWT public key PEM is invalid — check AGNOSAI_JWT_PUBLIC_KEY");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;

    if let Some(ref iss) = config.issuer {
        validation.set_issuer(&[iss]);
    }

    if let Some(ref aud) = config.audience {
        validation.set_audience(&[aud]);
    } else {
        validation.validate_aud = false;
    }

    let token_data = jsonwebtoken::decode::<Claims>(token, &key, &validation).map_err(|e| {
        tracing::warn!(error = %e, "JWT validation failed");
        StatusCode::UNAUTHORIZED
    })?;

    // Defense-in-depth: reject tokens without an explicit expiration claim.
    if token_data.claims.exp.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(token_data)
}

/// Middleware that checks for a valid Bearer token.
///
/// If auth is disabled, passes through unconditionally.
/// If JWT config is present, validates as an RS256 JWT with claims/expiry.
/// Otherwise falls back to shared-secret exact match.
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

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let t = &header["Bearer ".len()..];
            if t.is_empty() {
                tracing::warn!("auth rejected: empty bearer token");
                return Err(StatusCode::UNAUTHORIZED);
            }
            t
        }
        _ => {
            tracing::warn!("auth rejected: missing or malformed authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    if let Some(ref jwt_config) = config.jwt {
        // RS256 JWT validation path.
        validate_jwt(token, jwt_config)?;
    } else {
        // Shared-secret fallback with constant-time comparison.
        if !constant_time_eq(token.as_bytes(), config.secret.as_bytes()) {
            tracing::warn!("auth rejected: invalid shared secret");
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

    // ── Shared-secret tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn passes_when_disabled() {
        let app = test_router(AuthConfig {
            enabled: false,
            secret: String::new(),
            jwt: None,
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
            jwt: None,
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
            jwt: None,
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
            jwt: None,
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
            jwt: None,
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

    // ── JWT tests ────────────────────────────────────────────────────────

    // Test RSA key pair generated for tests only.
    const TEST_RSA_PRIVATE_PEM: &str = include_str!("../../tests/fixtures/test_rsa_private.pem");
    const TEST_RSA_PUBLIC_PEM: &str = include_str!("../../tests/fixtures/test_rsa_public.pem");

    fn make_jwt(claims: &Claims) -> String {
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_PEM.as_bytes())
            .expect("valid test key");
        let header = jsonwebtoken::Header::new(Algorithm::RS256);
        jsonwebtoken::encode(&header, claims, &key).expect("encode JWT")
    }

    fn jwt_config() -> JwtConfig {
        JwtConfig {
            public_key_pem: TEST_RSA_PUBLIC_PEM.to_string(),
            issuer: Some("test-issuer".to_string()),
            audience: Some("agnosai".to_string()),
        }
    }

    fn valid_claims() -> Claims {
        Claims {
            sub: Some("user-123".to_string()),
            iss: Some("test-issuer".to_string()),
            aud: Some("agnosai".to_string()),
            exp: Some(u64::MAX), // far future
            iat: Some(0),
            scope: None,
        }
    }

    #[tokio::test]
    async fn jwt_valid_token_passes() {
        let config = AuthConfig {
            enabled: true,
            secret: String::new(),
            jwt: Some(jwt_config()),
        };
        let token = make_jwt(&valid_claims());
        let app = test_router(config);
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn jwt_expired_token_rejected() {
        let config = AuthConfig {
            enabled: true,
            secret: String::new(),
            jwt: Some(jwt_config()),
        };
        let mut claims = valid_claims();
        claims.exp = Some(0); // expired
        let token = make_jwt(&claims);
        let app = test_router(config);
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn jwt_wrong_issuer_rejected() {
        let config = AuthConfig {
            enabled: true,
            secret: String::new(),
            jwt: Some(jwt_config()),
        };
        let mut claims = valid_claims();
        claims.iss = Some("wrong-issuer".to_string());
        let token = make_jwt(&claims);
        let app = test_router(config);
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn jwt_wrong_audience_rejected() {
        let config = AuthConfig {
            enabled: true,
            secret: String::new(),
            jwt: Some(jwt_config()),
        };
        let mut claims = valid_claims();
        claims.aud = Some("wrong-audience".to_string());
        let token = make_jwt(&claims);
        let app = test_router(config);
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn jwt_garbage_token_rejected() {
        let config = AuthConfig {
            enabled: true,
            secret: String::new(),
            jwt: Some(jwt_config()),
        };
        let app = test_router(config);
        let resp = app
            .oneshot(
                Request::get("/test")
                    .header("Authorization", "Bearer not.a.jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
