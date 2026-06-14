# FM02 — Forme Plugin Host: Manifest, Sandboxing, Wire Protocol, Capability Mediation

> **Status:** Code-ready specification. Read alongside FM00 (vision),
> FM01 (kernel), and FM03 (orchestrator).
> **Scope:** Everything required to load third-party Forme plugins
> safely and run them under a strong isolation boundary. The packages
> `forme-manifest`, `forme-plugin-host`, `forme-plugin-runner-ts`,
> and the per-OS sandbox modules `forme-sandbox-linux`,
> `forme-sandbox-macos`, `forme-sandbox-windows`.
> **Out of scope:** The orchestrator runtime itself (FM03), the
> kernel types every plugin speaks (FM01), the IRs plugins
> contribute to (FM04 Style IR, FM05 Interactivity IR), the AOT
> compiler (FM06), the dev server and CLI (FM07).

---

## 0. Preface

FM01 specified the contract every stage implements. FM03 specified
the runtime that wires stages into a pipeline. Both presumed that
stages were available as plain TypeScript values — loaded via direct
import, trusted by virtue of being in the dependency tree the user
pinned in their config.

FM02 is what changes when stages come from **someone else** — a
plugin published to npm by a developer the user has never met. The
plugin's bytes arrived on the user's machine because the user typed
its name into a config file. The user has not audited the source,
will not audit the source, and should not have to audit the source.
That is the entire problem this document solves.

The mental model: **a plugin is a piece of untrusted code that may
do anything its author wants, including malicious things**, and the
host's job is to give it just enough access to do its declared job
and nothing else. The boundary is enforced by the operating system,
not by the language. A Node `vm.Context` is not a security boundary.
A Worker thread is not a security boundary. Only a separate process
with restricted syscalls is a security boundary, and that is what
FM02 specifies.

### 0.1 Relationship to other FM specs

- **FM00** sets the vision — pluggable authoring surface, third-party
  plugins, themes-as-plugins, capability-gated APIs.
- **FM01** defines `Stage<In, Out>`, `StageContext`, `Capability`,
  `LogicalId`, `RevisionId`, the error model, and the manifest
  schema *outline*. FM02 fills in the manifest schema concretely
  and specifies how the runtime APIs FM01 names get realised across
  a process boundary.
- **FM03** consumes a `PluginHost` interface to load stages
  (`loadStage`), check capabilities (`validateCapability`), and
  build per-stage contexts (`buildContext`). FM02 supplies the
  third-party implementation of that interface — the
  `SubprocessPluginHost` — alongside the existing first-party
  `DefaultDirectImportHost`.
- **FM04, FM05, FM06, FM07** consume plugin-host services
  transparently. None of them need to know whether a stage is
  in-process or sandboxed.

### 0.2 What this spec pins down

1. **The threat model.** What we protect against, what we
   explicitly don't, and where the trust boundary sits.
2. **The plugin manifest (`plugin.toml`).** Concrete schema, JSON
   Schema validator, signature scheme stub.
3. **Discovery and loading.** Where plugins live on disk; how
   `StageRef`s resolve.
4. **The sandbox architecture.** One subprocess per plugin
   instance, plus OS-level sandboxing on top.
5. **The wire protocol.** JSON-RPC 2.0 with Content-Length framing
   (LSP-style) over stdio. Every host-mediated API has a documented
   method, every error has a code, every notification has a shape.
6. **Capability mediation.** Every capability-gated API in
   `StageContext` becomes a host-mediated RPC. The plugin's
   subprocess never makes a syscall outside of stdio.
7. **Per-OS sandboxing.** seccomp-bpf + namespaces on Linux;
   `sandbox-exec` profiles on macOS; Job Objects + AppContainer on
   Windows. The OS layer is defence in depth — the wire protocol is
   the primary boundary.
8. **Lifecycle.** Spawn, handshake, init, run, dispose, kill.
9. **Streaming.** How `Stream<K>` and `AsyncIterable<K>` cross the
   process boundary with backpressure.
10. **Resource limits.** Memory, CPU, wall-clock, file-descriptor
    caps; both rlimit-style and orchestrator-side enforcement.
11. **Plugin SDKs.** The per-language libraries that hide the wire
    protocol behind a clean `defineStage`-shaped API. Reference
    TypeScript SDK; sketch for Python and Rust.
12. **Package layout.** Six new packages; their dependencies; their
    BUILD ordering.
13. **Testing contract.** The fault-injection matrix every
    implementation must pass.

### 0.3 Compatibility promise

The wire protocol is versioned by an `apiVersion` integer that
matches FM01's `KERNEL_API_VERSION`. A plugin built against
`apiVersion: 1` runs against any host that supports `apiVersion: 1`.
Breaking changes bump the version; the host loads only plugins
whose declared `apiVersion` falls inside its supported set.

`plugin.toml` is versioned by a separate `manifestVersion` field
(initial value `1`), so the manifest schema can evolve independently
of the wire protocol when changes are confined to discovery/loading.

---

## 1. Terminology

Reuses FM01's terminology. Adds:

- **Plugin** — a unit of distribution that contributes one or more
  stages, kinds, or extensions. A plugin is a directory containing
  a `plugin.toml`, an entry-point binary or script, and optional
  resources.
- **Plugin host** — the runtime that discovers plugins, parses
  manifests, spawns sandboxed subprocesses, and mediates capabilities.
- **Plugin process** — the OS process the host spawns to execute a
  plugin's code. One process per plugin *instance* (a plugin used
  twice in a pipeline has two processes).
- **Plugin runner** — the per-language library that runs *inside*
  the plugin process, handles the wire protocol, marshals values
  to/from the user's stage code.
- **Host API** — a method the plugin can call across the wire to
  request a capability-gated operation (e.g. read a file). Each one
  maps to a wire method on the host side and a function on the
  plugin SDK side.
- **Sandbox** — the OS-level isolation layer wrapping the plugin
  process. Defence in depth; not a substitute for the wire-protocol
  boundary.
- **Wire protocol** — the JSON-RPC 2.0 framed message stream the
  host and plugin exchange over stdio.
- **Trust tier** — a level of trust the user has assigned to a
  plugin: first-party (in-process), verified third-party (sandboxed,
  signature-gated), unverified third-party (sandboxed, user-grant
  per capability).

---

## 2. The Threat Model

### 2.1 What we defend against

A plugin's author is treated as **potentially malicious**. We
defend against:

1. **Data exfiltration.** The plugin reads files it was not granted
   access to (the user's home directory, credentials, secrets) and
   tries to ship the bytes off the machine.
2. **Network exfiltration.** The plugin opens sockets to attacker-
   controlled hosts to exfiltrate data the plugin legitimately
   processed.
3. **Code execution beyond the plugin.** The plugin tries to spawn
   shell commands, modify other plugins, write to system paths, or
   load native libraries the host did not authorise.
4. **Environment leaks.** The plugin reads `process.env` to grab
   `AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, or similar.
5. **Cache poisoning.** The plugin produces non-deterministic
   outputs that depend on hidden state (time, random, ambient
   filesystem), tricking the orchestrator's cache into reusing
   bad results.
6. **Resource exhaustion.** The plugin allocates unbounded memory,
   spins in a tight loop, opens unlimited file descriptors, or
   forks subprocesses without bound.
7. **Cross-plugin interference.** Plugin A reads or writes data
   belonging to Plugin B; or one plugin's failure cascades into
   another's process.
8. **Tampering with the host or other stages' outputs.** The plugin
   modifies the orchestrator's cache files, the host's
   `node_modules`, the pipeline config, or another stage's
   intermediate output.
9. **Privilege escalation.** The plugin tries to use a granted
   capability to escalate to a denied one (e.g. given
   `filesystem:read` to one directory, try to read another via
   path traversal or symlink follow).
10. **Supply-chain attacks via transitive plugins.** A plugin tries
    to load other plugins or `require()` packages outside its own
    `node_modules`.

### 2.2 What we explicitly do not defend against

1. **Side-channel attacks.** Cache-timing, Spectre/Meltdown,
   electromagnetic emanations. Out of scope; the OS and CPU
   handle those.
2. **Malicious host operator.** If the user runs Forme with a
   tampered orchestrator, FM02 cannot help. The host is the trust
   anchor.
3. **A correctly-functioning plugin's policy choices.** If a plugin
   requests `network:s3.amazonaws.com` and the user grants it, the
   plugin can ship anything it processes to S3. The mitigation is
   the user reading the capability grants at install time, not
   FM02 inferring intent.
4. **Filesystem races outside the plugin's grant.** A user who
   places a symlink in a granted directory pointing outside it is
   trusting the symlink target by transitivity. FM02 resolves
   symlinks before checking the path against the grant; it does
   not pre-scan for "suspicious" symlinks.
5. **DoS via legal-but-slow operations.** A plugin that requests
   100,000 small file reads is wasteful but not malicious. Resource
   limits cap the total work, but slow honest work is not
   distinguishable from slow malicious work.
6. **Reverse-engineering of the user's input data.** A plugin
   legitimately processing a document can derive anything from
   that document. If the document contains a secret the user
   didn't want the plugin to see, the wrong threat surface was
   chosen.

### 2.3 The trust boundary

The trust boundary is the plugin process. **Everything inside the
plugin process is hostile.** Everything outside the plugin process
is the host's responsibility.

In particular:

- The plugin process has no direct access to filesystem, network,
  environment, system clock (beyond `CLOCK_MONOTONIC` for
  performance measurement), process spawning, native library
  loading, or any other syscall that touches state outside its own
  address space.
- The plugin process **does** have CPU and memory inside its
  rlimits, stdio (the wire), and a few harmless syscalls
  (`brk`/`mmap` for heap, `clock_gettime(MONOTONIC)`, etc.).
- Every other capability is exercised by sending an RPC over the
  wire. The host evaluates the request against the plugin's
  declared and granted capabilities and either performs the
  operation on the plugin's behalf or denies it.

### 2.4 Trust tiers

A plugin operates at one of three tiers, set at install time by
the user (with defaults):

| Tier | Sandbox | Capability grants | Use case |
|---|---|---|---|
| **first-party** | none (in-process) | all declared | bundled Forme stages (`@coding-adventures/forme-*`) |
| **verified third-party** | full subprocess sandbox | declared, after one-time user grant | plugins signed by a key in the user's trust store |
| **unverified third-party** | full subprocess sandbox | per-capability prompt at install | everything else |

The CLI surfaces this prominently. The default for `forme install`
is **unverified third-party** — the user reviews every capability
the plugin requests before installation completes. There is no
flag to skip the review; bypassing it requires editing the trust
store manually.

---

## 3. The Plugin Manifest (`plugin.toml`)

### 3.1 Location

Every plugin is a directory containing a `plugin.toml` at the
root. The directory is the plugin's *root*; the manifest is
discovered relative to it.

```
my-plugin/
├── plugin.toml
├── entry.js              ← or entry.py, entry-bin, etc.
├── README.md
└── (other files)
```

### 3.2 Schema

```toml
# Required: schema version of this manifest format itself.
manifestVersion = 1

# Required: machine-readable identity.
[plugin]
name        = "@forme/parse-markdown"     # globally unique; namespace recommended
version     = "1.4.2"                      # semver
apiVersion  = 1                            # FM01 KERNEL_API_VERSION targeted
description = "Parse CommonMark + GFM into a ContentNode"
license     = "MIT"
authors     = ["Alice <alice@example.com>"]
homepage    = "https://example.com/parse-markdown"
repository  = "https://github.com/alice/parse-markdown"

# Required: how the host should spawn this plugin.
[runtime]
kind        = "node"                       # "node" | "deno" | "bun" | "python" | "binary"
entry       = "./entry.js"                 # path relative to plugin root
# For "binary": platform-specific entries.
# [runtime.platforms]
# linux-x86_64   = "./bin/plugin-linux-x86_64"
# linux-aarch64  = "./bin/plugin-linux-aarch64"
# darwin-x86_64  = "./bin/plugin-darwin-x86_64"
# darwin-aarch64 = "./bin/plugin-darwin-aarch64"
# windows-x86_64 = "./bin/plugin-windows-x86_64.exe"

# Required: the capabilities this plugin REQUIRES to function.
# Missing capabilities here cause the host to deny installation
# (the plugin can't even start without these).
[[capabilities.required]]
realm   = "filesystem"
scope   = "read"
detail  = "$storageRoot"                   # template — resolved at runtime
reason  = "Read source files to parse"     # shown to user at install

# Capabilities the plugin can use but doesn't require. Missing optional
# capabilities cause the plugin to run with reduced functionality (e.g.
# slower fallback paths).
[[capabilities.optional]]
realm  = "system"
scope  = "time"
detail = "wallclock"
reason = "Use file mtime for cache invalidation; falls back to content hash without it."

# Required: what this plugin contributes.
[[contributes.stages]]
id         = "parse-markdown"              # local id, qualified by plugin.name
consumes   = "ContentSource"               # KindName
produces   = "ContentNode"
configSchema = "./schemas/config.json"     # JSON Schema for stage config; optional

[[contributes.kinds]]
name       = "ext:youtube-embed"
version    = "1.0"
schema     = "./schemas/youtube-embed.json"
subtypeOf  = "ContentNode"

# Resource limits the plugin requests. The host enforces these as
# upper bounds; the user can lower them in their config but not raise.
[resources]
maxMemoryMb       = 512
maxWallClockMs    = 30000
maxFileDescriptors = 256
maxConcurrentRpcs  = 64

# Optional: signature. Verified third-party plugins SHOULD ship one.
[signature]
algorithm = "ed25519"
publicKey = "MCowBQYDK2VwAyEA..."          # base64 SPKI
signature = "MEUCIQD..."                   # base64 signature over canonical manifest + entry hash
signedAt  = "2026-05-16T12:00:00Z"
```

### 3.3 Validation rules

A `plugin.toml` is valid when:

1. `manifestVersion` is a positive integer the host supports
   (initially `1`).
2. `plugin.name` matches `^(@[a-z0-9][a-z0-9-]*\/)?[a-z0-9][a-z0-9-]*$`.
3. `plugin.version` parses as semver.
4. `plugin.apiVersion` equals one of the host's supported
   `KERNEL_API_VERSION` values.
5. `runtime.kind` is one of the recognised values; the matching
   runtime is installed; the entry path resolves to a file that
   exists.
6. For `runtime.kind == "binary"`, the current platform has a
   matching entry under `runtime.platforms`.
7. Every `capabilities.required` and `capabilities.optional`
   entry parses as a valid `Capability` (FM01 §5; format
   `realm:scope` or `realm:scope:detail`).
8. No entry under `capabilities.required` is in
   `FIRST_PARTY_ONLY` (per `forme-capability` — `system:shell`,
   `system:time-nondeterministic`). A third-party plugin
   requesting one of these fails validation outright. The CLI
   refuses to install the plugin even with a user override flag.
9. Every `contributes.stages` entry has unique `id`, references
   recognised kinds (or kinds the same plugin contributes), and
   the optional `configSchema` parses as JSON Schema draft-07.
10. Every `contributes.kinds` entry has a name beginning with
    `ext:`; kernel kind names are reserved.
11. `resources.*` values are positive integers within the host's
    hard ceiling (host configurable, e.g. memory ≤ 16 GiB).
12. If a `[signature]` section is present, the signature verifies
    over `(canonical_toml(everything except [signature]) ||
    entryFileHash)`.

A `ManifestError` carries the failing field path, the rule
violated, and remediation text.

### 3.4 `$variables` in capability detail

Capability `detail` strings may contain a small set of templated
variables that the host resolves at plugin-load time, before the
capability is recorded:

- `$storageRoot` — resolves to the absolute path of the pipeline's
  `settings.storageRoot`.
- `$cacheDir` — resolves to `settings.cacheDir`, if set.
- `$pluginDir` — resolves to the plugin's installation directory.

Any unrecognised `$variable` causes a manifest validation error.
Plain `$` characters that should not be templated must be escaped
as `$$`.

The templating is intentionally minimal — it exists so a plugin's
declared filesystem grant can scope to the user's project root
without the plugin knowing that path. **It is not a general
template engine**; expressions, conditionals, and arbitrary
substitution are explicitly out of scope.

### 3.5 Manifest hash and stage identity

The manifest contributes to the cache key of every stage the
plugin provides. The hash is computed over the *canonical TOML*
representation of the manifest minus the `[signature]` section,
concatenated with BLAKE2b-256 of the entry file (or, for `binary`
runtime, the per-platform entry file actually used).

This hash is recorded into each stage's `cacheKey` derivation
(FM03 §5.2). The orchestrator's cache invalidates automatically
when a plugin updates, with no per-stage version bumping needed.

---

## 4. Discovery & Loading

### 4.1 Where plugins live

The host searches these locations in order; the first match wins:

1. **Project plugins** — `<project>/forme-plugins/<name>/plugin.toml`.
2. **User plugins** — `~/.forme/plugins/<name>/plugin.toml`.
3. **System plugins** (Linux/macOS) — `/usr/local/lib/forme/plugins/...`.
   Windows equivalent: `%ProgramData%\Forme\plugins\...`.
4. **Bundled first-party** — within the Forme installation itself
   (typically `<forme>/lib/plugins/`).

The search order is fixed: project beats user beats system beats
bundled. A user can override a system plugin by dropping a same-
named replacement into the project tree, which is the intended
escape hatch.

A `pluginPath` field in `PipelineSettings` lets the user add
additional roots ahead of the defaults — useful for CI and
hermetic-build scenarios.

### 4.2 The `forme install` flow (informative)

The CLI (FM07) provides `forme install <package>` which:

1. Fetches the plugin distribution (npm tarball, OCI artifact,
   tarball URL — registry abstraction TBD in FM02.1).
2. Verifies the archive's integrity hash matches what the
   registry advertised.
3. Unpacks into `<project>/forme-plugins/<name>/`.
4. Parses `plugin.toml`; validates per §3.3.
5. If `[signature]` present and a public key is in the user's
   trust store, verifies and assigns trust tier "verified".
6. Otherwise, assigns trust tier "unverified".
7. **Renders an install prompt** listing every required and
   optional capability with its `reason` text and any
   `SENSITIVE` flag from `forme-capability`. The user must
   accept-or-deny per capability. Defaults: required → ask
   (no default), optional → deny.
8. Records the grants in `<project>/forme-plugins/<name>/grants.toml`.
   Subsequent runs read this file; no re-prompting unless the
   plugin's manifest changes.

`forme install` is outside FM02's package surface (it lives in the
CLI, FM07), but the trust-store and grants-file formats are FM02's
responsibility and are specified in this document.

### 4.3 Trust store

User-wide trust store: `~/.forme/trust.toml`.

```toml
[[trustedKeys]]
algorithm = "ed25519"
publicKey = "MCowBQYDK2VwAyEA..."
addedAt   = "2026-05-16T12:00:00Z"
note      = "Alice's signing key — from her keybase"
```

Plugins signed by any key in the trust store load at the
"verified" tier, which still requires per-capability grant the
first time but skips it on subsequent installs by the same
publisher.

### 4.4 Grants file

Per-plugin grants: `<project>/forme-plugins/<name>/grants.toml`.

```toml
manifestHash = "blake2b:abcdef..."         # hash at time of grant; mismatch → re-prompt

[[granted]]
capability = "filesystem:read:/abs/path/to/storageRoot"
grantedAt  = "2026-05-16T12:00:00Z"
note       = "$storageRoot resolved to this path at install time"

[[granted]]
capability = "system:time:wallclock"
grantedAt  = "2026-05-16T12:00:00Z"
```

If `manifestHash` no longer matches the plugin's current manifest
(the publisher updated the plugin), the host re-prompts for any
new or changed capability and updates the grants file.

### 4.5 Resolving `StageRef`

A `StageRef` (FM03 §2.1) carries `{ packageName, export? }`.
Resolution:

1. Locate the plugin directory whose `plugin.toml` declares
   `plugin.name == packageName`.
2. Look up the stage by id in `contributes.stages`; if `export`
   is supplied, that's the id; otherwise default to the plugin's
   `name` minus the namespace (e.g. `@forme/parse-markdown` →
   `parse-markdown`).
3. Construct a `Stage<KindDescriptor, KindDescriptor>` *proxy*
   that, when invoked, forwards the call across the wire to the
   plugin's subprocess. The proxy is a real `Stage` value — the
   orchestrator never knows it's talking to another process.

The proxy carries the manifest's declared `consumes`/`produces`/
`capabilities`/`configSchema`/`apiVersion`. These come from the
manifest, not the plugin process, so the orchestrator can run
its full typecheck (FM03 §3.4) without ever launching the
plugin.

---

## 5. The Sandbox Architecture

### 5.1 Process-per-instance

Each `StageInstance` in a pipeline becomes one plugin process.

If the same plugin appears twice (two `StageInstance`s of the
same `Stage<>`), there are two plugin processes. They share
nothing — no module-level state, no caches, no sockets. This is
explicit, costs CPU/memory, and is correct: stages are pure
(FM01 §3.3), so per-instance state was never legitimate anyway.

A future optimisation may pool processes per stage (one warm
process serving multiple invocations sequentially), but the
correctness model is "one process per instance" and pooling must
be transparent.

### 5.2 The two-layer boundary

```
┌──────────────────────────────────────────────────────────────┐
│                       Host process                            │
│  (orchestrator, plugin-host, all kernel state)                │
└────────────────┬─────────────────────────────────────────────┘
                 │ stdio (wire protocol)
                 │
                 │   ┌─────────────────────────────────────┐
                 │   │     OS sandbox boundary              │
                 │   │   (seccomp / sandbox-exec /          │
                 └───┤    Job Object) — defence in depth    │
                     │                                      │
                     │  ┌────────────────────────────────┐  │
                     │  │   Plugin process                │  │
                     │  │   - user's stage code           │  │
                     │  │   - per-language plugin runner  │  │
                     │  │   - bounded heap, no syscalls   │  │
                     │  │     beyond stdio + mmap         │  │
                     │  └────────────────────────────────┘  │
                     └──────────────────────────────────────┘
```

Two boundaries:

- **The wire boundary.** The plugin can only do anything observable
  by sending JSON-RPC requests over stdout. Every capability-gated
  operation is the host's responsibility. Even if the OS sandbox
  is misconfigured, the plugin still has no way to do anything
  except via the wire.
- **The OS sandbox.** Defence in depth. If the plugin's runtime
  (Node, Python, etc.) has a bug that lets the plugin call a
  syscall, the sandbox blocks it. The sandbox is configured
  to deny everything except the syscalls needed to read/write
  stdio, allocate memory, and read the monotonic clock.

The wire boundary is the primary trust boundary. The OS sandbox
is its safety net.

### 5.3 What the plugin process gets

At spawn time, the host gives the plugin:

- **stdin** — host → plugin RPC stream (Content-Length framed JSON).
- **stdout** — plugin → host RPC stream (Content-Length framed JSON).
- **stderr** — uncaptured by the protocol; routed to the host's
  logger as `level: warn, source: plugin-stderr`. Plugins are
  encouraged to use the wire protocol's `log` notification
  instead; stderr is the fallback for runtime crashes the
  plugin runner couldn't intercept.
- **No environment variables.** The host launches the plugin with
  a scrubbed environment. A minimal `PATH` is provided only when
  the plugin's runtime needs it (e.g. Python needs `PATH` to find
  `libpython` on Linux); even then, it's set to a known-safe value
  pointing only at the plugin runtime's own bin directory.
- **No filesystem cwd.** The process's working directory is set to
  a fresh empty tempdir which is cleaned up on process exit. The
  plugin cannot infer the user's project layout from `process.cwd()`.
- **No arguments containing user data.** `argv[1..]` carries only
  a small protocol-version handshake token.

### 5.4 What the plugin process does NOT get

- **No `process.env` with user values.** The scrubbed env has
  only the bare minimum the runtime needs (`PATH`, `HOME` set
  to a tempdir, locale).
- **No network.** Outbound network is blocked at the sandbox
  layer (`setrlimit(RLIMIT_NOFILE)` is not enough — see §12 for
  per-OS network blocking).
- **No filesystem outside the cwd tempdir.** All reads of the
  pipeline's content go through the host via `ctx.storage`.
- **No syscalls beyond a tight allow-list.** seccomp-bpf on
  Linux, sandbox-exec on macOS, AppContainer + Job Object on
  Windows. See §12 for the exact allow-lists.
- **No native module loading**, except those the plugin runtime
  embeds itself (Node's built-ins, Python's stdlib). User code
  in the plugin may not `require('child_process')` or
  `import socket`.
- **No subprocesses.** `fork`, `execve`, `posix_spawn`, etc. are
  blocked.

### 5.5 Plugin runtimes

Supported runtimes for v0:

- **`node`** — Node 20+ launched with `--no-addons --frozen-intrinsics`
  and a sandbox loader that blocks `child_process`, `fs`, `net`,
  `worker_threads`, etc., from inside the JavaScript layer. The
  OS sandbox blocks them again at the syscall layer.
- **`deno`** — Deno run with `--no-permission` (denies all
  built-in capabilities) plus the OS sandbox. Deno's permission
  system is convenient but not the trust boundary.
- **`python`** — CPython 3.10+ launched without `PYTHONPATH`,
  with `sys.path` restricted to the plugin's own directory and
  the stdlib. The runner blocks `os.system`, `subprocess`,
  `socket`, etc., in the Python layer; sandbox blocks at syscalls.
- **`binary`** — a pre-compiled executable the plugin ships per
  platform. The host runs it under the OS sandbox; no language-
  level mitigations apply, but the sandbox is the same.

The choice of runtime is the plugin author's; the host launches
whichever the manifest names, provided it is installed and on the
host's allow-list. Hosts MAY refuse runtimes they don't trust
(e.g. a host configured to only allow `node` and `binary`).

### 5.6 Why not Wasm

WebAssembly is an attractive sandbox — fast startup, deterministic,
in-process — but its host-call surface for Forme would have to be
identical to the subprocess wire protocol (since plugins want to do
network I/O via host-mediated capabilities), and the security
properties on top would be no stronger than what the host enforces
in those host calls.

A WebAssembly-backed runner is a perfectly reasonable thing to add
in a future spec (FM02.1 or FM02.2). The wire protocol is designed
so a WebAssembly runner is a drop-in replacement for the subprocess
runner — the orchestrator-facing `PluginHost` interface doesn't
change. v0 ships subprocess only because that's the strongest
boundary that is universally available across OSes and runtimes
today.

---

## 6. The Wire Protocol

### 6.1 Framing

JSON-RPC 2.0 messages, each prefixed with a Content-Length header,
identical to the Language Server Protocol's framing:

```
Content-Length: 142\r\n
\r\n
{"jsonrpc":"2.0","method":"stage.run","id":1,"params":{...}}
```

- One message per frame; frames concatenated on the same stream.
- `Content-Length` is the byte count of the JSON payload, UTF-8 encoded.
- Headers terminated by `\r\n\r\n`.
- Payloads are valid JSON-RPC 2.0 messages: `{ jsonrpc: "2.0", ... }`.
- Other headers are ignored on receipt; only `Content-Length` is
  mandatory. (LSP also defines `Content-Type` but we don't use it.)

### 6.2 Message kinds

Three kinds, per JSON-RPC 2.0:

- **Request** — `{ jsonrpc, id, method, params? }`. Expects a
  response.
- **Response** — `{ jsonrpc, id, result }` (success) or
  `{ jsonrpc, id, error: { code, message, data? } }` (failure).
- **Notification** — `{ jsonrpc, method, params? }`. No `id`; no
  response.

`id` is a number or string. The originator chooses; the receiver
echoes it on response. The host uses positive integers
monotonically; the plugin uses negative integers monotonically.
This prevents id collisions on a bidirectional stream.

### 6.3 Conversation phases

A plugin process's wire conversation passes through six phases:

```
   ┌─────────┐  ┌──────────┐  ┌──────┐  ┌─────┐  ┌─────────┐  ┌──────┐
   │handshake│→ │announce  │→ │ init │→ │ run │→ │ dispose │→ │ exit │
   └─────────┘  └──────────┘  └──────┘  └─────┘  └─────────┘  └──────┘
```

Each phase is gated by a host-issued request; the plugin advances
only when the host says so. The plugin may abort at any point by
exiting; the host detects this via EOF on the plugin's stdout.

### 6.4 Phase 1 — handshake

The host's first message is a `handshake` request:

```jsonc
// Host → Plugin
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "handshake",
  "params": {
    "hostName":          "forme-orchestrator",
    "hostVersion":       "0.1.1",
    "apiVersion":        1,                    // FM01 KERNEL_API_VERSION
    "protocolVersion":   1,                    // FM02 wire protocol version
    "pluginName":        "@forme/parse-markdown",
    "pluginVersion":     "1.4.2",
    "manifestHash":      "blake2b:...",
    "instanceId":        "parse-markdown",
    "trustTier":         "unverified-third-party"
  }
}
```

The plugin must respond within `handshakeTimeoutMs` (default
5000):

```jsonc
// Plugin → Host
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "pluginName":       "@forme/parse-markdown",
    "pluginVersion":    "1.4.2",
    "apiVersion":       1,
    "protocolVersion":  1,
    "runner":           "@coding-adventures/forme-plugin-runner-ts",
    "runnerVersion":    "0.1.0"
  }
}
```

The host validates that the plugin's `pluginName`,
`pluginVersion`, `apiVersion`, and `protocolVersion` match the
manifest. Any mismatch → kill the process, fail the run.

### 6.5 Phase 2 — announce

The host requests the plugin's stage manifest as it sees it at
runtime, for parity with the static manifest:

```jsonc
// Host → Plugin
{ "jsonrpc": "2.0", "id": 2, "method": "announce", "params": {} }

// Plugin → Host
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "stage": {
      "name":          "@forme/parse-markdown",
      "version":       "1.4.2",
      "apiVersion":    1,
      "description":   "Parses CommonMark + GFM into a ContentNode.",
      "consumes":      { "name": "ContentSource", "version": "1.0" },
      "produces":      { "name": "ContentNode",   "version": "1.0" },
      "capabilities":  ["filesystem:read:$storageRoot"],
      "configSchemaHash": "blake2b:..."
    }
  }
}
```

The host verifies announced shapes match the manifest. Mismatch =
kill + fail. This guards against a plugin shipping a manifest that
says one thing and code that does another.

### 6.6 Phase 3 — init

Once announced, the host invokes `stage.init` exactly once:

```jsonc
// Host → Plugin
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "stage.init",
  "params": {
    "config": { ... },                       // validated against configSchema
    "instanceId":  "parse-markdown",
    "storageRoot": "/abs/path/to/storage",
    "logLevel":    "info"
  }
}

// Plugin → Host (success)
{ "jsonrpc": "2.0", "id": 3, "result": null }
```

A plugin's `init` is the place to validate config beyond what the
schema covers (cross-field invariants, etc.) and to prepare any
in-memory state. Init may NOT make capability-gated calls — those
are reserved for `run`. (Init is invoked before any input is
known; capabilities scoped to inputs don't make sense yet.)

### 6.7 Phase 4 — run

The host invokes `stage.run` with the input value and a stream
id:

```jsonc
// Host → Plugin
{
  "jsonrpc": "2.0",
  "id": 100,
  "method": "stage.run",
  "params": {
    "input":     { /* KindPayload<In> */ },
    "config":    { /* same as init */ },
    "streamId":  500                            // even if not streaming, used to tag any sub-RPCs
  }
}
```

#### 6.7.1 Single-output stages

A stage that produces a single value sends back:

```jsonc
// Plugin → Host
{
  "jsonrpc": "2.0",
  "id": 100,
  "result": {
    "kind":  "single",
    "value": { /* KindPayload<Out> */ }
  }
}
```

#### 6.7.2 Streaming-output stages

A stage that produces `Stream<K>` emits multiple values as
notifications, then responds once the stream completes:

```jsonc
// Plugin → Host (notifications, no id)
{ "jsonrpc": "2.0", "method": "stream.value", "params": { "streamId": 500, "value": {...} } }
{ "jsonrpc": "2.0", "method": "stream.value", "params": { "streamId": 500, "value": {...} } }
{ "jsonrpc": "2.0", "method": "stream.value", "params": { "streamId": 500, "value": {...} } }

// Plugin → Host (the request response — signals stream end)
{
  "jsonrpc": "2.0",
  "id": 100,
  "result": { "kind": "stream", "streamId": 500, "produced": 3 }
}
```

The host correlates `stream.value` notifications by `streamId` to
the active `stage.run` request. When the response to the
`stage.run` request arrives, the host knows the stream is
complete and the count it has buffered should match `produced`.

#### 6.7.3 Streaming-input stages

For a stage that consumes `Stream<K>`, the host sends the input
stream as a separate set of notifications interleaved with the
request:

```jsonc
// Host → Plugin
{ "jsonrpc": "2.0", "id": 100, "method": "stage.run",
  "params": { "input": { "kind": "stream-handle", "streamId": 600 },
              "config": {...}, "streamId": 500 } }

{ "jsonrpc": "2.0", "method": "stream.value", "params": { "streamId": 600, "value": {...} } }
{ "jsonrpc": "2.0", "method": "stream.value", "params": { "streamId": 600, "value": {...} } }
{ "jsonrpc": "2.0", "method": "stream.end",   "params": { "streamId": 600 } }
```

The plugin's runner exposes `input` as an `AsyncIterable<K>` that
yields as the wire delivers `stream.value` notifications.

Hybrid stages (Stream-in, single-out) and pure stream-stream
stages compose these two patterns.

#### 6.7.4 Backpressure

stdio's underlying pipes provide coarse-grained backpressure:
when one side stops reading, the OS pipe buffer (~64KB on Linux,
~8KB on Windows) fills and the writer's `write()` blocks. This
gives correct (if coarse) flow control with no application-level
logic.

For finer control, a future revision may add a credit-based
flow-control mechanism (`stream.grant` notifications). For v0
the OS pipe is sufficient.

### 6.8 Phase 5 — dispose

After every `run` invocation, or on cancellation, the host calls
`stage.dispose`:

```jsonc
// Host → Plugin
{ "jsonrpc": "2.0", "id": 999, "method": "stage.dispose", "params": {} }

// Plugin → Host
{ "jsonrpc": "2.0", "id": 999, "result": null }
```

The plugin's `dispose` hook (if any) runs; resources are
released. After dispose, the plugin SHOULD exit cleanly within
`disposeGracePeriodMs` (default 2000). If it doesn't, the host
sends SIGTERM; if it still doesn't exit within
`killGracePeriodMs` (default 2000), the host sends SIGKILL.

### 6.9 Phase 6 — exit

The plugin process exits, the OS reaps it, the host records the
exit code. Non-zero exit codes are logged but not propagated as
errors *unless* the plugin exited before completing a request
(e.g. crashed mid-run). In that case the host synthesises a
`StageError { code: "PLUGIN_CRASHED", ... }` for the corresponding
`stage.run`.

### 6.10 Cancellation

Cancellation flows host → plugin as a notification:

```jsonc
{ "jsonrpc": "2.0", "method": "$/cancelRequest", "params": { "id": 100 } }
```

The runner translates this into a `CancellationToken.cancel()`
on the stage's context. The plugin's `run` method is expected to
check the token at safe points and throw a `CancellationError`,
which the runner translates into:

```jsonc
{
  "jsonrpc": "2.0",
  "id": 100,
  "error": { "code": -32800, "message": "Request cancelled", "data": {} }
}
```

If the plugin doesn't acknowledge cancellation within
`cancellationGracePeriodMs` (default 5000), the host escalates
to dispose + SIGTERM + SIGKILL.

---

## 7. Capability Mediation (Host APIs)

Every capability-gated API in `StageContext` (FM01 §4) maps to a
wire method the plugin calls on the host. The host evaluates the
plugin's grants and either performs the operation or returns a
`CAPABILITY_DENIED` error.

### 7.1 The general shape

```jsonc
// Plugin → Host
{
  "jsonrpc": "2.0",
  "id": -42,
  "method": "ctx.storage.readFile",
  "params": { "path": "posts/hello.md" }
}

// Host → Plugin (granted)
{
  "jsonrpc": "2.0",
  "id": -42,
  "result": {
    "bytes":    "SGVsbG8gd29ybGQ=",      // base64
    "mimeType": "text/markdown"
  }
}

// Host → Plugin (denied)
{
  "jsonrpc": "2.0",
  "id": -42,
  "error": {
    "code":    -32001,
    "message": "CAPABILITY_DENIED",
    "data": {
      "capability": "filesystem:read:/abs/path/to/posts/hello.md",
      "reason":     "capability is not in plugin's granted set"
    }
  }
}
```

### 7.2 Storage API

`ctx.storage` exposes content under the pipeline's `storageRoot`.
Plugins requesting `filesystem:read:$storageRoot` get:

- `ctx.storage.readFile(path: string) → Promise<{ bytes, mimeType }>`
  → wire `ctx.storage.readFile`
- `ctx.storage.statFile(path: string) → Promise<StorageStat>`
  → wire `ctx.storage.statFile`
- `ctx.storage.listDir(path: string) → Promise<readonly string[]>`
  → wire `ctx.storage.listDir`
- `ctx.storage.watch(path: string) → AsyncIterable<StorageChange>`
  → wire `ctx.storage.watch` (returns a streamId, plugin reads
  via `stream.value` notifications)

All paths are resolved relative to `storageRoot` on the host
side. Path arguments containing `..` or absolute paths are
rejected with `INVALID_PATH` (-32002). Symlinks resolved on the
host side; if a resolved path falls outside `storageRoot`, the
operation is rejected with `CAPABILITY_DENIED`.

`ctx.storage.writeFile` and `ctx.storage.removeFile` exist for
emit-shaped stages with `filesystem:write` capability. They go
through identical mediation.

### 7.3 Network API

`ctx.network` is gated by `network:<host>` capabilities (FM01
§4.8.2 host-hierarchy semantics).

- `ctx.network.fetch(url, init?) → Promise<Response>`
  → wire `ctx.network.fetch`

The host parses the URL, checks the request's hostname against
the plugin's grants (using the FM01 §4.8.2 dotted-suffix rule),
performs the fetch, and returns the response. Response body is
streamed back via `stream.value` notifications for large
responses, or inline as base64 for small ones (threshold:
1 MiB).

DNS resolution happens on the host side; the plugin never
contacts a name server.

WebSocket and other long-lived connection types are NOT in v0.
A future revision may add them with explicit per-message
mediation.

### 7.4 Environment API

`ctx.env` is gated by `env:<name>` capabilities. The plugin asks
for specific env vars by name; the host returns the value (or
null) if granted. There is no "list all env vars" API — the
plugin must know what it wants.

- `ctx.env.get(name) → Promise<string | null>`
  → wire `ctx.env.get`

### 7.5 Time API

`ctx.time` provides wall-clock and monotonic time. Wall-clock
requires `system:time:wallclock`; monotonic is always available
(needed for `setTimeout`, performance measurement — denying it
would prevent the runtime from functioning).

- `ctx.time.nowMs() → number` (monotonic, in-process, no RPC)
- `ctx.time.wallclockMs() → Promise<number>` → wire `ctx.time.wallclockMs`
- `ctx.time.nowIso() → Promise<string>` → wire `ctx.time.nowIso`

In FM03 §8 reproducible-build mode, the host returns a frozen
wall-clock value to all plugins for the duration of the run.

### 7.6 Random API

`ctx.random` provides cryptographic randomness, gated by
`system:random`. (`Math.random()`-style PRNG is available
in-process; only crypto-grade entropy needs mediation.)

- `ctx.random.bytes(n) → Promise<Uint8Array>` → wire `ctx.random.bytes`
- `ctx.random.deterministic(name) → number` → in-process,
  seeded from the stage's cache key. Always available; ensures
  reproducible builds remain reproducible.

### 7.7 Logger API

`ctx.logger` is NOT capability-gated. Logging is a fundamental
observability primitive; denying it would make plugins impossible
to debug. Every level (trace/debug/info/warn/error) is a wire
notification:

```jsonc
{
  "jsonrpc": "2.0",
  "method": "log",
  "params": {
    "level":   "info",
    "message": "parsed 42 documents",
    "fields":  { "count": 42 }
  }
}
```

The host's logger routes these through the same sink as kernel
logs, tagged with `source: plugin, plugin: <name>, instance: <id>`.

Plugins MUST NOT log secrets or user data they processed. This
is a policy the user trusts the author to follow; FM02 cannot
enforce it without inspecting every log message, which would
itself be a privacy concern.

### 7.8 Cache API

Plugins do not have direct cache access. The orchestrator caches
plugin outputs on its side; plugins are invoked as if every call
were fresh. If a plugin needs internal memoization across
invocations within a single run, it does so in-process (which is
permitted since the host-mediated cache key invalidates the
process on input change).

Cross-run caching beyond the orchestrator's automatic cache is
not provided.

---

## 8. Lifecycle & Cancellation

(Already covered in §6's phases; this section consolidates the
state machine.)

### 8.1 State machine

```
              ┌─────────────┐
              │   spawned   │
              └──────┬──────┘
                     │ handshake ok
                     ▼
              ┌─────────────┐
              │  announced  │
              └──────┬──────┘
                     │ stage.init returns
                     ▼
       ┌──→  ┌─────────────┐
       │     │    ready    │ ←──┐
       │     └──────┬──────┘    │
       │            │ stage.run │ stage.run returns
       │            ▼           │
       │     ┌─────────────┐    │
       │     │   running   │────┘
       │     └──────┬──────┘
       │            │ stage.dispose returns
       │            ▼
       │     ┌─────────────┐
       │     │  disposing  │
       │     └──────┬──────┘
       │            │ process exits or kill timer
       │            ▼
       │     ┌─────────────┐
       │     │    exited   │
       │     └─────────────┘
       │
       │ on error in any state above
       │     ┌─────────────┐
       └───→ │   faulted   │ → killed
             └─────────────┘
```

Transitions out of `running` and into `faulted`:

- Plugin crash (process exits with non-zero before responding):
  faulted, error code `PLUGIN_CRASHED`.
- Wire protocol violation (malformed frame, wrong jsonrpc version,
  unknown method): faulted, `PROTOCOL_VIOLATION`.
- Resource limit hit (rlimit, sandbox kill): faulted,
  `RESOURCE_LIMIT_EXCEEDED`.
- Cancellation acknowledged: returns to `ready`.
- Timeout on cancellation: faulted, escalates to dispose+kill.

### 8.2 Cleanup invariants

Regardless of fault, every spawned plugin process must:

1. Have its stdin/stdout/stderr file descriptors closed by the
   host.
2. Have any pending RPC requests synthesised into rejected
   promises on the orchestrator side (so awaiting code unblocks).
3. Have its tempdir cwd removed.
4. Be reaped (no zombie processes).

The host's `dispose()` method walks every plugin instance and
ensures these invariants hold within `killGracePeriodMs * 2`.

---

## 9. Streaming & Backpressure

(Wire details in §6.7; this section is the conceptual story.)

### 9.1 Bidirectional streams

A pipeline naturally streams in both directions for a per-item
stage between two stream stages. The wire protocol handles this
with two `streamId`s — one for the input stream, one for the
output stream. They flow concurrently; the runner inside the
plugin process iterates the input as the host pushes values
and yields output values as the user's `run` function emits them.

### 9.2 Buffering bounds

The host buffers at most `streamBufferSize` (default 64) values
per stream direction. When the buffer fills, the host stops
reading from the plugin's stdout, which (via OS pipe buffer
saturation) eventually blocks the plugin's writes. This is the
backpressure mechanism.

### 9.3 Cancellation during streaming

If the host cancels mid-stream, it sends `$/cancelRequest` for
the active `stage.run`. The runner cancels the user's iterator;
the user's code (which should be checking the cancellation token)
unwinds; the runner sends back the cancellation error response.
Buffered stream values are discarded on both sides.

---

## 10. Error & Cancellation Across the Boundary

### 10.1 Error codes

| Code | Constant | Meaning |
|---|---|---|
| -32700 | `PARSE_ERROR` | Plugin sent malformed JSON |
| -32600 | `INVALID_REQUEST` | Plugin's RPC violates JSON-RPC 2.0 |
| -32601 | `METHOD_NOT_FOUND` | Host doesn't recognise the method |
| -32602 | `INVALID_PARAMS` | Wrong shape for method params |
| -32603 | `INTERNAL_ERROR` | Generic host-side error |
| -32001 | `CAPABILITY_DENIED` | Capability not in plugin's grants |
| -32002 | `INVALID_PATH` | Path arg out of allowed scope or malformed |
| -32003 | `RESOURCE_LIMIT_EXCEEDED` | Plugin hit memory/cpu/fd cap |
| -32004 | `PROTOCOL_VIOLATION` | Plugin broke a protocol invariant |
| -32005 | `PLUGIN_CRASHED` | Plugin process exited unexpectedly |
| -32006 | `MANIFEST_MISMATCH` | Runtime announcement differs from manifest |
| -32800 | `CANCELLED` | Request was cancelled by the host |
| -32900 | `STAGE_ERROR` | Wraps a FM01 `StageError` thrown by the stage |

`STAGE_ERROR` is the common one: when the user's `run` function
throws a `StageError`, the runner serialises it into the `data`
field:

```jsonc
{
  "code":    -32900,
  "message": "Parse error on line 3",
  "data": {
    "stageErrorCode": "PARSE_ERROR",
    "inputPath":      "posts/bad.md",
    "inputId":        "01952c...",
    "stageName":      "@forme/parse-markdown",
    "recoverable":    false,
    "fields":         { "line": 3 }
  }
}
```

The host reconstructs the `StageError` on its side and surfaces
it through the normal FM01 §6 error path.

### 10.2 Stack traces

Stack traces are NOT crossed by default — they can leak file paths
inside the plugin's package. A debug-mode flag in the host's
`OrchestratorOptions` enables stack-trace forwarding for plugin
development; otherwise stage errors carry only the `code` +
`message` + `fields`.

### 10.3 Cancellation semantics

A cancelled `stage.run` returns a `CANCELLED` error. The
orchestrator translates that into a `CancellationError` (FM01 §6)
exactly as if the cancellation had happened in-process.
Downstream code can't tell the difference.

---

## 11. Resource Limits

### 11.1 What is limited

| Resource | Default cap | Enforcement |
|---|---|---|
| Resident memory | 512 MiB | rlimit on POSIX, Job Object on Windows |
| Virtual memory | 2 GiB | rlimit on POSIX, Job Object on Windows |
| CPU seconds | 60 | rlimit/cgroup on Linux, Job Object on Windows |
| Wall-clock per run | 30 s | orchestrator-side timer |
| File descriptors | 256 | rlimit on POSIX, Process Mitigations on Windows |
| Concurrent in-flight RPCs | 64 | host-side wire layer |
| stdout/stderr bytes/sec | 10 MiB/s | host-side wire layer |

The plugin's manifest can request higher caps via `[resources]`
(§3.2). The host enforces a hard ceiling regardless (configurable
in host policy, e.g. memory ≤ 16 GiB).

### 11.2 What happens on exceedance

- **rlimit-style exceedance** (memory, CPU) — the OS kills the
  plugin process. The host detects the exit code and reports
  `RESOURCE_LIMIT_EXCEEDED`.
- **wall-clock exceedance** — the host sends `$/cancelRequest`,
  waits the grace period, then escalates to dispose + SIGTERM +
  SIGKILL.
- **fd exceedance** — the plugin's runtime sees `EMFILE` from
  syscalls. The runner translates that to a `StageError { code:
  "RESOURCE_LIMIT_EXCEEDED" }`.
- **In-flight RPC exceedance** — the host stops servicing new
  requests until the plugin has at most `maxConcurrentRpcs - 1`
  outstanding. Backpressure, not error.
- **stdout/stderr rate exceedance** — the host throttles its
  reading, which (via pipe backpressure) throttles the plugin.

---

## 12. OS-Specific Sandboxing

Defence in depth. The wire protocol is the trust boundary; OS
sandboxing is the safety net. Every implementation MUST install
the OS sandbox before the plugin's user code begins executing.

### 12.1 Linux

- **seccomp-bpf** filter installed before `execve` of the plugin
  runtime. Allow-list approach: only the syscalls the runtime
  needs are permitted. Everything else returns `EPERM`. The
  default policy permits: `read`, `write`, `close`, `fstat`,
  `lseek`, `mmap`/`munmap`/`mprotect`/`brk`, `rt_sigaction`,
  `rt_sigprocmask`, `rt_sigreturn`, `nanosleep`/`clock_nanosleep`,
  `clock_gettime`, `getpid`, `gettid`, `tgkill`, `exit`,
  `exit_group`, `arch_prctl`, `set_tid_address`, `futex`, plus
  a small set of runtime-specific extras (e.g. Node needs
  `epoll_*`, `eventfd2`, `pipe2`).
- **User namespace** (`CLONE_NEWUSER`) so the plugin process
  appears as root inside its own namespace and as a non-
  privileged user outside; combined with **mount namespace**
  (`CLONE_NEWNS`) to give the plugin a private rootfs.
- **Network namespace** (`CLONE_NEWNET`) with no interfaces,
  defeating any DNS/socket attempts at the kernel level.
- **PID namespace** so the plugin can't see other host processes.
- **`no_new_privs`** to prevent setuid escape paths.
- **cgroups v2** for memory and CPU limits.

This is bubblewrap / nsjail-style isolation. The implementation
SHOULD use `libseccomp` and the `unshare` syscall directly rather
than shell out to `bwrap`, but a bubblewrap-backed implementation
is acceptable as a v0 fallback.

### 12.2 macOS

- **`sandbox-exec`** profile applied via `sandbox_init` in the
  plugin process at startup. The profile denies all filesystem
  access except the cwd tempdir, denies all network, denies
  process spawning, and denies Mach IPC except for what the
  language runtime needs.
- Example profile sketch:
  ```scheme
  (version 1)
  (deny default)
  (allow process-fork process-exec)   ;; for runtime startup; revoked after
  (allow file-read-data
    (subpath (param "runtime-bin"))
    (literal "/usr/lib/dyld"))
  (allow file-write* (subpath (param "tempdir-cwd")))
  (allow mach-lookup (global-name "com.apple.system.opendirectoryd.libinfo"))
  ```
- **`taskgated`** is not used; we rely on the in-process profile
  installed during the runner's startup.
- Resource limits via `setrlimit` (POSIX-compatible).

### 12.3 Windows

- **Job Object** containing the plugin process, configured with:
  - `JOB_OBJECT_LIMIT_PROCESS_MEMORY`
  - `JOB_OBJECT_LIMIT_JOB_MEMORY`
  - `JOB_OBJECT_LIMIT_PROCESS_TIME`
  - `JOB_OBJECT_LIMIT_ACTIVE_PROCESS = 1` (no child processes)
  - `JOB_OBJECT_LIMIT_BREAKAWAY_OK = 0`
  - `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
- **Restricted token**: plugin process runs as a restricted user
  with `SECURITY_MANDATORY_LOW_RID` integrity level.
- **AppContainer** (Windows 8+) with no capabilities granted —
  blocks network and most filesystem access at the kernel.
- **Process Mitigations**: ASLR, DEP, CFG, no remote images, no
  dynamic code.
- File-handle limits via Job Object.

### 12.4 If the OS doesn't support a feature

The host MUST refuse to load third-party plugins on a host
without working sandboxing. A clear error explains: "FM02
requires OS-level sandboxing; this OS/version doesn't support
it. First-party plugins (in-process) are unaffected."

Future revisions may relax this (e.g. allow opt-in unsandboxed
plugins for trusted dev environments) but **never silently**.

---

## 13. Plugin SDKs

The wire protocol is what a plugin and host actually exchange,
but plugin authors don't write JSON-RPC by hand. Each supported
language ships an SDK that hides the wire behind a clean API
matching FM01's `defineStage` shape.

### 13.1 TypeScript SDK — `@coding-adventures/forme-plugin-runner-ts`

A plugin in TypeScript looks identical to an in-process stage:

```typescript
// my-plugin/entry.ts
import { runPlugin } from "@coding-adventures/forme-plugin-runner-ts";
import { Kinds, defineStage } from "@coding-adventures/forme-types";

const stage = defineStage({
  name:        "@me/my-plugin",
  version:     "1.0.0",
  apiVersion:  1,
  description: "...",
  consumes:    Kinds.ContentSource,
  produces:    Kinds.ContentNode,
  capabilities: ["filesystem:read:$storageRoot"],
  configSchema: null,
  async run(source, _config, ctx) {
    // ctx.storage, ctx.network, ctx.logger, etc. all work —
    // the runner wires them to the host via RPC transparently.
    const bytes = await ctx.storage.readFile(source.path);
    // ... do work ...
    return result;
  },
});

await runPlugin(stage);
```

`runPlugin`:
- Reads `argv[1]` for the handshake token.
- Reads Content-Length-framed JSON from stdin.
- Writes Content-Length-framed JSON to stdout.
- Constructs a `StageContext` whose APIs send RPCs.
- Translates the user's `run` return value (single, Promise,
  AsyncIterable) into the appropriate response shape.
- Translates user-thrown `StageError`s into wire errors.
- Hooks `SIGINT`/`SIGTERM` to dispose cleanly.

### 13.2 Python SDK — `forme-plugin-runner-py`

Similar shape, idiomatic Python:

```python
from forme_plugin_runner import run_plugin, define_stage

@define_stage(
    name="@me/my-plugin",
    version="1.0.0",
    api_version=1,
    consumes="ContentSource",
    produces="ContentNode",
    capabilities=["filesystem:read:$storageRoot"],
)
async def my_stage(source, config, ctx):
    bytes = await ctx.storage.read_file(source["path"])
    return { "kind": "ContentNode", ... }

if __name__ == "__main__":
    run_plugin(my_stage)
```

### 13.3 Rust SDK — `forme-plugin-runner-rs`

For `binary` runtime plugins:

```rust
use forme_plugin_runner::{run_plugin, Stage, StageContext, Kinds};

struct MyStage;
impl Stage for MyStage {
    const NAME: &str = "@me/my-plugin";
    const VERSION: &str = "1.0.0";
    const API_VERSION: u32 = 1;
    const CONSUMES: &str = "ContentSource";
    const PRODUCES: &str = "ContentNode";

    async fn run(&self, input: ContentSource, _config: Config, ctx: &StageContext) -> Result<ContentNode, StageError> {
        let bytes = ctx.storage().read_file(&input.path).await?;
        // ...
    }
}

#[tokio::main]
async fn main() {
    run_plugin(MyStage).await;
}
```

### 13.4 Conformance suite

Every SDK must pass the conformance suite in `forme-plugin-runner-conformance/`:

- Handshake/announce/init/run/dispose lifecycle correctness.
- Single, streaming, and hybrid I/O shapes.
- All capability APIs proxy correctly.
- Cancellation honoured within the grace period.
- StageError serialisation round-trips.
- Resource-limit exceedance produces the expected wire error.

The conformance suite is a test harness that spawns the SDK
under test as a subprocess and drives it through every scenario.
A new SDK is "ready" when it passes the suite.

---

## 14. Package Layout

Six new packages under `code/packages/typescript/` (plus per-OS
sandbox modules that may be native add-ons or shell-outs to
existing tools like `bwrap` / `sandbox-exec`).

### 14.1 `@coding-adventures/forme-manifest`

Pure types and parser for `plugin.toml`. Depends on
`forme-capability` and `forme-errors` only.

- `src/manifest-types.ts` — every interface from §3
- `src/parse-toml.ts` — `parseManifest(text: string): Manifest`
- `src/validate.ts` — per §3.3
- `src/canonical.ts` — canonical TOML serialiser (for signing/hashing)
- `src/manifest-hash.ts` — `computeManifestHash(manifest, entryBytes)`
- `src/signature.ts` — Ed25519 sign + verify
- `src/templating.ts` — `$variable` resolution (§3.4)
- `src/errors.ts` — `ManifestError`

### 14.2 `@coding-adventures/forme-plugin-host`

The host runtime: discovers, loads, spawns, mediates. Implements
the `PluginHost` interface FM03 §12 declared.

- `src/host.ts` — `createPluginHost(opts): PluginHost`
- `src/discovery.ts` — `discoverPlugins(roots): Manifest[]`
- `src/load-stage.ts` — `loadStage(ref): Stage<...>` (returns the proxy)
- `src/spawn.ts` — process spawning with sandbox application
- `src/wire.ts` — Content-Length framing + JSON-RPC plumbing
- `src/capability-mediator.ts` — handlers for every `ctx.*` method
- `src/build-context.ts` — builds the wire-backed `StageContext`
- `src/grants.ts` — read/write `grants.toml`
- `src/trust-store.ts` — read `~/.forme/trust.toml`
- `src/resources.ts` — rlimit/Job-Object setup
- `src/lifecycle.ts` — state machine, kill timers
- `src/types.ts` — public API

### 14.3 `@coding-adventures/forme-plugin-runner-ts`

The TypeScript-side SDK. The library a plugin author imports.

- `src/run-plugin.ts` — entry point (`runPlugin(stage)`)
- `src/wire.ts` — mirror of host's wire layer, plugin-side
- `src/build-context.ts` — wire-backed `StageContext` for the
  plugin's `run` to consume
- `src/error-translate.ts` — `StageError` ↔ wire error
- `src/stream-bridge.ts` — `AsyncIterable` ↔ stream notifications
- `src/index.ts`

### 14.4 `@coding-adventures/forme-sandbox-linux`

Linux-only sandbox primitives.

- `src/seccomp.ts` — generates seccomp-bpf programs
- `src/namespaces.ts` — `unshare` wrapper
- `src/cgroups.ts` — cgroup v2 setup
- Native addon (Rust + N-API) for the syscalls Node can't make
  directly.

### 14.5 `@coding-adventures/forme-sandbox-macos`

macOS-only sandbox primitives.

- `src/sandbox-exec.ts` — generates `sandbox_init` profiles
- `src/rlimits.ts` — `setrlimit` wrapper

### 14.6 `@coding-adventures/forme-sandbox-windows`

Windows-only sandbox primitives.

- `src/job-object.ts` — Job Object creation and assignment
- `src/appcontainer.ts` — AppContainer setup
- `src/restricted-token.ts` — token creation
- Native addon for Win32 APIs not exposed in Node.

### 14.7 Dependency graph

```
forme-types ◄── forme-errors ◄── forme-capability ◄── forme-manifest
                                                              │
                                              ┌──────────────┴──┐
                                              │                 │
                              forme-sandbox-* (per OS)   forme-plugin-host
                                                                 │
                                              forme-plugin-runner-ts
                                                                 │
                                              (depends on forme-stage,
                                               forme-types — same as any
                                               in-process stage author)
```

### 14.8 BUILD ordering

Leaf-to-root, per `lessons.md` convention:

```
forme-types → forme-errors → forme-capability → forme-manifest
                                              → forme-sandbox-linux
                                              → forme-sandbox-macos
                                              → forme-sandbox-windows
                                              → forme-plugin-host
                                              → forme-plugin-runner-ts
```

---

## 15. Testing Contract

### 15.1 `forme-manifest`

- Round-trip: `parseManifest(canonicalToml(m)) === m`.
- Every validation rule has a test case (both pass and fail).
- Signature: a manifest signed with key X verifies under X and
  fails under Y; tampering with any field breaks verification.
- Hash: changing any field changes the manifest hash; changing
  the entry file bytes changes it.
- `$variable` resolution: every recognised variable, plus
  rejection of unknown ones, plus `$$` escape.

### 15.2 `forme-plugin-host`

- Discovery: plugins found in project before user before system.
- Loading: `StageRef` resolves to a working proxy; missing
  plugin errors clearly.
- Sandbox refusal: third-party load fails when sandboxing is
  unavailable.
- Wire protocol: every method handler tested for happy path
  and every error code.
- Capability mediation: denied capabilities produce
  `CAPABILITY_DENIED`; granted ones execute.
- Path escape: any `..` or absolute path in `ctx.storage` calls
  rejected.
- Symlink escape: symlinks resolving outside `storageRoot` rejected.
- Resource limits: a deliberately-allocating fixture plugin
  triggers `RESOURCE_LIMIT_EXCEEDED`.
- Crashes: a fixture plugin that exits mid-run yields
  `PLUGIN_CRASHED`.
- Cancellation: a fixture that ignores cancel gets killed
  within the grace period.

### 15.3 `forme-plugin-runner-ts`

- Pass the conformance suite (§13.4).
- Idle plugin doesn't busy-wait (CPU ≈ 0 between requests).
- Memory of a no-op plugin stays bounded across 10,000
  request cycles.

### 15.4 `forme-sandbox-*` (per OS)

- The sandbox blocks an unauthorised filesystem read (a fixture
  process tries `open("/etc/passwd")`; syscall fails).
- The sandbox blocks an unauthorised network connect.
- The sandbox blocks process spawn (`fork`, `execve`, `posix_spawn`).
- The sandbox enforces memory limit (a fixture allocates 1 GiB
  with a 256 MiB cap; process killed).
- The sandbox enforces fd limit.

### 15.5 Integration tests

A fixture plugin published as `code/packages/typescript/forme-fixture-plugin/`
that does:

- Reads a file via `ctx.storage`.
- Fetches a localhost URL via `ctx.network`.
- Logs through `ctx.logger`.
- Honours cancellation.
- Throws a `StageError` on certain inputs.

The host loads it, runs it through the orchestrator, asserts
end-to-end behaviour. This becomes the smallest possible
end-to-end FM02 demo, analogous to `forme-hello-world` for
FM03.

### 15.6 Coverage target

≥ 95% line and branch across `forme-manifest` and
`forme-plugin-host`. ≥ 90% for the per-OS sandbox modules
(some paths are unreachable without exhausting kernel limits).

---

## 16. Performance Characteristics (Informative)

Subprocess sandboxing is more expensive than in-process loading.
The targets are deliberately modest:

- **Spawn cost** — < 100 ms for a Node-runtime plugin from
  spawn to first handshake response. Hot caches help; cold
  start may be 200–400 ms.
- **RPC round-trip** — < 1 ms for a simple capability call
  (e.g. `ctx.time.wallclockMs`) on localhost.
- **Throughput** — > 10,000 single-output `stage.run` calls per
  second on a modern laptop, sustained.
- **Memory overhead** — < 50 MiB per plugin process baseline
  (driven mostly by the runtime, Node ≈ 30 MiB cold).

These numbers are *not* a contract; they are realistic order-
of-magnitude expectations. A plugin author whose plugin is
spawn-bound should batch invocations in their stage rather than
treat the host as the bottleneck.

A future revision (FM02.1) may add **process pooling** — one
warm plugin process serving multiple instances sequentially —
which amortises spawn cost. The wire protocol is designed to
accommodate this transparently; no plugin-side changes needed.

---

## 17. Open Questions

1. **Plugin registry.** Where do third-party plugins live for
   `forme install` to fetch them? Reuse npm? A Forme-specific
   registry? Sigstore + OCI artifacts? Deferred to a separate
   spec (FM02.1).
2. **Cross-plugin RPC.** Today plugins can only talk to the
   host. A future "plugin extension points" model might let
   plugin A consume a service exported by plugin B (e.g. a
   syntax-highlight theme provider). Out of scope for v0;
   the wire protocol's namespacing leaves room.
3. **Sandbox in containers.** What if the host is already
   running inside a container with seccomp filters applied?
   Nested user namespaces may not work. Document the policy:
   plugins inherit container limits, and the host's sandbox
   is best-effort.
4. **Wasm runner.** Should land as FM02.1 once a Wasm-side
   implementation of the wire protocol exists. The host-facing
   interface doesn't change.
5. **Binary protocol.** JSON encoding overhead for large
   `ContentSource.bytes` is real (base64 = +33%). A future
   revision may add MessagePack framing as an opt-in for plugins
   that handle large blobs.
6. **Hot reload during watch mode.** When the user edits a
   plugin's source during `forme watch`, does the host
   re-spawn? In v0: no — plugin updates require a full
   `forme run` restart. A future revision may add SIGHUP-style
   reload.
7. **Untrusted plugin debugging.** Plugin authors want stack
   traces and stdout logs; the host's policy denies them by
   default for security. A `--debug-plugin <name>` flag may
   relax this. Specify exactly.
8. **Multi-tenant hosts.** If a host serves multiple users
   (Forme as a service), per-user isolation needs more than
   per-plugin isolation. Out of scope for v0; FM00 §2.1 says
   Forme is not a multi-tenant platform.
9. **Plugin update flow.** When `forme install` updates a
   plugin, the orchestrator's cache must invalidate. The
   manifest-hash field in `cacheKey` handles this; verify
   end-to-end.
10. **Verifying signature trust.** Where do the user's trusted
    keys come from? Hardcoded org defaults? Manual `forme
    trust-add`? TUF-style key rotation? FM02.1.

---

## 18. Success Criteria

FM02 is complete when:

1. **All six packages exist** under `code/packages/typescript/forme-*`,
   each with `package.json`, `BUILD`, `BUILD_windows`,
   `README.md`, `CHANGELOG.md`.
2. **Test coverage ≥ 95%** for `forme-manifest` and
   `forme-plugin-host`, ≥ 90% for the per-OS sandbox modules.
3. **The fixture plugin** (§15.5) loads end-to-end through
   `forme-orchestrator` via the subprocess host.
4. **Sandbox conformance tests pass** on Linux, macOS, and
   Windows in CI.
5. **A malicious fixture plugin** that tries to read `/etc/passwd`,
   `~/.ssh/`, the host's `node_modules`, and any non-granted
   capability is blocked at every layer (wire mediation + OS
   sandbox). Verified in CI.
6. **Wire protocol round-trips** all kernel kinds losslessly
   (the same `ContentNode` goes in and the same comes out).
7. **Cancellation honoured** within `cancellationGracePeriodMs`
   for cooperative plugins; uncooperative plugins killed at
   the deadline.
8. **Resource limits enforced** — verified by fixture plugins
   that allocate excess memory, spin CPU, open too many fds.
9. **`forme install` flow** prompts for and records grants
   correctly. Re-running with no manifest changes does not
   re-prompt.
10. **A first-party stage and a third-party plugin** coexist in
    one pipeline; the host routes each via the correct
    `PluginHost` (direct import vs subprocess) and the
    orchestrator is none the wiser.

---

## Appendix A — `plugin.toml` JSON Schema (informative)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Forme Plugin Manifest",
  "type": "object",
  "required": ["manifestVersion", "plugin", "runtime", "contributes"],
  "properties": {
    "manifestVersion": { "type": "integer", "enum": [1] },
    "plugin": {
      "type": "object",
      "required": ["name", "version", "apiVersion"],
      "properties": {
        "name":        { "type": "string", "pattern": "^(@[a-z0-9][a-z0-9-]*/)?[a-z0-9][a-z0-9-]*$" },
        "version":     { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+(-[A-Za-z0-9.-]+)?(\\+[A-Za-z0-9.-]+)?$" },
        "apiVersion":  { "type": "integer", "minimum": 1 },
        "description": { "type": "string" },
        "license":     { "type": "string" },
        "authors":     { "type": "array", "items": { "type": "string" } },
        "homepage":    { "type": "string", "format": "uri" },
        "repository":  { "type": "string", "format": "uri" }
      }
    },
    "runtime": {
      "type": "object",
      "required": ["kind", "entry"],
      "properties": {
        "kind":      { "enum": ["node", "deno", "bun", "python", "binary"] },
        "entry":     { "type": "string" },
        "platforms": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        }
      }
    },
    "capabilities": {
      "type": "object",
      "properties": {
        "required": {
          "type": "array",
          "items": { "$ref": "#/$defs/capabilityEntry" }
        },
        "optional": {
          "type": "array",
          "items": { "$ref": "#/$defs/capabilityEntry" }
        }
      }
    },
    "contributes": {
      "type": "object",
      "properties": {
        "stages": {
          "type": "array",
          "items": { "$ref": "#/$defs/stageEntry" }
        },
        "kinds": {
          "type": "array",
          "items": { "$ref": "#/$defs/kindEntry" }
        }
      }
    },
    "resources": {
      "type": "object",
      "properties": {
        "maxMemoryMb":        { "type": "integer", "minimum": 1 },
        "maxWallClockMs":     { "type": "integer", "minimum": 100 },
        "maxFileDescriptors": { "type": "integer", "minimum": 8 },
        "maxConcurrentRpcs":  { "type": "integer", "minimum": 1 }
      }
    },
    "signature": {
      "type": "object",
      "required": ["algorithm", "publicKey", "signature", "signedAt"],
      "properties": {
        "algorithm": { "enum": ["ed25519"] },
        "publicKey": { "type": "string" },
        "signature": { "type": "string" },
        "signedAt":  { "type": "string", "format": "date-time" }
      }
    }
  },
  "$defs": {
    "capabilityEntry": {
      "type": "object",
      "required": ["realm", "scope", "reason"],
      "properties": {
        "realm":  { "type": "string" },
        "scope":  { "type": "string" },
        "detail": { "type": "string" },
        "reason": { "type": "string" }
      }
    },
    "stageEntry": {
      "type": "object",
      "required": ["id", "consumes", "produces"],
      "properties": {
        "id":           { "type": "string" },
        "consumes":     { "type": "string" },
        "produces":     { "type": "string" },
        "configSchema": { "type": "string" }
      }
    },
    "kindEntry": {
      "type": "object",
      "required": ["name", "version"],
      "properties": {
        "name":      { "type": "string", "pattern": "^ext:[a-z0-9][a-z0-9-]*$" },
        "version":   { "type": "string" },
        "schema":    { "type": "string" },
        "subtypeOf": { "type": "string" }
      }
    }
  }
}
```

---

## Appendix B — Wire Protocol Reference

### B.1 Host-issued requests (host → plugin)

| Method | Params | Result | Phase |
|---|---|---|---|
| `handshake` | `HandshakeParams` | `HandshakeResult` | 1 |
| `announce` | `{}` | `AnnounceResult` | 2 |
| `stage.init` | `StageInitParams` | `null` | 3 |
| `stage.run` | `StageRunParams` | `StageRunResult` | 4 |
| `stage.dispose` | `{}` | `null` | 5 |

### B.2 Plugin-issued requests (plugin → host)

| Method | Params | Result | Capability |
|---|---|---|---|
| `ctx.storage.readFile` | `{ path }` | `{ bytes, mimeType }` | `filesystem:read` |
| `ctx.storage.statFile` | `{ path }` | `StorageStat` | `filesystem:read` |
| `ctx.storage.listDir` | `{ path }` | `readonly string[]` | `filesystem:read` |
| `ctx.storage.writeFile` | `{ path, bytes }` | `null` | `filesystem:write` |
| `ctx.storage.removeFile` | `{ path }` | `null` | `filesystem:write` |
| `ctx.storage.watch` | `{ path }` | `{ streamId }` | `filesystem:read` |
| `ctx.network.fetch` | `{ url, init? }` | `FetchResult` | `network:<host>` |
| `ctx.env.get` | `{ name }` | `string \| null` | `env:<name>` |
| `ctx.time.wallclockMs` | `{}` | `number` | `system:time` |
| `ctx.time.nowIso` | `{}` | `string` | `system:time` |
| `ctx.random.bytes` | `{ n }` | `string` (base64) | `system:random` |

### B.3 Notifications (either direction)

| Method | Params | Direction |
|---|---|---|
| `log` | `{ level, message, fields? }` | plugin → host |
| `stream.value` | `{ streamId, value }` | either |
| `stream.end` | `{ streamId }` | either |
| `stream.error` | `{ streamId, error }` | either |
| `$/cancelRequest` | `{ id }` | host → plugin |

### B.4 Error codes

See §10.1.

---

## Appendix C — Glossary

Terms introduced in this spec; see FM00 / FM01 / FM03 appendices
for the broader vocabulary.

- **Capability mediation** — the pattern of every plugin-side
  capability-gated call being routed across the wire to the host,
  which checks grants and either performs the operation or denies it.
- **Content-Length framing** — the LSP-style message framing:
  `Content-Length: N\r\n\r\n` followed by N bytes of payload.
- **Grants file** — `grants.toml` per plugin recording the
  user's accepted capabilities.
- **Manifest hash** — BLAKE2b-256 of the canonical TOML
  representation of the manifest (minus `[signature]`)
  concatenated with the entry file hash.
- **OS sandbox** — the platform-specific isolation layer
  (seccomp/sandbox-exec/Job Object) wrapping the plugin process.
- **Plugin process** — the OS process the host spawns to execute
  a plugin's code; one per stage instance.
- **Plugin runner** — the in-plugin library that implements the
  wire protocol's plugin side and exposes a `defineStage`-shaped
  API to the author.
- **Sandbox refusal** — the host's policy of refusing to load
  third-party plugins on a host where OS sandboxing is unavailable.
- **Trust store** — `~/.forme/trust.toml` listing publisher keys
  the user trusts for signature verification.
- **Trust tier** — first-party / verified-third-party /
  unverified-third-party.
- **Wire** — the JSON-RPC 2.0 stream over stdio carrying every
  host ↔ plugin message.

---

## Appendix D — Pointers to sibling specs

- **FM00** — Forme vision
- **FM01** — Kernel: types, kinds, stages, capabilities, identity
- **FM03** — Orchestrator: pipeline config, DAG, scheduling, cache, watch, repro
- **FM04** — Style IR (consumed by plugins)
- **FM05** — Interactivity IR (consumed by plugins)
- **FM06** — AOT compiler
- **FM07** — Dev server, CLI, shell integration (consumes `forme install`)

## Appendix E — This is a living document

Like FM00 / FM01 / FM03, FM02 evolves as implementation lands.
Where running code disagrees with this spec, the code wins and
the spec is updated; the history of the tension is part of the
project's record.

When packages from §14 begin landing, this document gains an
"Implementation Notes" appendix recording per-package divergences
(BLAKE2b vs BLAKE3, etc.) for the same reasons FM01's identity
package documented its own hash-algorithm choice.
