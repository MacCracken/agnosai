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
| [017](017-genai-span-call-sites.md) | GenAI spans are recorded at the call sites the oracle only declares | Accepted — amended by 023 |
| [018](018-sankoch-path-check-on-import.md) | `.agpkg` import trusts sankoch's path check | Accepted |
| [019](019-wasm-tools-spawn-wasmtime-directly.md) | WASM tools spawn wasmtime through kavach | Accepted — validated 2026-08-11 |
| [020](020-output-filter-wired-into-task-output.md) | `output_filter` is wired into task output; the oracle never wires it | Accepted |
| [021](021-rate-limit-mounted-by-default.md) | `rate_limit` is mounted by default | Accepted |
| [022](022-crew-events-and-status-say-what-happened.md) | Crew events and registry status say what happened | Accepted — shipped in 2.1.5 |
| [023](023-genai-semconv-spans-and-w3c-trace-context.md) | GenAI spans follow the semconv agent operations, and join the caller's W3C trace (amends 017) | Accepted — shipped in 2.1.5 |
| [024](024-an-embedding-consumer-records-http-server-spans.md) | An embedding consumer can record an HTTP server span through agnosai's exporter | Accepted — 2.1.7 |
| [025](025-a-tool-call-passes-a-consumers-gate.md) | A tool call passes a consumer's gate, and leaves a record on its task | Accepted |
| [026](026-the-crew-registry-keeps-what-has-not-been-read.md) | The crew registry keeps what has not been read | Accepted |
| [027](027-the-built-in-presets-are-an-example-library.md) | The built-in presets are an example library | Accepted |
| [028](028-a-crew-lives-in-an-arena-its-consumer-gives-back.md) | A crew lives in an arena its consumer gives back | Accepted |
| [029](029-the-tool-loop.md) | The tool loop: the model proposes, and agnosai disposes | Accepted |
| [030](030-a-model-comes-from-the-gateways-catalog.md) | A model comes from the gateway's catalog, not from a name agnosai carries | Accepted |
| [031](031-confine-the-process-and-python-tiers-in-agnosais-spawner.md) | Confine the process and Python tiers in agnosai's spawner, with kavach's primitives | Accepted |
| [032](032-the-crew-lifecycle-is-one-state-machine.md) | The crew lifecycle is one state machine, numbered, with the approval gate in it | Accepted — shipped in 2.4.0; its batches are a sliding window since 2.4.1 (033) |
| [033](033-budgets-caps-and-crew-concurrency-are-enforced.md) | Budgets, caps and crew concurrency are enforced | Accepted — shipped in 2.4.1 |
| [034](034-a-dependent-task-is-handed-its-dependencies-answers.md) | A dependent task is handed its dependencies' answers, as data | Accepted — shipped in 2.4.2 |
| [035](035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md) | A manager assigns its workers and writes the crew's answer | Accepted — shipped in 2.4.3 |
| [036](036-a-crew-keeps-a-log-and-resumes-from-it.md) | A crew keeps a log, and resumes from it | Accepted — shipped in 2.5.0 |
| [037](037-the-servers-crews-are-the-orchestrators-to-give-back.md) | The server's crews are the orchestrator's to give back | Accepted — shipped in 2.5.1 |

ADRs 001-005 predate the current heading convention (`ADR-00N: Title` rather
than `00N — Title`). The headings are left as written — renumbering accepted
decisions buys nothing and breaks inbound links — but **all five now carry a
`## Status` heading**, added by the 2026-08-11 review. (This paragraph used to
say they carried none, which stopped being true the moment that review ran.)

⚠ **Six rows above are not a plain "Accepted".** Three of them only add a fact:
019 records when it was validated, and 022 and 023 the release that shipped them.
The other three need reading. ADR 017 stays Accepted for the decision it still
carries (the call sites); 023, accepted with 022 in 2.1.5, amends the rest of
it. ADR 006's "replacing WASM" half is retired — kavach ships a wasmtime backend and
[019](019-wasm-tools-spawn-wasmtime-directly.md) supersedes its transport half,
so cx and WASM are complementary rather than alternatives. ADR 012 is superseded
outright by 013 and is kept only as the record of a decision that turned out to
rest on a sandhi limitation which was then fixed upstream.
