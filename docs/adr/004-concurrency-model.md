# ADR-004: Concurrency Model

**Status:** Accepted
**Date:** 2026-03-18

## Context

Agnostic v1 (Python/CrewAI) is limited by the GIL — concurrent crew execution is serialized. AgnosAI needs real concurrency for multi-agent orchestration.

## Decision

Use patterns proven in Agnosticos (8997+ tests):

| Pattern | Where | Why |
|---------|-------|-----|
| `Arc<RwLock<OrchestratorState>>` | Orchestrator | Single lock for compound operations; readers dominate |
| `tokio::sync::broadcast` | Pub/sub | One-to-many event delivery with backpressure |
| `DashMap` | Tool registry, subscriptions | Lock-free concurrent reads; high read:write ratio |
| `tokio::Semaphore` | Rate limiting, parallel execution | Bounded concurrency without busy-waiting |
| `tokio::task::JoinSet` | Parallel/DAG crew execution | Concurrent task execution with result collection |
| Priority `VecDeque` per level | Task scheduler | O(1) enqueue/dequeue per priority tier |

## Rationale

- These patterns are battle-tested in Agnosticos with thousands of tests
- `RwLock` over `Mutex` because orchestrator state is read far more than written
- `DashMap` over `RwLock<HashMap>` for registries with concurrent lookups
- `broadcast` channels for pub/sub because subscribers need independent receivers

## Consequences

- All shared state must be `Send + Sync`
- Lock ordering must be documented to prevent deadlocks as the system grows
- `broadcast` channels have bounded capacity (256) — slow subscribers lose messages
