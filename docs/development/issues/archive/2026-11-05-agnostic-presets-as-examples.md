# agnosai's preset set should become a small example library — RESOLVED

**Discovered:** recorded by agnostic, 2026-08-20. **Severity:** Low.
**Status:** ✅ **RESOLVED in agnosai 2.2.4** (2026-11-05). Was 🔴 OPEN.
**Resolution:** the set is three examples, `security-lean`, `security-standard` and `security-large` ([ADR 027](../../../adr/027-the-built-in-presets-are-an-example-library.md)). They are the three agnosai names agnostic's library does not use, so no name collides, and they show the size ladder of two, four and six agents. The other fifteen documents are deleted. agnostic calls none of this, so nothing moves on its side.

agnosai's 18 presets in `src/presets/` and agnostic's 18 share a naming scheme and no content: 15
names collide, not one with a matching description, roster or tool vocabulary. agnostic's library is
canonical.

**Ask:** reduce agnosai's set to a small illustrative example library, so the two stop diverging by
accident.
