# Adding an LLM Provider

⚠ **Read this before anything else: there is almost certainly nothing to add
here.** This guide used to describe a Rust `LlmProvider` trait with per-provider
`reqwest` calls. That design does not exist on the Cyrius line, and the thing it
described — a new HTTP client per vendor — is **not** how a provider is added.

## AgnosAI has ONE outbound LLM path

Everything goes to the AGNOS **hoosh** gateway over its OpenAI-compatible
`/v1/chat/completions` (`src/llm/hoosh.cyr`). There is no `/v1/messages` client,
no `/api/chat` client, and no vendor SDK anywhere in the tree.

**hoosh owns the per-vendor protocols.** AgnosAI addresses the gateway by
**model string**; hoosh maps that to a vendor and speaks that vendor's wire.

So:

- **To support a new vendor** — add it to **hoosh**, not here. Nothing in this
  repo needs to change; a new model string starts working the moment the gateway
  understands it.
- **To make agnosai *label* a vendor** (for routing tiers, cost attribution and
  metadata), extend the enum below. That is a label, not a transport.

## The provider labels

`AgnosaiProviderType` in `src/llm/hoosh.cyr` — seven variants:

```cyr
enum AgnosaiProviderType {
    AGNOSAI_PROVIDER_OPENAI = 0;
    AGNOSAI_PROVIDER_ANTHROPIC = 1;
    AGNOSAI_PROVIDER_GOOGLE = 2;
    AGNOSAI_PROVIDER_MISTRAL = 3;
    AGNOSAI_PROVIDER_DEEPSEEK = 4;
    AGNOSAI_PROVIDER_GROK = 5;
    AGNOSAI_PROVIDER_OLLAMA = 6;
}
```

To add one:

1. **Append** a variant. ⚠ Append — the numbering is positional and existing
   values must not shift.
2. Add its spelling to `agnosai_provider_to_wire` and
   `agnosai_provider_from_wire`. ⚠ `from_wire` reads an unknown spelling as
   `OpenAi` rather than failing, matching the oracle.
3. Teach `_agnosai_crew_infer_provider` (`src/orchestrator/crew_runner.cyr`) the
   model-string prefix that implies it, if there is one.
4. Extend the router's tier matrix in `src/llm/router.cyr` if the vendor should
   participate in Fast / Capable / Premium selection.

⚠ **These spellings are not on any wire agnosai controls.** The comment above the
enum says so: they are the conventional `snake_case` forms, but the gateway is
addressed by model string. If a caller ever needs them to match hoosh
byte-for-byte, **verify against hoosh** rather than trusting the enum.

## Why the gateway seam exists

hoosh is a binary, not a library, so it cannot be `include`d — it is consumed
over HTTP. That is stated at the top of `src/llm/hoosh.cyr`. The seam is what
keeps vendor churn out of this repo entirely: agnosai has never needed a change
to gain a vendor.

## Testing

`tests/llm_hoosh.tcyr` and `tests/llm_router.tcyr` drive the request builder and
the tier matrix without a network, by feeding the response decoder captured
bytes. Add a case there rather than standing up a live gateway.
