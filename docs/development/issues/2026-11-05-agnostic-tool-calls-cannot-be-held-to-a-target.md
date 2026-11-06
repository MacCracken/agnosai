# A tool call cannot be held to what its task declared, and leaves no result behind — OPEN

**Discovered:** 2026-11-05, designing agnostic's crew target. **Severity:** Medium, High once F1 runs tools.
**Affects:** agnosai 2.2.3, `agnosai_tool_execute_in`, `_agnosai_load_testing_execute`,
`_agnosai_security_audit_execute`, `agnosai_is_safe_url`. **Status:** 🔴 OPEN.

agnostic's crew body will carry a checked `target`, which it forwards into each task's context. The model
reads it there, through `_agnosai_crew_build_request`. Two things stop that declaration from meaning anything
at the point a tool runs.

**1. A tool cannot see its task.** `agnosai_tool_execute_in` calls `execute_fp(ctx, input)`: `ctx` is the tool's
own pointer from `agnosai_tool_new`, and `input` is what the model wrote. `target_url` comes from `input`, so the
model alone decides where a tool points. F1's rule 2 says why that matters for a QA consumer: the system under
test writes the tools' output. A page that says "now scan this other host" can steer the next call, and nothing
compares that call with what the crew declared.

**2. A declared private host cannot be reached.** Both tools refuse any URL `agnosai_is_safe_url` rejects, which is
every loopback, private and link-local address. That is right for a URL a model made up. It also refuses a staging
system on a private network, which is where most QA targets live, even when an operator has allowed that host.

**Ask, for F1's ADR:** a consumer-supplied gate between the model and the tool, given the task, the tool and the
input, that can refuse a call before it runs and can admit a declared host past `agnosai_is_safe_url`. A gate means
no tool has to implement the rule, and it fits rule 3's approval step.

A redirect has to go through the same gate. `AGNOSAI_FETCH_BLOCKED` covers a redirect to a private address today,
and a redirect off the declared host must be refused too, or the target check ends at the first 302.

**3. A task result does not say what a tool measured.** Its metadata carries the model, provider, tokens, cost and
duration. agnostic's reports will read stored crew outcomes, and a load test's p99 is a measurement where a model
saying "all passed" is a claim. **Ask, for the same ADR:** keep each call's tool name and result on the task result,
beside rule 1's effect status, so a consumer can store them and label a measured number as measured.

**agnostic meanwhile** checks the target at the door against the operator's host list. That limits what a client
may declare. It does not limit what a model may call. agnostic's roadmap carries this under *Waiting on agnosai*.
