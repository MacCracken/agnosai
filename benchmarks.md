# Benchmarks

Latest: **2026-03-28** — v1.0.0

Full history in [`bench-history.csv`](bench-history.csv). Run with `make bench` or `./scripts/bench-history.sh`.

## Orchestration Core

| Benchmark | Latest |
|-----------|--------|
| `Orchestrator::new` | 715 ns |
| `rank_agents (10 agents)` | 870 ns |
| `task_json/serialize` | 652 ns |
| `task_json/deserialize` | 853 ns |

## Server Endpoints

| Benchmark | Latest |
|-----------|--------|
| `GET /health` | 1.28 us |
| `GET /ready` | 1.52 us |
| `GET /metrics` | 1.09 us |
| `POST /mcp (initialize)` | 4.38 us |

## Tools

| Benchmark | Latest |
|-----------|--------|
| `EchoTool::execute` | 94 ns |
| `ToolRegistry::get (50 tools)` | 64 ns |
| `ToolRegistry::has (50 tools, hit)` | 27 ns |
| `ToolRegistry::list (50 tools)` | 9.98 us |

## Learning / RL

| Benchmark | Latest |
|-----------|--------|
| `Ucb1::select (10 arms)` | 47 ns |
| `Ucb1::select (50 arms)` | 253 ns |

## Fleet

| Benchmark | Latest |
|-----------|--------|
| `Relay::send throughput` | 166 ns |

## Head-to-Head: AgnosAI v1.0.0 vs CrewAI

Benchmark date: 2026-03-28. LLM: Ollama llama3.2:1b (1.2B Q8_0), 3 rounds, 30s cooldown.

| Scenario | CrewAI | AgnosAI | Speedup |
|----------|--------|---------|---------|
| large-crew-6-agents | 303.5s | 3ms | **103,130x** |
| multi-agent-sequential | 33.9s | 2ms | **13,633x** |
| single-agent-single-task | 5.4s | 2ms median | CrewAI wins cold |
| multi-agent-parallel | failed | 3ms | AgnosAI only |
| dag-dependencies | failed | 3ms | AgnosAI only |

See [`docs/development/comparison-crewai.md`](docs/development/comparison-crewai.md) for detailed analysis.
