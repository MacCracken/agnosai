# 010 — A configured `iss` or `aud` is required, not merely matched

## Status: Accepted

## Context

`rust-old/src/server/auth.rs:128-136` configures claim validation like this:

```rust
if let Some(ref iss) = config.issuer {
    validation.set_issuer(&[iss]);
}

if let Some(ref aud) = config.audience {
    validation.set_audience(&[aud]);
} else {
    validation.validate_aud = false;
}
```

The intent reads clearly: if the operator configures an issuer, tokens must come
from that issuer.

**jsonwebtoken does not implement that.** `Validation::set_issuer`
(`jsonwebtoken-10.3.0/src/validation.rs:143-145`) is:

```rust
pub fn set_issuer<T: ToString>(&mut self, items: &[T]) {
    self.iss = Some(items.iter().map(|x| x.to_string()).collect())
}
```

It sets the expected value and **nothing else**. It does not add `"iss"` to
`required_spec_claims`, which `Validation::new` seeds as `{"exp"}` only
(`validation.rs:113-118`). The validation loop (`validation.rs:259-272`) checks
`required_spec_claims` for presence, and the issuer comparison only runs when
the claim is `Parsed`. A token carrying **no `iss` member at all** therefore
passes issuer validation even when an issuer is configured. `set_audience` has
the identical shape.

The practical consequence: an operator who sets `AGNOSAI_JWT_ISSUER` gets
enforcement against a token bearing the *wrong* issuer, and **no enforcement at
all** against a token bearing *no* issuer. Since `JwtConfig` holds one static
public key with no `kid` routing and no JWKS (`auth.rs:68-75`), `iss` and `aud`
are the only cross-tenant separation the design has. Anyone holding a token
signed by that key can strip `iss` and reach every endpoint.

This is not a Rust-only artifact; it is what the wire contract does today.
CLAUDE.md's rule — a Cyrius behaviour that diverges *silently* from Rust is the
worst outcome — means the choice must be explicit either way.

## Decision

**Require the claim when, and only when, it is configured.** If `JwtConfig`
carries an issuer, a token must present a matching string `iss`; absent is a
rejection, not a pass. Same for `aud`.

The tightening is *conditional*, mirroring the oracle's own `if let Some(..)`
shape: `JwtConfig::new(key)` with neither configured still accepts tokens
carrying neither claim, so the default path is unchanged.

## Consequences

**Zero oracle assertions change.** The oracle's `valid_claims()` fixture
(`auth.rs:332-341`) always populates both `iss` and `aud`, and its three claim
tests mutate them to *wrong* values, never to absent. All four of the oracle's
JWT tests pass against the tightened implementation unmodified.

**What changes is a case the oracle has no test for**, and which an operator
would report as a bug rather than rely on. `tests/server_auth.tcyr`
(`_t_jwt_absent_iss_aud_rejected`) pins the new behaviour, and
`_t_jwt_tightening_is_conditional` pins that the unconfigured path is untouched
— including the mixed case where an issuer is configured but an audience is not.

**Wire impact for a real deployment.** Any identity provider worth using emits
`iss` and `aud` on every token, so this rejects nothing a conforming IdP
produces. A hand-rolled token generator that omitted them would start getting
401s — which is the point.

**Not fixed here, and deliberately.** Two neighbouring oracle behaviours are
*reproduced* rather than tightened, because unlike this one they are fail-closed:
an array-valued `aud` is rejected (the oracle rejects it too, via a
deserialization failure into `Option<String>` at `decoding.rs:285-287`, including
the single-element form), and jsonwebtoken's 60-second `exp` leeway
(`validation.rs:120`) is reproduced as a named constant. Tightening a
fail-closed quirk buys nothing; tightening this one closes a bypass.

⚠ The full decision record used to be pointed at in `docs/development/state.md`,
under "Four decisions waiting on the maintainer" — **that section is gone**;
state.md now carries a "Standing decisions that shaped the port" table which
links here. The decisions live at their call sites in `src/server/auth.cyr`: this
one as D1 (:702), the array-`aud` rejection as D3 (:720), and the leeway constant
as D4 (`AGNOSAI_JWT_LEEWAY_SECS`, :478).

**Precedent.** Where the oracle's stated intent and its actual behaviour disagree
and the gap is a *weakening*, the port implements the intent and records it here.
Where the gap is fail-closed or merely surprising, the port reproduces the
behaviour. [ADR 009](009-auth-constant-time-secret-compare.md) is the same
judgement applied to the shared-secret compare.
