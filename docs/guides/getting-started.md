# Getting started with agnosai

## Prerequisites

- The Cyrius toolchain at the version pinned in `cyrius.cyml` (`cyrius = "X.Y.Z"`) —
  install it with the upstream `scripts/install.sh`, exactly as `.github/workflows/ci.yml`
  and the `Dockerfile` do.
- Optional: [`wasmtime`](https://wasmtime.dev) on a path kavach probes
  (`/usr/local/bin`, `/usr/bin` or `~/.cargo/bin`) for WASM tools, and a running
  [hoosh](https://github.com/MacCracken/hoosh) gateway for real LLM inference. Without
  hoosh every task runs the placeholder path, which needs nothing installed.

## Build

```sh
cyrius lib sync --full                     # provision lib/ from the pinned stdlib snapshot
cyrius deps                                # overlay the [deps.*] bundles
cyrius build src/main.cyr build/agnosai    # compile the server
```

## Run the example

```sh
cyrius build examples/simple_crew.cyr build/simple_crew
./build/simple_crew
```

One crew, one task, no LLM. The file's header shows how to attach a hoosh client
(`agnosai_crew_runner_with_llm` + `agnosai_hoosh_client_new`) for real inference.

## Run the API server

```sh
./build/agnosai
```

It listens on `PORT`, else `AGNOSAI_PORT`, else 8080, and reaches hoosh at `HOOSH_URL`.
Auth is off unless `AGNOSAI_AUTH_ENABLED` is set — a shared secret via
`AGNOSAI_AUTH_SECRET`, or RS256 JWTs via `AGNOSAI_JWT_PUBLIC_KEY` (with optional
`AGNOSAI_JWT_ISSUER` / `AGNOSAI_JWT_AUDIENCE`). Rate limiting is on by default; see
[ADR 021](../adr/021-rate-limit-mounted-by-default.md). The README lists every route.

## Run the tests

```sh
cyrius tests tests            # every .tcyr, recursively
cyrius coverage --min 80      # the coverage gate
./scripts/check-clean.sh      # fmt, lint, doc, vet, deny, lock and lib-snapshot checks
```

## Using agnosai as a library

`cyrius distlib --all` writes `dist/agnosai.cyr` (the whole engine) and
`dist/agnosai-guard.cyr` (the output-filter and prompt-guard leaves only), each with a
`.deps` sidecar naming the stdlib leaves it needs. Vendor the bundle through a
`[deps.agnosai]` block and call the orchestrator directly —
`agnosai_orchestrator_submit_crew` — or stand up your own serve loop.

### Agent definitions from files

```cyrius
var err = 0;
var def = agnosai_load_from_file(str_from("agents/reviewer.yaml"), &err);   # .json / .yaml / .yml
var all = agnosai_load_all_from_dir(str_from("agents/"), &err);             # every definition in a directory
```

`agnosai_load_from_json` / `agnosai_load_from_yaml` take the document as a `Str`. YAML
goes through bayan's YAML subset parser.

### AGNOS ecosystem tools

Synapse, Mneme and Delta tools are built on a client pointed at the service:

```cyrius
var mneme = agnosai_agnos_client(str_from("http://localhost:8400"), str_from("mneme"));
agnosai_tool_registry_register(registry, agnosai_mneme_search_tool(mneme));
```

The default base URLs are `AGNOSAI_SYNAPSE_BASE_URL` (8420), `AGNOSAI_MNEME_BASE_URL`
(8400) and `AGNOSAI_DELTA_BASE_URL` (8070) in `src/tools/agnos.cyr`. The default server
binary registers none of them — a deployment opts in.

## Adding a feature

1. Put the code in the module it belongs to under `src/` (the layout mirrors the
   original module tree), prefixing every public symbol `agnosai_`.
2. Add a suite, or assertions to an existing one, under `tests/`.
3. Run `cyrius tests tests` and `./scripts/check-clean.sh`.
4. Add a CHANGELOG entry; `sh scripts/version-bump.sh X.Y.Z` cuts a release.

See [`../adr/`](../adr/) for the decisions already made, and add an ADR when a
non-obvious choice deserves one.
