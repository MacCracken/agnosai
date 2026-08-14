# Adding a Native Tool

⚠ **This guide used to describe a Rust `NativeTool` trait with `async fn` and
`Pin<Box<dyn Future>>`.** None of that exists on the Cyrius line. A tool is a
**vtable**: two function pointers plus an opaque context.

Native tools run in-process with zero overhead — and with the server's full
privileges. Review one as carefully as you would review the server itself.

## The shape

`agnosai_tool_new` (`src/tools/native.cyr`) takes a name, a description, and two
function pointers:

```cyr
fn agnosai_tool_new(name: Str, description: Str, schema_fp, execute_fp, ctx): i64
```

| slot | signature | purpose |
|---|---|---|
| `schema_fp` | `fn(a, ctx) -> schema` | declares parameters; `a` is an allocator |
| `execute_fp` | `fn(ctx, input) -> output` | does the work |
| `ctx` | any pointer, or `0` | per-tool state, handed back to both |

⚠ **`schema_fp` takes the allocator FIRST.** Every one of the fourteen
implementors threads it, which is what lets `GET /api/v1/tools` render with zero
global-bump allocation. A schema built with the bare `agnosai_tool_schema_new`
leaks per request.

## A complete tool

This is `echo`, verbatim from `src/tools/builtin/basic.cyr` — the whole thing:

```cyr
fn _agnosai_echo_schema(a, ctx): i64 {
    var s = agnosai_tool_schema_new_a(a, str_from_a(a, "echo"),
    str_from_a(a, "Returns the input message unchanged. Useful for testing."));
    agnosai_tool_schema_param_a(a, s, str_from_a(a, "message"),
    str_from_a(a, "The message to echo back"), str_from_a(a, "string"), 1);
    return s;
}

fn _agnosai_echo_execute(ctx, input): i64 {
    var value = agnosai_tool_input_get(input, str_from("message"));
    if (value == 0) {
        return agnosai_tool_output_err(str_from("missing required parameter: message"));
    }
    return agnosai_tool_output_ok(value);
}

fn agnosai_echo_tool(): i64 {
    return agnosai_tool_new(str_from("echo"),
    str_from("Returns the input message unchanged. Useful for testing."),
    &_agnosai_echo_schema, &_agnosai_echo_execute, 0);
}
```

The trailing `1` on `agnosai_tool_schema_param_a` marks the parameter
**required**.

## Reading input

| accessor | returns |
|---|---|
| `agnosai_tool_input_get(input, key)` | a `bayan` JSON value, or `0` when absent |
| `agnosai_tool_input_get_int(input, key)` | an integer, or `AGNOSAI_NO_LIMIT` |
| `agnosai_tool_input_get_f64(input, key, &found)` | an f64, with a found flag |

⚠ **`agnosai_tool_input_get_int` treats a NEGATIVE as absent**, because the
oracle's `as_u64` returns `None` for one. Do not read `-1` as a value — it is the
sentinel.

## Returning output

- `agnosai_tool_output_ok(value)` — success, carrying a `bayan` JSON value.
- `agnosai_tool_output_err(msg: Str)` — failure; `result` becomes JSON null.

Say **why** in the error. A fixed string like "request failed" for every failure
mode is a real defect — this port shipped one and had to fix it.

## Registering

```cyr
agnosai_tool_registry_register(registry, agnosai_echo_tool());
```

⚠ The registry is a hashmap behind a **futex mutex**, and that mutex is
mandatory: `sandhi_server_run_pooled` makes every worker a real OS thread, so an
unguarded write during a concurrent read corrupts the table. Registering the same
name twice **replaces** rather than duplicating.

## Naming rules

- **Prefix every public symbol `agnosai_*`.** Cyrius has ONE flat namespace and
  last-definition-wins. `_`-prefix internals so they leave the coverage
  denominator.
- ⚠ **Never name a function `X_str`, `X_int` or `X_ptr` alongside an `X`.** Those
  are reserved overload slots — Cyrius silently routes `X(a, …)` to `X_str` when
  `a` is Str-typed, and to `X_int` when `a` is a bare call result. The symptom is
  the wrong function body running, with no error.

## Testing

Suites live in `tests/*.tcyr` and may `include "src/tools/…"` directly. Cover
the absent-parameter arm, the wrong-type arm, and the success arm — then
**mutation-verify**: break the tool, confirm a named assertion fails, restore.
