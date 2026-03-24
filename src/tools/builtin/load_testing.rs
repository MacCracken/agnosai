//! Load testing tool — HTTP load generation with concurrent users.
//!
//! Sends concurrent HTTP requests to a target URL and measures latency,
//! throughput, error rate, and status code distribution. Leverages tokio
//! and reqwest for real concurrency with connection pooling.

use crate::tools::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// Native Rust load testing tool.
pub struct LoadTestingTool;

impl NativeTool for LoadTestingTool {
    fn name(&self) -> &str {
        "load_testing"
    }

    fn description(&self) -> &str {
        "Generate HTTP load against a target URL with concurrent users and measure performance metrics."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "target_url".to_owned(),
                    description: "URL to load test".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "concurrent_users".to_owned(),
                    description: "Number of concurrent users (default: 10)".to_owned(),
                    param_type: "number".to_owned(),
                    required: false,
                },
                ParameterSchema {
                    name: "duration_seconds".to_owned(),
                    description: "Test duration in seconds (default: 10)".to_owned(),
                    param_type: "number".to_owned(),
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

            // SSRF protection: reject requests to private/internal networks.
            if !crate::server::ssrf::is_safe_url(&target_url) {
                return ToolOutput::err(
                    "target_url rejected: cannot target private/internal networks",
                );
            }

            let concurrent_users = input.get_u64("concurrent_users").unwrap_or(10) as usize;
            let duration_secs = input.get_u64("duration_seconds").unwrap_or(10);

            // Cap to reasonable limits.
            let concurrent_users = concurrent_users.min(500);
            let duration = Duration::from_secs(duration_secs.clamp(1, 300));

            match run_load_test(&target_url, concurrent_users, duration).await {
                Ok(result) => ToolOutput::ok(serde_json::to_value(result).unwrap_or_default()),
                Err(e) => ToolOutput::err(format!("load test failed: {e}")),
            }
        })
    }
}

#[derive(serde::Serialize)]
struct LoadTestResult {
    target_url: String,
    concurrent_users: usize,
    duration_seconds: u64,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    avg_latency_ms: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
    throughput_rps: f64,
    error_rate: f64,
    status_codes: HashMap<u16, u64>,
}

async fn run_load_test(
    target_url: &str,
    concurrent_users: usize,
    duration: Duration,
) -> Result<LoadTestResult, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(concurrent_users)
        .build()
        .map_err(|e| e.to_string())?;

    let url = target_url.to_string();
    let deadline = Instant::now() + duration;

    // Each "user" is a spawned task that loops until the deadline.
    let mut handles = Vec::with_capacity(concurrent_users);

    for _ in 0..concurrent_users {
        let client = client.clone();
        let url = url.clone();

        handles.push(tokio::spawn(async move {
            let mut latencies = Vec::new();
            let mut status_counts: HashMap<u16, u64> = HashMap::new();
            let mut errors: u64 = 0;

            while Instant::now() < deadline {
                let start = Instant::now();
                match client.get(&url).send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        *status_counts.entry(status).or_default() += 1;
                        latencies.push(start.elapsed());
                    }
                    Err(_) => {
                        errors += 1;
                        latencies.push(start.elapsed());
                    }
                }
            }

            (latencies, status_counts, errors)
        }));
    }

    // Collect results from all users.
    let mut all_latencies: Vec<Duration> = Vec::new();
    let mut all_status_codes: HashMap<u16, u64> = HashMap::new();
    let mut total_errors: u64 = 0;

    for handle in handles {
        match handle.await {
            Ok((latencies, statuses, errors)) => {
                all_latencies.extend(latencies);
                for (code, count) in statuses {
                    *all_status_codes.entry(code).or_default() += count;
                }
                total_errors += errors;
            }
            Err(_) => total_errors += 1,
        }
    }

    let total_requests = all_latencies.len() as u64;
    let successful_requests = total_requests.saturating_sub(total_errors);

    if all_latencies.is_empty() {
        return Err("no requests completed".to_string());
    }

    // Sort for percentile calculation.
    all_latencies.sort();

    let to_ms = |d: Duration| d.as_secs_f64() * 1000.0;

    let avg_latency =
        all_latencies.iter().map(|d| to_ms(*d)).sum::<f64>() / all_latencies.len() as f64;
    let last = all_latencies.len() - 1;
    let p50 = to_ms(all_latencies[(all_latencies.len() * 50 / 100).min(last)]);
    let p95 = to_ms(all_latencies[(all_latencies.len() * 95 / 100).min(last)]);
    let p99 = to_ms(all_latencies[(all_latencies.len() * 99 / 100).min(last)]);
    let min_latency = to_ms(all_latencies[0]);
    let max_latency = to_ms(*all_latencies.last().unwrap());

    let elapsed = duration.as_secs_f64();
    let throughput = total_requests as f64 / elapsed;
    let error_rate = if total_requests > 0 {
        total_errors as f64 / total_requests as f64
    } else {
        0.0
    };

    Ok(LoadTestResult {
        target_url: url,
        concurrent_users,
        duration_seconds: duration.as_secs(),
        total_requests,
        successful_requests,
        failed_requests: total_errors,
        avg_latency_ms: (avg_latency * 100.0).round() / 100.0,
        p50_latency_ms: (p50 * 100.0).round() / 100.0,
        p95_latency_ms: (p95 * 100.0).round() / 100.0,
        p99_latency_ms: (p99 * 100.0).round() / 100.0,
        min_latency_ms: (min_latency * 100.0).round() / 100.0,
        max_latency_ms: (max_latency * 100.0).round() / 100.0,
        throughput_rps: (throughput * 100.0).round() / 100.0,
        error_rate: (error_rate * 10000.0).round() / 10000.0,
        status_codes: all_status_codes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_testing_name() {
        assert_eq!(LoadTestingTool.name(), "load_testing");
    }

    #[test]
    fn load_testing_schema_has_target_url() {
        let schema = LoadTestingTool.schema();
        assert!(
            schema
                .parameters
                .iter()
                .any(|p| p.name == "target_url" && p.required)
        );
    }

    #[tokio::test]
    async fn load_testing_missing_url() {
        let tool = LoadTestingTool;
        let input = ToolInput {
            parameters: HashMap::new(),
        };
        let output = tool.execute(input).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("target_url"));
    }
}
