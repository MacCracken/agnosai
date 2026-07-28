//! Benchmarks for Unix socket IPC round-trip, throughput, and large payloads.

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;

use agnosai::orchestrator::ipc::{IpcClient, IpcServer};

// ── IPC round-trip: bind + connect + send + recv ────────────────────────

fn bench_ipc_roundtrip(c: &mut Criterion) {
    c.bench_function("IPC round-trip (single message)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let tmp = tempfile::tempdir().unwrap();
                let path = tmp.path().join("bench.sock");

                let server = IpcServer::bind(&path).await.unwrap();

                let client_handle = tokio::spawn({
                    let path = path.clone();
                    async move {
                        let mut client = IpcClient::connect(&path).await.unwrap();
                        client.send(&json!({"ping": 1})).await.unwrap();
                        let _ = client.recv().await.unwrap();
                    }
                });

                let mut conn = server.accept().await.unwrap();
                let msg = conn.recv().await.unwrap();
                conn.send(&msg).await.unwrap();

                client_handle.await.unwrap();
            });
    });
}

// ── IPC throughput: 100 messages on same connection ─────────────────────

fn bench_ipc_throughput_100(c: &mut Criterion) {
    c.bench_function("IPC throughput (100 msgs, same conn)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let tmp = tempfile::tempdir().unwrap();
                let path = tmp.path().join("bench.sock");

                let server = IpcServer::bind(&path).await.unwrap();

                let client_handle = tokio::spawn({
                    let path = path.clone();
                    async move {
                        let mut client = IpcClient::connect(&path).await.unwrap();
                        for i in 0..100 {
                            client.send(&json!({"seq": i})).await.unwrap();
                            let _ = client.recv().await.unwrap();
                        }
                    }
                });

                let mut conn = server.accept().await.unwrap();
                for _ in 0..100 {
                    let msg = conn.recv().await.unwrap();
                    conn.send(&msg).await.unwrap();
                }

                client_handle.await.unwrap();
            });
    });
}

// ── IPC large payload: 1 MiB JSON ──────────────────────────────────────

fn bench_ipc_large_payload(c: &mut Criterion) {
    // Pre-build the 1 MiB payload outside the benchmark loop.
    let big_string = "x".repeat(1024 * 1024);
    let payload = json!({"data": big_string});

    c.bench_function("IPC large payload (1 MiB JSON)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let payload = payload.clone();
                async move {
                    let tmp = tempfile::tempdir().unwrap();
                    let path = tmp.path().join("bench.sock");

                    let server = IpcServer::bind(&path).await.unwrap();

                    let client_handle = tokio::spawn({
                        let path = path.clone();
                        let payload = payload.clone();
                        async move {
                            let mut client = IpcClient::connect(&path).await.unwrap();
                            client.send(&payload).await.unwrap();
                            let _ = client.recv().await.unwrap();
                        }
                    });

                    let mut conn = server.accept().await.unwrap();
                    let msg = conn.recv().await.unwrap();
                    conn.send(&msg).await.unwrap();

                    client_handle.await.unwrap();
                }
            });
    });
}

criterion_group!(
    benches,
    bench_ipc_roundtrip,
    bench_ipc_throughput_100,
    bench_ipc_large_payload,
);
criterion_main!(benches);
