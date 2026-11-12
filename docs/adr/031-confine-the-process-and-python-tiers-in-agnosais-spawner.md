# 031 — Confine the process and Python tiers in agnosai's spawner, with kavach's primitives

**Status**: Accepted
**Date**: 2026-11-07, updated at 2.3.6 for kavach 3.14.8 and later

## Context

The process, Python and OCI tiers spawned through `src/sandbox/spawn.cyr`. That gave them a SIGKILL deadline, a
controlled working directory and a sanitized environment, and nothing else: no seccomp, no landlock, no
NO_NEW_PRIVS. A tool on these tiers ran with the server's authority, so it could read the server's files and its
`/proc/<pid>/environ`, write anywhere the server could, and run a setuid binary. Since 2.3.0 a model can reach them
through a crew's tools, which made this the threat model's largest gap.

kavach's confined backend was bridged since M7 (`agnosai_kavach_execute`), and the roadmap said to route the three
tiers through it. It waited only on kavach's 1 MiB capture buffer per run, which 3.14.7 fixed. Routing them was then
measured, and `sandbox_exec` turned out to be the wrong door:

- **It refuses interpreters by name.** Without a rootfs, kavach's runtime guard blocks `python3`, `python`, `sh`,
  `bash`, `perl`, `node`, `ruby` and fourteen more as the program (`blocked command: sh`, exit 126, before any
  fork). That is the Python tier's whole job.
- **It keeps each run's result in process-wide variables.** The confined capture writes the payload's stderr into
  one 64 KiB static buffer as it arrives (`_confine_err_buf`), and the backends read the exit code, the timeout flag
  and the stderr back from globals once the capture returns (`confine_last_exit`, `confine_last_timed_out`,
  `confine_last_stderr`). agnosai runs tool calls on several threads at once. **Measured:** two threads, one running
  `/bin/ls` and one `/bin/cat` on missing paths, 100 calls each through `agnosai_kavach_execute`. In three runs, 97,
  112 and 102 of the 200 came back with the other program's stderr, or with the two spliced together. No exit code
  swapped, as that window is a load after a store. Filed with kavach, which keeps a run's results per run since 3.14.8.

The confinement itself is a separate, exported sequence, `confine_child_caps`, which kavach's own backends call in
the child. Its order is the security property, which is why kavach keeps it as one function: descriptors, cgroup,
NO_NEW_PRIVS, namespaces, rootfs, landlock, capabilities, and seccomp last.

## Decision

**agnosai keeps its own spawner, and the child calls kavach's `confine_child_caps` before it execs.** argv, stdin,
the environment, the work directory, the deadline and the two captured streams are agnosai's, exactly as before. The
confinement is kavach's, step for step. Nothing is shared between two calls: the child is a copy of the process, and
the parent's capture is per call.

### What a confined child gets

- **NO_NEW_PRIVS**, so a setuid or file-capability binary cannot raise the child's privileges.
- **kavach's exec seccomp filter** (`security_create_exec_seccomp_filter`): `mount`, `umount2`, `pivot_root`,
  `unshare`, `setns`, `ptrace`, `kexec_*`, module loading, `bpf`, `userfaultfd`, the new mount API and more are
  denied, as is a `clone` asking for namespaces. Built once, and loaded last.
- **landlock**, allow-list only. Read and execute on `/usr`, `/bin`, `/sbin`, `/lib`, `/lib32`, `/lib64`, `/libx32`,
  `/etc`, `/run/systemd/resolve` (the target of `/etc/resolv.conf` on systemd hosts), `/proc/self`, `/dev/urandom`,
  `/dev/random` and `/dev/zero`. Read and write on `/tmp`, `/dev/null`, `/dev/shm` and the call's work directory.
  Nothing else: not `/home`, `/root`, `/var`, `/srv`, `/opt` or `/proc/<pid>` for another process. A path the host does
  not have is left out, as kavach fails a rule it cannot open.
- **Every inherited descriptor closed**, and death with the server (`PR_SET_PDEATHSIG`).
- **The binary pinned by descriptor in the parent** (`kv_exec_pin`), and executed through it (`kv_exec_pinned`). The
  file that runs is the file that was checked, whatever replaces its path in between. That narrows the threat
  model's time-of-check gap on these tiers. A name that resolves to nothing executable is refused before any fork.

### Reporting: a marker, not an exit code

kavach's steps exit with codes 114 to 125, and a program may exit with any of those itself. So the child tells the
parent it reached its program. After `confine_child_caps` returns, it writes a marker word down agnosai's errno pipe,
and then execs. If the exec fails, it writes the failure after the marker.

- **A marker means the program ran.** Its exit code is its own: a tool exiting 124 is not "cannot apply landlock".
- **No marker means the child stopped inside kavach's sequence.** Its exit code is kavach's, and
  `spawn_exit_name` names the step. The tiers report `failed to confine <program>: cannot apply landlock`.
- **The pipe and the pin are lifted above fd 1024** in the child (`F_DUPFD_CLOEXEC`), as kavach closes every
  descriptor below `SPAWN_MAX_FD`. A child whose soft open-file limit is too low raises its own, after the fork. A
  hard limit at or below 1024 is a refusal, not an unconfined run.

### Defaults, and the opt-out

The process tier (`agnosai_process_config_default`, and so the manager's) and the Python tier
(`agnosai_python_sandbox_new`) are confined by default. `agnosai_process_config_set_confinement(c, 0)` and
`agnosai_python_sandbox_with_confinement(s, 0)` run unconfined, for a command the operator wrote and trusts. A tool
that needs one more tree gets `agnosai_confinement_with_path`. An operator gives every tool a tree with
`AGNOSAI_SANDBOX_READ_PATHS` or `AGNOSAI_SANDBOX_WRITE_PATHS`, each a `:`-separated list, read before any tool runs.

**It fails closed.** A host where kavach cannot build its seccomp filter refuses every confined run rather than run it
unconfined. A kernel with no landlock is the one exception, and it is kavach's rule: kavach installs no ruleset there,
so agnosai warns once at the first build, and the threat model says what that leaves.

### Network: not isolated on these tiers

**A tool on the process or Python tier keeps the server's network**, as it did before. Without a rootfs, isolating it
takes a network namespace, and unprivileged that needs a user namespace. Ubuntu 24.04 and later restrict those, docker's
default seccomp profile refuses `unshare`, and GitHub's Ubuntu runners refuse them too. Requiring one would stop every
process and Python tool on the most common hosts, where the cx tier, which does require one (ADR 006), already does
not run. kavach's score has given these tiers no network credit since 3.14.7 (`score_backend`), so nothing reports a
network isolation that is not applied.

Landlock ABI 4 (Linux 6.7) refuses TCP connect and bind with no namespace and no privilege, and since 3.14.8 kavach
applies that to a confined payload whose policy denies the network. agnosai passed `network_enabled = 1` through 2.3.7,
what it applied before. **Since 2.3.8 it denies the network by default** (the addendum below).

### The OCI tier: docker's own hardening

The OCI tier runs a container CLI, and the daemon builds the container, so confining the CLI process would confine the
wrong thing. The container is the confinement there, and its flags decide it. Since 2.3.4 the run line carries
`--cap-drop=ALL`, `--security-opt=no-new-privileges` and `--pids-limit 256`, after `--memory`. Measured through
docker: `CapEff` 0, `NoNewPrivs` 1, `pids.max` 256. The Rust line passed none of them.

### Capabilities, and the image runs as an unprivileged user

Through kavach 3.14.7, `confine_child_caps` bounded a root child's capabilities only inside a rootfs, which these tiers
do not use. Since 3.14.8 it bounds every confined child, and agnosai passes an empty mask (`AGNOSAI_SPAWN_CAPS_NONE`,
2.3.6), not `KAVACH_CAPS_DEFAULT`, the twelve a container's init needs. A tool under a root server holds no capabilities.
Measured in a container as uid 0: `CapEff`, `CapPrm`, `CapBnd` and `CapAmb` all read zero (`tests/sandbox_confine.tcyr`).
Root without `CAP_SETPCAP` cannot be bounded, so kavach refuses the run: every confined tool fails with `cannot bound
capabilities`. Measured with `--cap-drop=SETPCAP`.

The `Dockerfile` runs the server as `agnosai`, uid 10001, all the same. It needs nothing root-only: it listens on 8080
and writes only `/tmp`. Measured: the image builds, answers `/ready`, and `id` reads `uid=10001(agnosai)`.

## Consequences

- **Positive.** A tool on these tiers cannot read the server's files or another process's environment, cannot write
  outside `/tmp` and its work directory, cannot raise its privileges, and cannot mount, unshare or ptrace. Measured from
  inside the child in `tests/sandbox_confine.tcyr`.
- **Positive.** Python runs confined. kavach's own exec path refuses it by name.
- **Positive.** Confinement adds nothing per call, in memory or in time. The policy and the filter are built once, and a
  work directory's policy once per directory. `benches/sandbox.bcyr`: `/bin/true` 10.310 ms unconfined and 10.320 ms
  confined, a Python tool call 41.27 and 41.24 ms. Both were the spawner's 5 ms idle tick. 2.3.5 waits in `ppoll`
  instead, and confinement's own cost shows: `/bin/true` 0.51 ms unconfined and 1.12 ms confined in the A/B, the 14 landlock paths
  opened in the child and the filter loaded.
- **Positive, with it.** The process and Python tiers take the call's allocator (`agnosai_process_execute_argv_a`,
  `agnosai_python_execute_tool_a`), and the Python and WASM tools read it from their input, as 2.3.2's built-ins do. A
  Python tool call in the call's arena keeps 0 B, where it kept 11,032 B, and a process call 0 B, where it kept 13,992
  B. The sanitized environment and the bridge script are built once. Asserted in `tests/sandbox_confine.tcyr`.
- **Negative, a behaviour change.** A tool that read or wrote outside the rules now gets EACCES. That includes `$HOME`,
  the user site-packages in `~/.local`, `/var`, `/opt`, and another process's `/proc`. The operator's paths are how to
  give one back.
- **Negative, through 2.3.7.** Network is not isolated on these tiers, as above. 2.3.8 closes it where the kernel has
  landlock ABI 4 (the addendum below).
- **Negative.** `/proc/self` is the confined process's own `/proc/<pid>`, as the rule is opened in the child, so a
  process the tool starts reads its own `/proc` as EACCES. A tool that runs `sh -c "a; b"` or forks a helper that
  reads `/proc/self` sees it fail. Allowing all of `/proc` would give every tool the server's
  `/proc/<pid>/environ`, keys included, so it is not offered. Found in the CI replica, where dash forks a lone
  command that bash on the development box execs.
- **Positive, since 2.3.6.** A root server's tools hold no capabilities, and an exec cannot take one back.
- **Negative, a behaviour change at 2.3.6.** A root server with `CAP_SETPCAP` dropped can run no confined tool. It ran
  them through 2.3.5, with root's capabilities. Give it `CAP_SETPCAP`, or run it as a user.
- **Neutral.** The cx tier is unchanged: it already runs under kavach's persistent confinement, with a network
  namespace it requires. WASM is unchanged: kavach runs wasmtime, and its guest has stdin and stdout alone.
- **Closed by kavach 3.14.8.** Two WASM calls at once read each other's stderr inside kavach, as measured above, which
  could misclassify a module "refused: it exceeds the store limits" either way. kavach keeps a run's result per run now.
  Measured again at 3.14.9, the same two threads and 200 runs, three times: none of the 600 read another run's stderr.

## Alternatives considered

- **`sandbox_exec`, as the roadmap first said.** Refuses `python3` and `sh`, and shares each run's stderr between
  threads. Measured above.
- **A lock around `sandbox_exec`.** It would serialize every sandboxed tool in the process behind the slowest, a 30 s
  Python tool holding up every other, and it leaves the interpreter refusal.
- **A rootfs per tier.** It lifts kavach's blocklist and bounds capabilities, and needs an image of the interpreter and
  its libraries per tier, which agnosai does not have and an operator would have to build.
- **Naming the confinement step from the exit code alone, as kavach's own callers do.** A tool exiting 124 would read
  as a landlock failure. The marker costs one 8-byte write.
- **Requiring a network namespace.** Fails every process and Python tool on Ubuntu 24.04 and later, in docker, and on
  GitHub's runners, as above.
- **Confining the docker CLI.** The daemon builds the container, so the CLI's confinement says nothing about the tool's.

## Addendum (2.3.8): a tool denied the network loses it

**The process and Python tiers deny the network unless their config allows it**, as the OCI tier's `--network=none` always
has. `agnosai_process_config_set_network(c, 1)` and `agnosai_python_sandbox_with_network(s, 1)` give it back, and anything
but 1 is a no. The manager's process arm passes the policy's `needs_network`, which is 0 for every policy that reaches it,
as a policy needing the network resolves to the OCI tier.

### How

A confinement holds two kavach policies, with the network and without it, over the same landlock rules, and a work
directory's pair is made together and kept, so a call picks one and builds nothing. The spawner's
`agnosai_spawn_capture_confined_net_a` takes the flag. Its older forms keep the network, as every confined child did, so
a consumer calling the spawner directly is unchanged. kavach's `confine_child_caps` does the rest (3.14.8,
`_spawn_net_alone`): landlock's TCP connect and bind rights, and a seccomp filter for what landlock does not hold. Measured
from inside the child on Linux 7.2.14 (landlock ABI 10), all EACCES:

| What the child does | Allowed | Denied |
|---|---|---|
| TCP connect to a loopback listener (bash `/dev/tcp`, Python) | connects | `Permission denied` |
| TCP bind (Python, port 0) | binds | `PermissionError` |
| A UDP socket (bash `/dev/udp`, Python `SOCK_DGRAM`) | opens | `Permission denied` |
| A unix socket (Python `AF_UNIX`) | opens | opens: the host's own, by kavach's design |

### A kernel before landlock ABI 4

**There the child keeps the network**, whatever its config says: kavach applies neither half below ABI 4, and the only
other way to hold it is a network namespace, rejected above. `agnosai_confinement_denies_network(c)` says which a host is,
and the default confinement warns once when it is built. `tests/sandbox_confine.tcyr` measures both kernels on one box:
a child of the suite hides landlock behind a seccomp filter that answers `landlock_create_ruleset` with ENOSYS, builds the
default afresh, and finds the warning, `denies_network` 0, and a denied child that connects. Seccomp can only refuse, so
that is ABI 0 rather than 1 to 3. The network is the same on all four.

### Consequences

- **Negative, a behaviour change.** A process or Python tool that used the network fails with EACCES: an HTTP client, a
  package install, a DNS lookup. **To migrate:** allow it on that tool's config, or give the tool a policy with
  `needs_network`, which runs it in a container. The CHANGELOG says so.
- **Positive.** A tool's command or Python cannot reach the network from these tiers: not a cloud metadata address, not a
  service on the server's loopback, not an exfiltration endpoint. That was the threat model's largest open row for them.
- **Negative, small.** One more kavach policy per confinement and per work directory, built once, and a call allocates
  nothing more (asserted, 0 B). The child loads one more seccomp filter and landlock's network rights: about 57 µs a
  spawn, `spawn_true_confined_no_network` 1.297 ms against `spawn_true_confined` 1.240 ms in one binary. A Python tool
  call, at about 36 ms, does not show it.
- **Neutral.** kavach's score gives the process backend no network credit (`score_backend`), so a crew profile's
  `sandbox_strength` does not move.

