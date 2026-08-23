#!/usr/bin/env python3
"""Rules 4 and 5 — no two lib/ modules in the COMPILE SET define one name two ways.

Rule 4 covers constants (`var`, enum members); Rule 5 covers `fn` bodies.

Cyrius has one flat symbol table with last-definition-wins, and the compiler is SILENT
for `var` and for enum members (only a duplicate `fn` warns). A lib/<->lib/ collision
between two dependencies is therefore invisible to the compiler, to Rules 1-3 (which
scan src/), and to every other gate. That is exactly how kavach's BACKEND_COUNT (10)
was silently resolved to ai-hwaccel's 18, defeating a bounds check on a function-pointer
table -- see CHANGELOG 2.0.4.

This fails the build on a duplicate whose VALUES DIVERGE, and reports the rest.
Known-and-filed divergences live in scripts/lib-symbol-allow.txt, one per line, each
required to carry a `#` comment naming the upstream issue.
"""
import re, sys, os, glob, collections

# Platform-exclusive stdlib variants: these define the same names ON PURPOSE and are
# never co-compiled. Excluding them is correct for a duplicate-detection gate.
PLAT = re.compile(r'_(macos|win|windows|agnos|linux_common|aarch64|aarch64_linux|x86_64|x86_64_linux|x86_64_agnos)\.cyr$')

def strip_comments(s):
    r"""Drop `#` comments, respecting quotes.

    ⚠ Required, not cosmetic. A naive non-greedy `\[(.*?)\]` over the raw manifest
    stops at the first `]` -- and these manifests carry comments that mention other
    blocks by name, e.g. "agnosai declares `[deps.sigil]` at 3.12.9" INSIDE the
    stdlib array. That truncated the parsed stdlib list and silently dropped
    modules from the compile set, which is the precise failure mode this gate
    exists to prevent it having.
    """
    out, q = [], None
    for line in s.split("\n"):
        buf = []
        for ch in line:
            if q:
                buf.append(ch)
                if ch == q:
                    q = None
            elif ch in '"\'':
                q = ch; buf.append(ch)
            elif ch == '#':
                break
            else:
                buf.append(ch)
        out.append("".join(buf))
        q = None          # cyml has no multi-line strings here
    return "\n".join(out)

def _array_after(s, key):
    """Text of the bracket-balanced array following `key =`, or None."""
    m = re.search(r'^\s*' + re.escape(key) + r'\s*=\s*\[', s, re.M)
    if not m:
        return None
    i = s.index('[', m.start()); depth = 0
    for j in range(i, len(s)):
        if s[j] == '[': depth += 1
        elif s[j] == ']':
            depth -= 1
            if depth == 0:
                return s[i + 1:j]
    return None

def parse_manifest(path="cyrius.cyml"):
    s = strip_comments(open(path).read())
    arr = _array_after(s, "stdlib")
    stdlib = set(re.findall(r'"([^"]+)"', arr)) if arr else set()
    deps = {}
    for name in re.findall(r'^\[deps\.([A-Za-z0-9_-]+)\]', s, re.M):
        blk = re.search(r'^\[deps\.' + re.escape(name) + r'\](.*?)(?=^\[|\Z)', s, re.S | re.M).group(1)
        tag = (re.search(r'^tag\s*=\s*"([^"]+)"', blk, re.M) or [None, None])[1]
        pth = (re.search(r'^path\s*=\s*"([^"]+)"', blk, re.M) or [None, None])[1]
        marr = _array_after(blk, "modules")
        mods = re.findall(r'"([^"]+)"', marr) if marr else []
        deps[name] = {"tag": tag, "path": pth, "modules": mods}
    return stdlib, deps

def sidecar_leaves(name, info):
    """Stdlib leaves a dep's fold needs, from its dist/<mod>.deps sidecar.
    Looked up in the cyrius dep cache first (present on a CI runner) then the
    sibling checkout (present locally)."""
    out = set()
    for mod in info["modules"]:
        base = mod[:-4] + ".deps" if mod.endswith(".cyr") else mod + ".deps"
        cands = []
        if info["tag"]:
            cands.append(os.path.expanduser(f"~/.cyrius/deps/{name}/{info['tag']}/{base}"))
        if info["path"]:
            cands.append(os.path.join(info["path"], base))
        cands.append(os.path.join("..", name, base))
        for c in cands:
            if os.path.exists(c):
                for L in open(c):
                    L = L.strip()
                    if L and not L.startswith("#"):
                        out.add(L)
                break
    return out

def compile_set(stdlib, deps):
    """Every lib/ module that actually reaches the compile unit."""
    names = set(stdlib)
    dist_basenames = set()
    for n, info in deps.items():
        names |= sidecar_leaves(n, info)
        for mod in info["modules"]:
            dist_basenames.add(os.path.basename(mod))

    files = set()
    for n in names:
        p = f"lib/{n}.cyr"
        if os.path.exists(p):
            files.add(p)
    # Every non-stdlib file in lib/ is a dep dist that `cyrius deps` provisioned --
    # including transitive ones no [deps.*] block names directly (libro arrives via
    # bote). They all compile, so they all count.
    # Locate the pinned stdlib snapshot, so "not in the snapshot" identifies a dep
    # dist. CI's installer lays out $HOME/.cyrius/{bin,lib} and does NOT necessarily
    # create versions/<pin>/lib, so try the layouts in order of specificity. If none
    # is found we must NOT quietly fall through to a dist_basenames-only scan: that
    # would silently drop the TRANSITIVE dists (libro arrives via bote), making this
    # gate weaker in CI than on a developer machine -- the exact shape of blind spot
    # it exists to close.
    snap = None
    mp = re.search(r'^cyrius\s*=\s*"([^"]+)"', open("cyrius.cyml").read(), re.M)
    home = os.environ.get("CYRIUS_HOME", os.path.expanduser("~/.cyrius"))
    cands = []
    if mp:
        cands.append(os.path.join(home, "versions", mp.group(1), "lib"))
        cands.append(os.path.expanduser(f"~/.cyrius/versions/{mp.group(1)}/lib"))
    cands.append(os.path.join(home, "lib"))
    cands.append(os.path.expanduser("~/.cyrius/lib"))
    for cand in cands:
        if os.path.isdir(cand) and glob.glob(os.path.join(cand, "*.cyr")):
            snap = {os.path.basename(f) for f in glob.glob(os.path.join(cand, "*.cyr"))}
            break
    if snap is None:
        print("error: cannot locate the pinned Cyrius stdlib snapshot; tried:", file=sys.stderr)
        for c in cands:
            print(f"         {c}", file=sys.stderr)
        print("       Without it, transitive dep dists cannot be told from stdlib modules",
              file=sys.stderr)
        print("       and this check would silently cover less than it appears to.",
              file=sys.stderr)
        return 1
    for p in glob.glob("lib/*.cyr"):
        b = os.path.basename(p)
        if b in dist_basenames or (snap is not None and b not in snap):
            files.add(p)
    return {f for f in sorted(files) if not PLAT.search(f)}

def constants(path):
    """(name, value, kind) for every top-level `var` and every enum member."""
    out, lines, i = [], open(path, errors="replace").read().split("\n"), 0
    while i < len(lines):
        L = lines[i]
        m = re.match(r'^var\s+([A-Za-z_]\w*)\s*=\s*([^;]+);', L)
        if m:
            out.append((m.group(1), m.group(2).strip(), "var"))
        if re.match(r'^enum\s+[A-Za-z_]\w*', L):
            j, auto = i + 1, 0
            while j < len(lines) and not re.match(r'^\}', lines[j]):
                mm = re.match(r'^\s*([A-Za-z_]\w*)\s*(?:=\s*([^,;\n#]+))?\s*[,;]?\s*(?:#.*)?$', lines[j])
                if mm and mm.group(1):
                    raw = (mm.group(2) or "").strip()
                    if raw:
                        val = raw
                        try:
                            auto = int(raw, 0) + 1
                        except ValueError:
                            auto += 1
                    else:
                        val = str(auto); auto += 1
                    out.append((mm.group(1), val, "enum-member"))
                j += 1
            i = j
        i += 1
    return out


# --- Rule 5: no two lib/ modules in the COMPILE SET define one `fn` differently -
# Rule 4 above covers `var` and enum members, on the reasoning that the compiler is
# silent for those and *does* warn for a duplicate `fn`. It does warn -- and NOTHING
# GATES ON THE WARNING. A `fn` collision between two dependencies is therefore caught
# by a human reading build output, or not at all, which is not a gate.
#
# The shape mirrors Rule 4 deliberately: duplication is not the defect, DIVERGENCE is.
# kavach and sigil both vendor the same `agnosys` helper layer, so a dozen of these
# are byte-identical after normalisation and last-definition-wins is genuinely
# harmless. Those pass silently. A duplicate whose BODY differs is the real hazard --
# `_sub_new` is `(chan, filter_fn)` + `fl_alloc(40)` in majra and `(pattern)` +
# `alloc(24)` in libro, and libro's wins here.
#
# Bodies are compared with comments and whitespace normalised away, so reindentation
# upstream is not a false positive.

def _strip_for_braces(line):
    """Drop `#` comments and string literals so brace counting is not fooled."""
    out, q, i = [], None, 0
    while i < len(line):
        ch = line[i]
        if q:
            if ch == "\\":
                i += 2; continue
            if ch == q: q = None
        elif ch in '"\'':
            q = ch
        elif ch == '#':
            break
        else:
            out.append(ch)
        i += 1
    return "".join(out)

def fn_bodies(path):
    """name -> (normalised body, first line no) for every top-level `fn`."""
    out, lines, i = {}, open(path, errors="replace").read().split("\n"), 0
    while i < len(lines):
        m = re.match(r'^fn\s+([A-Za-z_]\w*)\s*\(', lines[i])
        if m:
            depth, buf, j, opened = 0, [], i, False
            while j < len(lines):
                buf.append(lines[j])
                clean = _strip_for_braces(lines[j])
                if "{" in clean: opened = True
                depth += clean.count("{") - clean.count("}")
                if opened and depth <= 0:
                    break
                j += 1
            txt = "\n".join(buf)
            txt = re.sub(r'#[^\n]*', '', txt)        # comments
            txt = re.sub(r'\s+', ' ', txt).strip()   # indentation / line breaks
            # Keep the FIRST definition in a file; a within-file duplicate is Rule 2's
            # business and would otherwise mask the cross-file one.
            out.setdefault(m.group(1), (txt, i + 1))
            i = j
        i += 1
    return out

def check_fn_collisions(files):
    allow = {}
    ap = "scripts/lib-fn-allow.txt"
    if os.path.exists(ap):
        for L in open(ap):
            L = L.strip()
            if not L or L.startswith("#"):
                continue
            nm, _, why = L.partition("#")
            allow[nm.strip()] = why.strip()

    seen = collections.defaultdict(list)
    for f in sorted(files):
        for nm, (body, ln) in fn_bodies(f).items():
            seen[nm].append((body, os.path.basename(f), ln))

    identical, diverge = 0, []
    for nm, e in seen.items():
        if len({f for _, f, _ in e}) > 1:
            if len({b for b, _, _ in e}) > 1:
                diverge.append((nm, e))
            else:
                identical += 1

    unflagged = [(n, e) for n, e in sorted(diverge) if n not in allow]
    flagged   = [(n, e) for n, e in sorted(diverge) if n in allow]

    print(f"lib-fn check: {len(seen)} fn names, {identical} identical duplicate(s), "
          f"{len(diverge)} divergent")
    for nm, e in flagged:
        print(f"  ALLOWED  {nm} — {allow[nm]}")
        for _, f, ln in e:
            print(f"             {f}:{ln}")
    if unflagged:
        print()
        print("error: a `fn` is defined by more than one dependency WITH A DIFFERENT BODY.")
        print("       Cyrius resolves this last-definition-wins. cycc warns, but no gate")
        print("       reads the warning, so this is the only thing that fails the build.")
        for nm, e in unflagged:
            print(f"  {nm}")
            for _, f, ln in e:
                print(f"      {f}:{ln}")
        print()
        print("       Fix it upstream in the owning library (prefix the name), then add it")
        print("       to scripts/lib-fn-allow.txt with the reason while the fix lands.")
        return 1
    return 0

def main():
    if not os.path.exists("cyrius.cyml"):
        print("lib-symbol check: no cyrius.cyml", file=sys.stderr); return 1
    stdlib, deps = parse_manifest()
    files = compile_set(stdlib, deps)
    if not files:
        print("lib-symbol check: empty compile set — is lib/ provisioned?", file=sys.stderr); return 1

    allow = {}
    ap = "scripts/lib-symbol-allow.txt"
    if os.path.exists(ap):
        for L in open(ap):
            L = L.strip()
            if not L or L.startswith("#"):
                continue
            nm, _, why = L.partition("#")
            allow[nm.strip()] = why.strip()

    seen = collections.defaultdict(list)
    for f in sorted(files):
        for nm, val, kind in constants(f):
            seen[nm].append((val, os.path.basename(f), kind))

    diverge, same = [], 0
    for nm, e in seen.items():
        if len({f for _, f, _ in e}) > 1:
            if len({v for v, _, _ in e}) > 1:
                diverge.append((nm, e))
            else:
                same += 1

    unflagged = [(n, e) for n, e in sorted(diverge) if n not in allow]
    flagged   = [(n, e) for n, e in sorted(diverge) if n in allow]

    print(f"lib-symbol check: {len(files)} modules in the compile set, "
          f"{len(seen)} constants, {same} benign duplicate(s)")
    for nm, e in flagged:
        print(f"  ALLOWED  {nm} — {allow[nm]}")
        for v, f, k in e:
            print(f"             {v:<24} {f} ({k})")
    if unflagged:
        print()
        print("error: a constant is defined more than once in the compile set WITH DIFFERENT VALUES.")
        print("       Cyrius resolves this silently (last definition wins) — no warning is emitted")
        print("       for `var` or for enum members, so nothing else in the build will catch it.")
        for nm, e in unflagged:
            print(f"  {nm}")
            for v, f, k in e:
                print(f"      {v:<24} {f} ({k})")
        print()
        print("       Fix it upstream in the owning library (prefix the name), then add it to")
        print("       scripts/lib-symbol-allow.txt with the issue reference while the fix lands.")
        return 1
    return check_fn_collisions(files)

sys.exit(main())
