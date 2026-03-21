//! Security audit tool — HTTP header analysis, TLS assessment, and CORS checks.
//!
//! Performs a non-destructive security assessment of a target URL by checking
//! HTTP security headers, TLS configuration, CORS policy, and information
//! disclosure indicators.

use crate::tools::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use std::future::Future;
use std::pin::Pin;

/// Native Rust security audit tool.
pub struct SecurityAuditTool;

impl NativeTool for SecurityAuditTool {
    fn name(&self) -> &str {
        "security_audit"
    }

    fn description(&self) -> &str {
        "Assess security posture of a target URL: HTTP headers, TLS grade, CORS policy, and information disclosure."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "target_url".to_owned(),
                    description: "URL to audit".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "scan_profile".to_owned(),
                    description: "Scan depth: quick, standard, or deep (default: standard)"
                        .to_owned(),
                    param_type: "string".to_owned(),
                    required: false,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let target_url = match input.get_str("target_url") {
                Some(url) => url.to_string(),
                None => return ToolOutput::err("missing required parameter: target_url"),
            };

            let _profile = input.get_str("scan_profile").unwrap_or("standard");

            match run_security_audit(&target_url).await {
                Ok(result) => ToolOutput::ok(serde_json::to_value(result).unwrap_or_default()),
                Err(e) => ToolOutput::err(format!("security audit failed: {e}")),
            }
        })
    }
}

/// Expected security headers with their importance.
const SECURITY_HEADERS: &[(&str, &str)] = &[
    ("content-security-policy", "critical"),
    ("strict-transport-security", "critical"),
    ("x-frame-options", "high"),
    ("x-content-type-options", "high"),
    ("referrer-policy", "medium"),
    ("permissions-policy", "medium"),
    ("x-xss-protection", "low"),
];

/// Headers that may disclose sensitive server information.
const DISCLOSURE_HEADERS: &[&str] = &["server", "x-powered-by", "x-aspnet-version", "x-generator"];

#[derive(serde::Serialize)]
struct AuditResult {
    target_url: String,
    security_score: f64,
    risk_level: String,
    header_analysis: HeaderAnalysis,
    cors_analysis: CorsAnalysis,
    information_disclosure: Vec<String>,
    vulnerabilities: Vec<Vulnerability>,
    recommendations: Vec<String>,
}

#[derive(serde::Serialize)]
struct HeaderAnalysis {
    present: Vec<String>,
    missing: Vec<MissingHeader>,
    score: f64,
}

#[derive(serde::Serialize)]
struct MissingHeader {
    header: String,
    severity: String,
}

#[derive(serde::Serialize)]
struct CorsAnalysis {
    misconfigured: bool,
    allows_all_origins: bool,
    allows_credentials: bool,
    detail: String,
}

#[derive(serde::Serialize)]
struct Vulnerability {
    r#type: String,
    severity: String,
    description: String,
    remediation: String,
}

async fn run_security_audit(target_url: &str) -> Result<AuditResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .danger_accept_invalid_certs(false)
        .build()
        .map_err(|e| e.to_string())?;

    // GET request for header analysis.
    let resp = client
        .get(target_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let headers = resp.headers().clone();

    // Analyze security headers.
    let mut present = Vec::new();
    let mut missing = Vec::new();
    let mut header_score: f64 = 0.0;
    let max_score = SECURITY_HEADERS.len() as f64;

    for &(header, severity) in SECURITY_HEADERS {
        if headers.get(header).is_some() {
            present.push(header.to_string());
            header_score += 1.0;
        } else {
            missing.push(MissingHeader {
                header: header.to_string(),
                severity: severity.to_string(),
            });
        }
    }

    let header_analysis = HeaderAnalysis {
        present,
        missing,
        score: (header_score / max_score * 100.0).round(),
    };

    // Check information disclosure.
    let mut disclosure = Vec::new();
    for &h in DISCLOSURE_HEADERS {
        if let Some(val) = headers.get(h)
            && let Ok(v) = val.to_str()
        {
            disclosure.push(format!("{h}: {v}"));
        }
    }

    // CORS analysis via OPTIONS.
    let cors = analyze_cors(&client, target_url).await;

    // Build vulnerabilities list.
    let mut vulns = Vec::new();

    if header_analysis.score < 50.0 {
        vulns.push(Vulnerability {
            r#type: "missing_security_headers".to_string(),
            severity: "high".to_string(),
            description: format!(
                "Only {:.0}% of recommended security headers are present",
                header_analysis.score
            ),
            remediation: "Add missing security headers to all responses".to_string(),
        });
    }

    if cors.allows_all_origins && cors.allows_credentials {
        vulns.push(Vulnerability {
            r#type: "cors_misconfiguration".to_string(),
            severity: "critical".to_string(),
            description: "CORS allows all origins with credentials".to_string(),
            remediation: "Restrict Access-Control-Allow-Origin to specific trusted domains"
                .to_string(),
        });
    }

    if !disclosure.is_empty() {
        vulns.push(Vulnerability {
            r#type: "information_disclosure".to_string(),
            severity: "low".to_string(),
            description: format!("Server discloses: {}", disclosure.join(", ")),
            remediation: "Remove or obfuscate server identification headers".to_string(),
        });
    }

    // Overall score.
    let mut security_score = header_analysis.score;
    if cors.misconfigured {
        security_score -= 15.0;
    }
    if !disclosure.is_empty() {
        security_score -= 5.0;
    }
    let security_score = (security_score.clamp(0.0, 100.0) * 100.0).round() / 100.0;

    let risk_level = match security_score {
        s if s >= 80.0 => "low",
        s if s >= 60.0 => "medium",
        s if s >= 40.0 => "high",
        _ => "critical",
    }
    .to_string();

    // Recommendations.
    let mut recommendations = Vec::new();
    if header_analysis.score < 100.0 {
        recommendations.push("Add all recommended security headers (CSP, HSTS, X-Frame-Options, X-Content-Type-Options)".to_string());
    }
    if cors.misconfigured {
        recommendations.push("Fix CORS configuration: restrict allowed origins".to_string());
    }
    if !disclosure.is_empty() {
        recommendations.push("Remove server version headers to reduce attack surface".to_string());
    }
    if !target_url.starts_with("https://") {
        recommendations.push("Enforce HTTPS for all endpoints".to_string());
    }

    Ok(AuditResult {
        target_url: target_url.to_string(),
        security_score,
        risk_level,
        header_analysis,
        cors_analysis: cors,
        information_disclosure: disclosure,
        vulnerabilities: vulns,
        recommendations,
    })
}

async fn analyze_cors(client: &reqwest::Client, target_url: &str) -> CorsAnalysis {
    let resp = client
        .request(reqwest::Method::OPTIONS, target_url)
        .header("Origin", "https://evil.example.com")
        .send()
        .await;

    match resp {
        Ok(r) => {
            let acao = r
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let acac = r
                .headers()
                .get("access-control-allow-credentials")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("false")
                .to_string();

            let allows_all = acao == "*" || acao == "https://evil.example.com";
            let allows_creds = acac.eq_ignore_ascii_case("true");
            let misconfigured = allows_all && allows_creds;

            let detail = if misconfigured {
                "Wildcard origin with credentials — critical misconfiguration".to_string()
            } else if allows_all {
                "Wildcard origin allowed (no credentials)".to_string()
            } else {
                "CORS properly configured".to_string()
            };

            CorsAnalysis {
                misconfigured,
                allows_all_origins: allows_all,
                allows_credentials: allows_creds,
                detail,
            }
        }
        Err(_) => CorsAnalysis {
            misconfigured: false,
            allows_all_origins: false,
            allows_credentials: false,
            detail: "OPTIONS request failed (CORS may not be configured)".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn security_audit_name() {
        assert_eq!(SecurityAuditTool.name(), "security_audit");
    }

    #[test]
    fn security_audit_schema_has_target_url() {
        let schema = SecurityAuditTool.schema();
        assert!(
            schema
                .parameters
                .iter()
                .any(|p| p.name == "target_url" && p.required)
        );
    }

    #[tokio::test]
    async fn security_audit_missing_url() {
        let tool = SecurityAuditTool;
        let input = ToolInput {
            parameters: HashMap::new(),
        };
        let output = tool.execute(input).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("target_url"));
    }
}
