# Architecture Decision Records

Decisions about agnosai — what we chose, the context, and the consequences we accept. Use these when a future reader would reasonably ask *"why did we do it this way?"*

## Conventions

- **Filename**: `NNN-kebab-case-title.md`, zero-padded to **three** digits. Never renumber.
  (This line said "four" until 2026-08-03, when every ADR on disk and CLAUDE.md's
  own ADR-format section already used three. The files are correct; the rule was
  not, and "never renumber" made fixing the rule the only option.)
- **One decision per ADR.** If a decision supersedes a prior one, add a new ADR and set the old one's status to `Superseded by NNNN`.
- **Status lifecycle**: `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Use [`template.md`](template.md) as the starting point.

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/adr/` | *Why did we choose X over Y?* |
| Architecture note | `docs/architecture/` | *What non-obvious constraint is true about the code?* |
| Guide | `docs/guides/` | *How do I do X?* |

## Index

Regenerate rather than hand-edit — this index read _"No ADRs yet"_ until
2026-08-03, by which point twelve existed.

| # | Decision | Status |
|---|---|---|
| [001](001-separate-repo.md) | AgnosAI as a separate repository | Accepted |
| [002](002-ecosystem-tools.md) | AGNOS ecosystem as optional tool backends | Accepted |
| [003](003-llm-native-http.md) | Native HTTP for LLM providers — the hoosh seam | Accepted |
| [004](004-concurrency-model.md) | Concurrency model — `sandhi_server_run_pooled` | Accepted |
| [005](005-ai-hwaccel-integration.md) | ai-hwaccel for hardware detection and workload planning | Accepted |
| [006](006-cx-tool-sandbox.md) | cx bytecode + kavach for sandboxed tool execution | Accepted in part — see below |
| [007](007-audit-redirect-revalidation.md) | Re-validate the SSRF guard on every redirect hop | Accepted |
| [008](008-durable-state-crew-id-validation.md) | `durable_state` validates `crew_id` | Accepted |
| [009](009-auth-constant-time-secret-compare.md) | Constant-time shared-secret comparison via SHA-256 digests | Accepted |
| [010](010-jwt-require-configured-iss-aud.md) | A configured `iss` or `aud` is required, not merely matched | Accepted |
| [011](011-metrics-endpoint-serves-agnosai-metrics.md) | `/metrics` serves agnosai's own registry, not hoosh's | Accepted |
| [012](012-no-graceful-shutdown-on-sandhi.md) | The server installs no signal handler and does not drain on shutdown | **Superseded by 013** |
| [013](013-graceful-shutdown-via-signalfd-and-stop-flag.md) | Graceful shutdown via `signalfd` + sandhi's stop flag | Accepted |
| [014](014-sse-stream-holds-a-pooled-worker.md) | An SSE stream holds a pooled worker for its whole life | Accepted |
| [015](015-mcp-resources-project-agent-definitions.md) | MCP resources project agent definitions, and nothing else | Accepted |
| [016](016-mcp-prompts-project-agent-personas.md) | MCP prompts project agent personas | Accepted |
| [017](017-genai-span-call-sites.md) | GenAI spans are recorded at the call sites the oracle only declares | Accepted |
| [018](018-sankoch-path-check-on-import.md) | `.agpkg` import trusts sankoch's path check | Accepted |
| [019](019-wasm-tools-spawn-wasmtime-directly.md) | WASM tools spawn wasmtime through kavach | Accepted — validated 2026-08-11 |

ADRs 001-005 predate the current heading convention (`ADR-00N: Title` rather
than `00N — Title`). The headings are left as written — renumbering accepted
decisions buys nothing and breaks inbound links — but **all five now carry a
`## Status` heading**, added by the 2026-08-11 review. (This paragraph used to
say they carried none, which stopped being true the moment that review ran.)

⚠ **Two rows above are deliberately not a simple "Accepted".** ADR 006's
"replacing WASM" half is retired — kavach ships a wasmtime backend and
[019](019-wasm-tools-spawn-wasmtime-directly.md) supersedes its transport half,
so cx and WASM are complementary rather than alternatives. ADR 012 is superseded
outright by 013 and is kept only as the record of a decision that turned out to
rest on a sandhi limitation which was then fixed upstream.
