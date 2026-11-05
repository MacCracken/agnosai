# kavach: no WASM store limits beyond memory size and fuel

**Found by:** agnosai (2026-11-05, roadmap B16 re-check). **Version:** kavach 3.14.2, wasmtime 50.0.1.

`_wasm_append_limits` emits only `-W max-memory-size=…` and `-W fuel=…`. The captured argv is
`wasmtime run -W max-memory-size=67108864 -W fuel=1000000000 -- <module>`. There is no config setter or
passthrough for `max-instances`, `max-tables`, `max-table-elements`, `max-memories` or
`trap-on-grow-failure`, all of which wasmtime 50 offers (`wasmtime run -W help`).

**Ask:** config setters for them, emitted by `_wasm_append_limits`, and a sane default for
`trap-on-grow-failure` (on). **agnosai** would map its WASM policy onto them. The Rust line's store limits
were 10 instances, tables and memories, and a trap on a failed grow.
