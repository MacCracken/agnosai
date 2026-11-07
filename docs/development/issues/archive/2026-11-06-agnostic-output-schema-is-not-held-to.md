# A task with an `output_schema` completes whether or not it met it, and the model is not told the shape until it has failed — RESOLVED

**Status:** ✅ **RESOLVED in agnosai 2.2.5** (2026-11-06). Was 🟡 OPEN.
**Resolution:** every answer is checked, the last included, so a task gets `AGNOSAI_MAX_VALIDATION_RETRIES` retries, each followed by a check. A task whose answers never met its schema is `Failed`: `output` is what the model last said, filtered as any answer is, metadata `error` says why, `validation_attempts` says how many were checked, and the usage metadata is there, summed over every call. It emits no `token` event, and a DAG's dependents do not run. The schema is in the first prompt, sanitized as `expected_output` is. A fenced answer that passes is returned as the block, without its fence. **For agnostic:** nothing to change. `output_valid` reads only completed tasks, and a failed one carries `error`.
**Placement:** unpinned. agnostic 0.4.0 forwards `output_schema` and reports the result on its own side, so nothing waits on this, but its reviewer preset and M8's structured result both lean on it.
**Discovered:** 2026-11-06, wiring `output_schema` through agnostic. Read from the source; not reproduced against a model.
**Severity:** Medium. Nothing crashes. A consumer that declares a shape is told `completed` about an answer that does not have it.
**Affects:** agnosai 2.2.4.

## Summary

Two things in `_agnosai_crew_execute` (`src/orchestrator/crew_runner.cyr:712-760`), both found by reading it.

### 1. The last answer is never checked, and the task completes either way

`AGNOSAI_MAX_VALIDATION_RETRIES` is 2, and the loop checks before it retries:

```
attempt 1: validate response 0, fails, ask again -> response 1
attempt 2: validate response 1, fails, ask again -> response 2
loop ends: response 2 is never validated
```

Then the task result is built with `AGNOSAI_STATUS_COMPLETED` regardless. So a model that never produces the shape gets three calls, the crew reads `completed`, and nothing in the result says the output is not what was declared. The same is true of a response 1 that failed when no retry was left.

### 2. The first prompt does not carry the schema

`agnosai_task_output_schema` has one reader, at `crew_runner.cyr:722`, which is that loop. `_agnosai_crew_user_message` sends the description and `expected_output`, and `_agnosai_crew_build_request` sends the system prompt and the context. The schema reaches the model only inside `agnosai_build_retry_prompt`, that is, after it has already failed.

So unless the task's text happens to describe the shape, every task with a schema pays for a call that could not have passed, on the model's first attempt.

## What agnostic does today

- It forwards `type` and `required`, the two keys `agnosai_validate_value` reads, and refuses anything else by name, because `properties` would be accepted and enforce nothing.
- It runs `agnosai_extract_and_validate` itself, once per crew, and reports `output_valid` on the task's result, so a caller can tell. That check stays useful after this is fixed, and if agnosai fails such a task instead, agnostic has nothing to change.
- It does **not** copy the schema into the task's context to reach the first prompt, because that is a way round this rather than a fix. Its documentation tells callers to describe the shape in `expected_output` meanwhile.

## The ask

1. Validate the last response too, so that `MAX_VALIDATION_RETRIES` retries are each followed by a check, and decide what an answer that never passed is: a `Failed` result, or a completed one that says so in its metadata. A consumer can read either. `completed` with no word of it is the one it cannot.
2. Put the schema in the first prompt. It belongs beside `expected_output` in `_agnosai_crew_user_message`, sanitized the same way.

One smaller thing, if the loop is opened anyway: a fenced answer that passes is returned with its fence, as `agnosai_inference_response_content(response)` and not the extracted block, so a consumer that parses `output` has to strip it again.

## Done means

A task with a schema that the model never meets does not read as a clean `completed`, and a model given a schema on its first call can pass on its first call. `tests/orch_crew_runner.tcyr` has no case of either: nothing there drives the retry loop.
