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
    var a = agnosai_tool_input_alloc(input);
    var value = agnosai_tool_input_get(input, str_from_a(a, "message"));
    if (value == 0) {
        return agnosai_tool_output_err_a(a, str_from_a(a, "missing required parameter: message"));
    }
    return agnosai_tool_output_ok_a(a, value);
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

- `agnosai_tool_output_ok_a(a, value)` — success, carrying a `bayan` JSON value.
- `agnosai_tool_output_err_a(a, msg: Str)` — failure; `result` becomes JSON null.

`a` is `agnosai_tool_input_alloc(input)`, the allocator this call's answer belongs in. Build everything the tool answers
with it, through the `_a` forms. On the tool loop that is an arena of the call's own, so what the tool built goes back
with the turn and a crew leaves nothing behind. Off the loop it is the global allocator, so the tool needs no second
path. The bare `agnosai_tool_output_ok` and `_err` still work, and what they build stays on the global heap for good.

⚠ **Not `str_cstr`.** It copies onto the global heap whatever it is given. `str_data(str_clone_a(a, s))` is NUL-terminated
and goes back with the call.

⚠ **The allocator is this call's, and this thread's.** An arena has no lock, so a tool that starts threads gives each one
its own. Anything that outlives the call, a crew it submits or state on its `ctx`, does not belong in it: `delegate`
answers there and builds its crew elsewhere.

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

## Holding tools to a gate

A consumer can put a gate between the model and its tools, so a call is judged against what its
task declared rather than only what the model wrote
([ADR 025](../adr/025-a-tool-call-passes-a-consumers-gate.md)). No tool has to implement it: a gate
is read from the tool at the chokepoint every call passes through.

```cyrius
# Refuse a call before it runs: answer the reason, or 0 to let it through.
fn my_gate_call(ctx, input): i64 {
    var task = agnosai_tool_input_task(input);  # 0 for a call no task made
    if (my_rule_allows(task, input) == 1) { return 0; }
    return str_from("not what the task declared");
}

# Judge a target and every redirect hop. Compare hosts, never URL prefixes.
fn my_gate_url(ctx, input, url: Str): i64 {
    if (agnosai_url_same_host(url, my_declared_url(input)) == 1) { return AGNOSAI_GATE_ADMIT; }
    return AGNOSAI_GATE_REFUSE;
}

agnosai_tool_registry_set_gate(registry, agnosai_tool_gate_new(&my_gate_call, &my_gate_url, 0));
```

`AGNOSAI_GATE_ADMIT` gets a declared host past `agnosai_is_safe_url`, private networks included,
`AGNOSAI_GATE_REFUSE` stops any URL, and `AGNOSAI_GATE_PASS` leaves it to that guard. Any other value
refuses, and no admission reaches a cloud metadata endpoint.

A tool that fetches a URL itself should return what `agnosai_tool_target_refused(input, url)` gives
it, if that is not 0, before it does, and take its redirects through `agnosai_guarded_fetch_hops`
with `&agnosai_tool_url_ok`, as `security_audit` does.

`agnosai_task_result_record_tool_call(result, tool, out)` keeps a call's name, result and effect on
its task result, under `metadata.tool_calls`, so a consumer can label a measured number as measured.
The tool loop writes these itself.

## Giving tools to a crew

An orchestrator with a registry runs a crew's tools ([ADR 029](../adr/029-the-tool-loop.md)). The registry stays
yours, so serve the same one on `/mcp` or register to it at any time:

```cyrius
var tools = agnosai_tool_registry_new();
agnosai_tool_registry_register(tools, my_tool());
agnosai_orchestrator_with_tools(orchestrator, tools);
```

An agent is offered exactly the tools its `tools` names, and a name the registry does not hold fails the task before
the first call, naming the tool. An agent that names none is offered none. With no registry on the orchestrator, an
agent's `tools` are words in its prompt.

The model calls them until it answers, within 16 turns, 8 calls a turn, 32 KiB a result and 256 KiB of results a task
(`agnosai_tool_loop_set_limits`). What each call did is on the task result, in the order the model made them, under
`metadata.tool_calls`: `id`, `tool`, `arguments`, `effect`, `success`, `result` or `error`, and `flagged`, `held_back` or
`asked_for` when they apply. `effect` is `committed`, `none` (it never started) or `unknown` (it may have run).

⚠ **An input lasts as long as its call.** Through the loop it lives in an arena that goes back with the task, so a tool
that keeps anything from it past `execute_fp`'s return, for a crew it submits or a thread it leaves running, must copy it
first. What a tool returns is copied for you, so it may live in `agnosai_tool_input_alloc`, and a gate or a journal that
keeps an output past its turn must copy it: `agnosai_tool_call_output` reads 0 once the turn is over.

**A journal** writes every call down before it runs. A call it refuses does not start, and an intent with no result
after a crash reads as an unknown effect, so have it on disk before you return. Key entries on the task and the call's id
together: an id is unique within its task, and only there.

```cyrius
fn my_journal(ctx, phase, call): i64 {
    if (phase == AGNOSAI_TOOL_PHASE_INTENT) {
        if (my_store_intent(agnosai_task_id(agnosai_tool_call_task(call)), agnosai_tool_call_id(call),
            agnosai_tool_call_name(call), agnosai_tool_call_arguments(call)) == 0) {
            return str_from("the journal is full");
        }
        return 0;
    }
    my_store_result(agnosai_task_id(agnosai_tool_call_task(call)), agnosai_tool_call_id(call),
      agnosai_tool_call_effect(call));
    return 0;
}

agnosai_orchestrator_with_tool_journal(orchestrator, agnosai_tool_journal_new(&my_journal, 0));
```

What it is given dies with the task, so copy what you keep. A gate has two more callbacks for the loop:

```cyrius
# Judge the calls of one turn as a set, before any runs. A reason refuses them all.
fn my_gate_batch(ctx, calls): i64 {
    if (reads_a_secret(calls) == 1 && posts_to_the_web(calls) == 1) {
        return str_from("those leak together");
    }
    return 0;
}

# Answer text for the model to read instead of a result, or 0 to let it through. Asked only about a
# call that ran. `out` may be 0 for a tool that answered nothing, and `finding` is what the injection
# scan said, or 0. The task's record keeps the result either way.
fn my_gate_result(ctx, input, out, finding): i64 {
    if (finding != 0) { return str_from("result held back: it matched an injection pattern"); }
    return 0;
}

var gate = agnosai_tool_gate_new(&my_gate_call, &my_gate_url, 0);
agnosai_tool_gate_with_batch(gate, &my_gate_batch);
agnosai_tool_gate_with_result(gate, &my_gate_result);
```

A result reaches the model as data, inside the markers its system prompt says never to obey, and by default a flagged
one is annotated rather than held back: the scan is a heuristic, and a page should not be able to hide itself from a
test by saying a magic phrase.

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
