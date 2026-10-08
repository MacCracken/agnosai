#!/usr/bin/env python3
"""check-log-lengths.py — a log literal's declared length must be its real length.

sakshi's logging API takes (pointer, length) pairs rather than NUL-terminated
strings, so the byte count is hand-written at every call site and nothing checks
it. Both ways of getting it wrong fail SILENTLY:

  * **One too many** hands sakshi the NUL terminator as part of the message. In
    JSON output that is an escaped NUL inside the string — legal JSON that breaks
    naive consumers. `src/main.cyr`'s shutdown line ended `gracefully\u0000` that way.
  * **One too few** truncates the last characters. A multi-byte `—` counted as one
    byte did that to 16 messages here, up to 5 bytes short.

Neither is a compile error, neither is a lint warning, and neither fails a test:
the suites check what handlers return, not what they log. Adopted verbatim from
agnostic (its `scripts/check-log-lengths.py`) at 2.1.7, roadmap B28, which found
17 such literals in this tree; `scripts/check-clean.sh` runs it.

Exits non-zero on any mismatch. Run from the repo root.
"""
import glob
import re
import sys

# sakshi_info("msg", N) and its level siblings.
BARE = re.compile(
    r'\b(sakshi_(?:info|warn|error|debug|trace))\(\s*"((?:[^"\\]|\\.)*)"\s*,\s*(\d+)\s*\)')

# agnostic_log_kv_str/int(LEVEL, "msg", N, "key", K, ...)
KV = re.compile(
    r'\b(agnostic_log_kv_(?:str|int))\(\s*[A-Za-z_]+\s*,\s*'
    r'"((?:[^"\\]|\\.)*)"\s*,\s*(\d+)\s*,\s*'
    r'"((?:[^"\\]|\\.)*)"\s*,\s*(\d+)\s*,')

# sakshi_log_kv(LEVEL, "msg", N, "key", K, ...)
SAKSHI_KV = re.compile(
    r'\bsakshi_log_kv\(\s*[A-Za-z_]+\s*,\s*'
    r'"((?:[^"\\]|\\.)*)"\s*,\s*(\d+)\s*,\s*'
    r'"((?:[^"\\]|\\.)*)"\s*,\s*(\d+)\s*,')


def byte_len(literal):
    """The length sakshi is actually handed: bytes, after resolving escapes."""
    return len(literal.encode().decode("unicode_escape").encode("utf-8"))


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def describe(declared, actual):
    if declared == actual + 1:
        return "declares the NUL terminator"
    if declared > actual:
        return "overruns by %d" % (declared - actual)
    return "truncates by %d" % (actual - declared)


def check(path, bad):
    text = open(path, errors="ignore").read()

    def note(pos, what, literal, declared):
        actual = byte_len(literal)
        declared = int(declared)
        if actual == declared:
            return
        bad.append(
            '    %s:%d  %s declared %d, actual %d (%s)\n        "%s"'
            % (path, line_of(text, pos), what, declared, actual,
               describe(declared, actual), literal))

    for m in BARE.finditer(text):
        note(m.start(), m.group(1) + " message", m.group(2), m.group(3))
    for m in KV.finditer(text):
        note(m.start(), m.group(1) + " message", m.group(2), m.group(3))
        note(m.start(), m.group(1) + " key", m.group(4), m.group(5))
    for m in SAKSHI_KV.finditer(text):
        note(m.start(), "sakshi_log_kv message", m.group(1), m.group(2))
        note(m.start(), "sakshi_log_kv key", m.group(3), m.group(4))


def main():
    bad = []
    paths = (sorted(glob.glob("src/**/*.cyr", recursive=True))
             + sorted(glob.glob("tests/*.tcyr"))
             + sorted(glob.glob("benches/*.bcyr")))
    for p in paths:
        check(p, bad)
    if bad:
        print("\n".join(bad))
        print("")
        print("error: a log literal's declared length does not match the literal.")
        print("       One too many hands sakshi the NUL terminator; one too few")
        print("       truncates the message. Both fail silently at runtime.")
        return 1
    print("log lengths OK — every declared length matches its literal across %d files"
          % len(paths))
    return 0


sys.exit(main())
