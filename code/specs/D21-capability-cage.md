# D21 — Capability Cage

## Overview

Java tried it with SecurityManager. .NET tried it with Code Access Security. Both failed
and were removed — Java deprecated SecurityManager in JDK 17 (JEP 411) and permanently
disabled it in JDK 24 (JEP 486); .NET dropped CAS entirely when moving to .NET Core.
Both failed for the same five reasons: opt-in security nobody opted into, policy files too
complex for humans, stack-walking permission checks that cascaded through dependencies,
in-process enforcement bypassable via native interop, and an "AllPermission" escape hatch
that became the universal default.

This package learns from those failures.

**Capability Cage** is a generic, language-agnostic capability security framework that
enforces the principle of least privilege for OS-level operations. It works at three
enforcement levels:

1. **Lint-time** — static analysis flags raw stdlib usage (`File.read`, `os.Open`,
   `socket.connect`) and requires the use of secure wrappers (`SecureFile.read`,
   `SecureNet.connect`). A second pass cross-references wrapper usage against the
   package's `required_capabilities.json` manifest.

2. **Runtime** — secure wrappers check the capability manifest before every OS operation.
   If a package tries to read a file it didn't declare in its manifest, it gets a
   `CapabilityViolationError` immediately, with a message explaining how to fix it.

3. **Hard cage** — for Chief of Staff agents (D18), the secure wrappers are the ONLY
   available API. Deno denies all raw stdlib access (`--deny-read`, `--deny-net`, etc.),
   and the wrappers delegate to the host process via JSON-RPC instead of stdlib. The
   agent literally cannot bypass the cage.

The key insight from studying successful models (Deno, Apple entitlements, WASI) is that
**successful permission systems start from zero authority and require explicit, simple,
auditable grants enforced below the application layer**. Failed systems started from full
authority and tried to subtract permissions via complex, optional, application-layer policy.

```
                    How Capability Cage avoids Java/CAS failures
┌────────────────────┬──────────────────────────┬──────────────────────────┐
│ Anti-pattern       │ Java SecurityManager     │ Capability Cage          │
│                    │ / .NET CAS               │                          │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ Stack walking      │ Every frame on the call  │ ONE check at the wrapper │
│                    │ stack must hold the       │ call site. No stack      │
│                    │ permission                │ inspection.              │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ Transitive perms   │ Callers AND callees must │ Flat manifest. If the    │
│                    │ have FilePermission      │ package declares fs:read │
│                    │                          │ any code in the package  │
│                    │                          │ can use SecureFile.read  │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ Hundreds of        │ FilePermission, Socket   │ 8 categories, 14 actions │
│ permission classes │ Permission, Property     │ The entire taxonomy fits │
│                    │ Permission, Runtime      │ on one page.             │
│                    │ Permission...            │                          │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ Default-allow      │ AllPermission for local  │ No manifest = zero       │
│                    │ code; SecurityManager    │ capabilities. The secure │
│                    │ disabled by default      │ path IS the default.     │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ AllPermission      │ Existed, became the      │ Structurally impossible. │
│ escape hatch       │ universal default        │ No "all" value in the    │
│                    │                          │ enum. Period.            │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ Runtime config     │ .policy files, JVM flags │ Static JSON manifest     │
│                    │ code signing policies    │ committed to the repo.   │
│                    │                          │ No runtime configuration.│
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ In-process only    │ Library-level checks     │ Boundary enforcement +   │
│                    │ bypassable via native    │ OS-level cage (Deno      │
│                    │ interop or JIT bugs      │ deny-all) for hard mode. │
├────────────────────┼──────────────────────────┼──────────────────────────┤
│ doPrivileged /     │ Temporarily elevates     │ No privilege elevation.  │
│ Assert blocks      │ permissions, undermining │ The manifest is fixed at │
│                    │ the security model       │ build time. Immutable.   │
└────────────────────┴──────────────────────────┴──────────────────────────┘
```

---

## Where It Fits

```
Capability Cage (D21)
│
├── defines ──► Capability Taxonomy
│                └── 8 categories (fs, net, proc, env, ffi, time, stdin, stdout)
│                └── 14 actions (read, write, create, delete, list, connect, ...)
│                └── Targets with glob matching
│
├── provides ──► Secure Wrappers
│                └── SecureFile, SecureNet, SecureProc, SecureEnv
│                └── Check manifest before every OS operation
│
├── provides ──► Manifest Loader
│                └── Loads and validates required_capabilities.json
│                └── Discovery via directory walk
│
├── used by ──► Every package in the monorepo
│                └── Pure packages: empty manifest (zero capabilities)
│                └── OS-touching packages: granular capability declarations
│
├── used by ──► Chief of Staff (D18)
│                └── Agent manifests extend this taxonomy
│                └── CageBackend replaces OpenBackend
│                └── host.* API enforces at the process boundary
│
├── extends ──► Spec 13 (Capability Security)
│                └── Implements the secure wrappers designed in Layer 2
│                └── Implements the linter rules designed in Layer 3
│                └── Provides data for the CI gate in Layer 4
│
├── uses ──► JSON Value (D20)
│             └── Parses required_capabilities.json manifests
│
└── uses ──► JSON Parser (existing)
              └── Parses manifest files from JSON text
```

**Depends on:** JSON Parser, JSON Value (for manifest loading).

**Used by:** Every package in the monorepo (manifest declaration), Chief of Staff agents
(hard cage enforcement), CI pipeline (static analysis gate), linters (usage verification).

---

## Key Concepts

### The Capability Manifest

Every package in the monorepo has a `required_capabilities.json` file that declares what
OS-level operations it needs. This is the single source of truth for a package's security
profile.

**The default is NOTHING.** A package without a manifest, or with an empty capabilities
array, has zero OS access. This is not a failure mode — it is the expected state for the
majority of packages, which are pure computation (arithmetic, data structures, parsers,
compilers, etc.).

```
required_capabilities.json
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ version          │ 1 (schema version for evolution)         │
│ package          │ "{language}/{package-name}"               │
│ capabilities     │ Array of capability declarations          │
│ justification    │ Why this package needs (or doesn't need)  │
│                  │ OS access                                 │
└──────────────────┴─────────────────────────────────────────┘

Capability
═══════════════════════════════════════════════════════════════
┌──────────────────┬─────────────────────────────────────────┐
│ category         │ One of: fs, net, proc, env, ffi, time,   │
│                  │ stdin, stdout                             │
│ action           │ One of: read, write, create, delete,      │
│                  │ list, connect, listen, dns, exec, fork,   │
│                  │ signal, call, load, sleep                 │
│ target           │ Specific resource with optional glob      │
│                  │ (e.g., "../../grammars/*.tokens")         │
│ justification    │ Why this specific capability is needed    │
└──────────────────┴─────────────────────────────────────────┘
```

**Example — a lexer package that reads grammar files:**

```json
{
  "version": 1,
  "package": "python/json-lexer",
  "capabilities": [
    {
      "category": "fs",
      "action": "read",
      "target": "../../grammars/json.tokens",
      "justification": "Reads token grammar definition file to build the lexer DFA."
    }
  ],
  "justification": "Reads one grammar file at initialization. No write, network, or process access."
}
```

**Example — a pure computation package:**

```json
{
  "version": 1,
  "package": "go/directed-graph",
  "capabilities": [],
  "justification": "Pure computation. No filesystem, network, process, or environment access needed."
}
```

**No AllPermission.** The `category` field is an enum with exactly 8 values. The `action`
field is an enum with exactly 14 values. There is no "all" value in either enum. To get
broad access, you must enumerate every `category:action` pair individually — making
escalation loud, visible in diffs, and impossible to hide.

---

### The Capability Taxonomy

The taxonomy defines what OS-level operations exist in the system. It is deliberately
small — 8 categories and 14 actions — so that a human can read and understand a complete
manifest in under 30 seconds.

**Contrast with Java:** Java's SecurityManager had hundreds of permission classes
(`FilePermission`, `SocketPermission`, `PropertyPermission`, `RuntimePermission`,
`AllPermission`, `AWTPermission`, `NetPermission`, `ReflectPermission`, `SerializablePermission`,
`SecurityPermission`...) with over 1,000 check points scattered across the JDK. Nobody
could reason about the full surface.

```
Category    Actions                          Target Format
─────────   ──────────────────────────       ──────────────────────
fs          read, write, create,             Path with glob
            delete, list                     e.g., "../../grammars/*.tokens"
                                             e.g., "/tmp/cache/*"

net         connect, listen, dns             host:port or hostname
                                             e.g., "imap.gmail.com:993"
                                             e.g., "api.weather.com:443"

proc        exec, fork, signal               Command name or PID
                                             e.g., "git"
                                             e.g., "*" (any command)

env         read, write                      Variable name
                                             e.g., "HOME"
                                             e.g., "PATH"

ffi         call, load                       Library name
                                             e.g., "libssl"

time        read, sleep                      "*" (time is not scoped)

stdin       read                             "*" (stdin is not scoped)

stdout      write                            "*" (stdout is not scoped)
```

**The `*` target:** Within a specific `category:action` pair, the target `*` means "any
resource of this type." For example, `fs:read:*` means "can read any file." This is
discouraged for most packages but necessary for infrastructure packages like `file-system`
(which IS the filesystem implementation) or `network-stack` (which IS the network stack).

**What `*` does NOT mean:** There is no way to say "grant all categories" or "grant all
actions." You must write `fs:read:*` AND `fs:write:*` AND `fs:create:*` separately. Each
line is a conscious, reviewable decision.

---

### Secure Wrappers

For each capability category, the package provides a drop-in wrapper that checks the
manifest before delegating to the real stdlib function. These wrappers serve three
purposes:

1. **Fail-fast during development.** A developer adding filesystem access immediately
   gets a `CapabilityViolationError` instead of discovering the problem 10 minutes later
   in CI.

2. **Audit trail via grep.** `grep -r "SecureFile" src/` shows every filesystem access
   point. Code review becomes: "did they use the wrapper? does the manifest match?"

3. **Linter hook point.** The linter flags raw stdlib calls and suggests the wrapper.

```
SecureFile.read("../../grammars/python.tokens")
│
│  Step 1: Check manifest
│  ┌──────────────────────────────────────────┐
│  │ manifest.check("fs", "read",             │
│  │   "../../grammars/python.tokens")        │
│  │                                          │
│  │ Walk the capabilities array:              │
│  │   cap[0]: fs:read:../../grammars/*.tokens │
│  │   Does "python.tokens" match "*.tokens"?  │
│  │   YES → capability granted                │
│  └──────────────────────────────────────────┘
│
│  Step 2: Delegate to backend
│  ┌──────────────────────────────────────────┐
│  │ OpenBackend:                              │
│  │   return stdlib.read("../../grammars/     │
│  │     python.tokens")                       │
│  │                                          │
│  │ CageBackend (Chief of Staff):            │
│  │   send JSON-RPC: {"method": "fs.read",   │
│  │     "params": {"path": "../../grammars/  │
│  │     python.tokens"}}                     │
│  │   return response.result                 │
│  └──────────────────────────────────────────┘
│
▼
File contents returned to caller
```

**No stack walking.** The wrapper checks the manifest exactly once, at the call site.
It does not inspect the call stack. It does not check whether the caller's caller has
permission. If the package declared `fs:read:*.tokens` in its manifest, any code in
the package can call `SecureFile.read("foo.tokens")`.

**Contrast with Java:** Java's `AccessController` walked the entire call stack. If
library A called library B called library C which opened a file, all three needed
`FilePermission`. This meant every dependency update could silently change required
permissions, and developers used `AccessController.doPrivileged()` to suppress the
walks — undermining the entire model.

---

### The Two-Backend Architecture

The secure wrappers have a pluggable backend. The backend determines HOW the OS
operation is performed after the capability check passes.

```
                     SecureFile / SecureNet / SecureProc / SecureEnv
                                    │
                            manifest.check()
                                    │
                     ┌──────────────┴──────────────┐
                     │                             │
              OpenBackend                    CageBackend
         (regular packages)           (Chief of Staff agents)
                     │                             │
              Real stdlib call            JSON-RPC to host
         (File.read, os.Open,         (host.fs.read via
          socket.connect, etc.)        stdin/stdout pipe)
                     │                             │
                     ▼                             ▼
              Operating System              Host Process (Rust)
                                      (double-checks capability,
                                       spawns ephemeral sub-agent,
                                       returns result, kills sub-agent)
```

**OpenBackend** — the default for regular packages. After the capability check passes,
it calls the real stdlib function. This is developer-facing enforcement: it catches
violations during development and testing. Raw stdlib is still physically callable
(the linter catches that at Layer 3).

**CageBackend** — for Chief of Staff agents. After the capability check passes, it sends
a JSON-RPC message to the host process over stdin/stdout. The host independently checks
the agent's manifest (double enforcement), spawns an ephemeral sub-agent to perform the
operation, returns the result, and kills the sub-agent. Raw stdlib is physically blocked
by Deno's `--deny-*` flags.

The CageBackend trait/interface is defined in this package. The implementation is deferred
to the Chief of Staff agent SDK (D18), which provides the concrete JSON-RPC transport.

---

### Manifest Discovery

When a secure wrapper is constructed without an explicit manifest path, it discovers the
manifest by walking up the directory tree from the current working directory:

```
Starting from: /repo/code/packages/python/json-lexer/src/json_lexer/
Walk up:
  /repo/code/packages/python/json-lexer/src/json_lexer/  → no manifest
  /repo/code/packages/python/json-lexer/src/              → no manifest
  /repo/code/packages/python/json-lexer/                  → FOUND! required_capabilities.json
  (stop here)

If not found before reaching a directory containing .git/:
  → treat as zero capabilities (deny everything)
```

The environment variable `CAPABILITY_CAGE_MANIFEST` overrides discovery, allowing CI and
testing to point to a specific manifest.

---

### Glob Matching

Targets in the manifest support a subset of glob syntax for practical scoping without
the complexity of full regex:

```
Pattern              Matches                  Does Not Match
─────────            ───────                  ──────────────
"data.txt"           "data.txt"               "data.csv", "other/data.txt"
"*.tokens"           "python.tokens",         "python.grammar"
                     "json.tokens"
"../../grammars/*"   "../../grammars/foo",    "../../other/foo"
                     "../../grammars/bar"
"*"                  everything               (nothing excluded)
```

**Rules:**
1. `*` matches any sequence of characters WITHIN a single path segment (no `/`).
2. `*` as the entire target matches everything.
3. Literal strings match exactly.
4. Path normalization resolves `../` segments BEFORE matching to prevent traversal
   attacks like `fs:read:../../safe/../../../etc/passwd`.

**What is NOT supported:** `**` (recursive glob), `?` (single character), `[a-z]`
(character classes). This keeps matching simple, fast, and auditable.

---

### Three Enforcement Modes

```
┌───────────────────────────────────────────────────────────────────┐
│  Mode 1: Lint-Time (static analysis)                               │
│                                                                    │
│  WHAT:  Linter scans source code for raw stdlib calls.             │
│         Flags: open(), File.read, os.ReadFile, socket.connect      │
│         Suggests: SecureFile.read, SecureNet.connect               │
│         Cross-references: SecureFile calls vs manifest entries     │
│                                                                    │
│  WHERE: BUILD file + independent CI gate                           │
│                                                                    │
│  CATCHES: ~95% of violations before code runs                     │
│                                                                    │
│  CANNOT CATCH: Dynamic dispatch, runtime-generated paths,          │
│                eval-based capability access                        │
├───────────────────────────────────────────────────────────────────┤
│  Mode 2: Runtime — Open Mode (regular packages)                    │
│                                                                    │
│  WHAT:  SecureFile/Net/Proc/Env check manifest at every call.     │
│         Raises CapabilityViolationError if undeclared.              │
│         Delegates to real stdlib via OpenBackend.                   │
│                                                                    │
│  WHERE: During development and test execution                      │
│                                                                    │
│  CATCHES: Everything that passes through the wrapper               │
│                                                                    │
│  CANNOT CATCH: Code that bypasses the wrapper and calls stdlib     │
│                directly (linter catches this at Mode 1)            │
├───────────────────────────────────────────────────────────────────┤
│  Mode 3: Hard Cage (Chief of Staff agents only)                    │
│                                                                    │
│  WHAT:  Deno deny-all flags block ALL raw stdlib access.           │
│         SecureFile/Net/Proc/Env delegate to host.* via CageBackend│
│         Host independently checks manifest (double enforcement).   │
│                                                                    │
│  WHERE: Inside Deno processes managed by Chief of Staff host       │
│                                                                    │
│  CATCHES: Everything. Unforgeable. Cannot be bypassed.             │
│                                                                    │
│  TO BYPASS: Must simultaneously exploit the host's safe Rust code  │
│             AND Deno's V8 sandbox. Both. At the same time.        │
└───────────────────────────────────────────────────────────────────┘
```

These modes are **complementary, not alternatives:**
- Regular packages use Modes 1 + 2 (lint + runtime)
- Chief of Staff agents use Modes 1 + 2 + 3 (lint + runtime + hard cage)

---

### Linter Rules

The package ships a `linter_rules.json` file that maps stdlib calls to required
capabilities for each language. External linters consume this data to flag raw stdlib
usage.

**Python — flagged patterns (rule `CAP001`):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `open(path)` | `fs:read:{path}` or `fs:write:{path}` | `SecureFile.read(path)` |
| `pathlib.Path.read_text()` | `fs:read:{path}` | `SecureFile.read(path)` |
| `os.listdir(path)` | `fs:list:{path}` | `SecureFile.list(path)` |
| `os.remove(path)` | `fs:delete:{path}` | `SecureFile.delete(path)` |
| `socket.connect()` | `net:connect:{host}:{port}` | `SecureNet.connect(host, port)` |
| `subprocess.run()` | `proc:exec:{cmd}` | `SecureProc.exec(cmd, args)` |
| `os.environ[key]` | `env:read:{key}` | `SecureEnv.read(key)` |

**Ruby — flagged patterns (cop `CA/DirectFileAccess`):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `File.read(path)` | `fs:read:{path}` | `SecureFile.read(path)` |
| `File.write(path)` | `fs:write:{path}` | `SecureFile.write(path)` |
| `Dir.glob(pattern)` | `fs:list:{pattern}` | `SecureFile.list(pattern)` |
| `TCPSocket.new()` | `net:connect:{host}:{port}` | `SecureNet.connect(host, port)` |
| `ENV[key]` | `env:read:{key}` | `SecureEnv.read(key)` |

**Go — flagged patterns (vet analyzer `cap`):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `os.ReadFile(path)` | `fs:read:{path}` | `capabilitycage.ReadFile(m, path)` |
| `os.Create(path)` | `fs:create:{path}` | `capabilitycage.CreateFile(m, path)` |
| `net.Dial()` | `net:connect:{host}:{port}` | `capabilitycage.Connect(m, host, port)` |
| `os/exec.Command()` | `proc:exec:{cmd}` | `capabilitycage.Exec(m, cmd, args)` |
| `os.Getenv(key)` | `env:read:{key}` | `capabilitycage.ReadEnv(m, key)` |

**TypeScript — flagged patterns (ESLint rule `capability-cage/no-raw-io`):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `Deno.readFile()` | `fs:read:{path}` | `secureFile.read(path)` |
| `Deno.writeFile()` | `fs:write:{path}` | `secureFile.write(path)` |
| `fetch()` | `net:connect:{url}` | `secureNet.connect(host, port)` |
| `Deno.env.get()` | `env:read:{key}` | `secureEnv.read(key)` |

**Rust — flagged patterns (clippy lint `capability_cage_raw_io`):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `std::fs::read()` | `fs:read:{path}` | `SecureFile::read(m, path)` |
| `std::fs::write()` | `fs:write:{path}` | `SecureFile::write(m, path, data)` |
| `std::net::TcpStream::connect()` | `net:connect:{addr}` | `SecureNet::connect(m, host, port)` |
| `std::env::var()` | `env:read:{key}` | `SecureEnv::read(m, key)` |

**Elixir — flagged patterns (Credo check `CapabilityCage.NoRawIO`):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `File.read()` | `fs:read:{path}` | `SecureFile.read(m, path)` |
| `File.write()` | `fs:write:{path}` | `SecureFile.write(m, path, data)` |
| `:gen_tcp.connect()` | `net:connect:{host}:{port}` | `SecureNet.connect(m, host, port)` |
| `System.get_env()` | `env:read:{key}` | `SecureEnv.read(m, key)` |

**Lua — flagged patterns (custom lint):**

| Raw Stdlib Call | Required Capability | Secure Wrapper |
|---|---|---|
| `io.open(path)` | `fs:read:{path}` or `fs:write:{path}` | `SecureFile:read(path)` |
| `os.execute(cmd)` | `proc:exec:{cmd}` | `SecureProc:exec(cmd, args)` |
| `os.getenv(key)` | `env:read:{key}` | `SecureEnv:read(key)` |
| `os.clock()` | `time:read:*` | `SecureTime:read()` |

**Suppression comments** — the secure wrapper package itself uses raw stdlib calls
internally. It suppresses linter warnings with language-native comment formats:

```
Python:     # noqa: CAP001
Ruby:       # rubocop:disable CA/DirectFileAccess
Go:         //nolint:cap
TypeScript: // eslint-disable-next-line capability-cage/no-raw-io
Rust:       #[allow(capability_cage_raw_io)]
Elixir:     # credo:disable-for-next-line CapabilityCage.NoRawIO
Lua:        -- nolint:cap
```

---

## Public API

### Core Types

```
Manifest
  .discover() -> Manifest         Walk up directories to find manifest
  .load(path) -> Manifest         Load from explicit path
  .check(cat, action, target)     Raise CapabilityViolationError if denied
  .has_capability(cat, act, tgt)  Boolean check (no-throw)
  .capabilities -> List           All declared capabilities

Capability
  .category -> String             fs, net, proc, env, ffi, time, stdin, stdout
  .action -> String               read, write, create, etc.
  .target -> String               Specific resource or glob
  .justification -> String        Why this capability is needed

CapabilityViolationError
  .category -> String             The denied category
  .action -> String               The denied action
  .target -> String               The denied target
  .manifest_path -> String        Path to the manifest that denied it
  .message -> String              Human-readable fix instructions

ManifestError
  .message -> String              What went wrong (invalid JSON, schema, etc.)

Backend (interface/trait)
  .read_file(path) -> bytes
  .write_file(path, data)
  .create_file(path)
  .delete_file(path)
  .list_dir(path) -> list[string]
  .connect(host, port) -> connection
  .listen(host, port) -> listener
  .dns_lookup(host) -> list[address]
  .exec(cmd, args) -> result
  .read_env(name) -> string
  .write_env(name, value)

OpenBackend implements Backend
  Delegates every method to the language's stdlib.

CageBackend implements Backend (defined here, implemented in D18)
  Delegates every method to host.* via JSON-RPC over stdin/stdout.
```

### Secure Wrappers

```
SecureFile(manifest, backend=OpenBackend)
  .read(path: string) -> bytes          checks fs:read:{path}
  .write(path: string, data: bytes)     checks fs:write:{path}
  .create(path: string)                 checks fs:create:{path}
  .delete(path: string)                 checks fs:delete:{path}
  .list(path: string) -> list[string]   checks fs:list:{path}

SecureNet(manifest, backend=OpenBackend)
  .connect(host, port) -> connection    checks net:connect:{host}:{port}
  .listen(host, port) -> listener       checks net:listen:{host}:{port}
  .dns_lookup(host) -> list[address]    checks net:dns:{host}

SecureProc(manifest, backend=OpenBackend)
  .exec(cmd, args) -> result            checks proc:exec:{cmd}
  .fork() -> pid                        checks proc:fork:*
  .signal(pid, sig)                     checks proc:signal:{pid}

SecureEnv(manifest, backend=OpenBackend)
  .read(name: string) -> string         checks env:read:{name}
  .write(name: string, value: string)   checks env:write:{name}
```

### Language-Specific API

**Python:**
```python
from capability_cage import SecureFile, Manifest

manifest = Manifest.discover()
fs = SecureFile(manifest)
data = fs.read("../../grammars/python.tokens")
```

**Ruby:**
```ruby
require "coding_adventures_capability_cage"

manifest = CodingAdventures::CapabilityCage::Manifest.discover
fs = CodingAdventures::CapabilityCage::SecureFile.new(manifest)
data = fs.read("../../grammars/python.tokens")
```

**Go:**
```go
manifest, _ := capabilitycage.Discover()
data, err := capabilitycage.ReadFile(manifest, "../../grammars/python.tokens")
```

**TypeScript:**
```typescript
import { SecureFile, Manifest } from "@coding-adventures/capability-cage";
const manifest = Manifest.discover();
const fs = new SecureFile(manifest);
const data = await fs.read("../../grammars/python.tokens");
```

**Rust:**
```rust
use coding_adventures_capability_cage::{Manifest, SecureFile};
let manifest = Manifest::discover()?;
let fs = SecureFile::new(manifest);
let data = fs.read("../../grammars/python.tokens")?;
```

**Elixir:**
```elixir
manifest = CodingAdventures.CapabilityCage.Manifest.discover!()
data = CodingAdventures.CapabilityCage.SecureFile.read!(manifest, "../../grammars/python.tokens")
```

**Lua:**
```lua
local CapabilityCage = require("capability_cage")
local manifest = CapabilityCage.Manifest.discover()
local fs = CapabilityCage.SecureFile.new(manifest)
local data = fs:read("../../grammars/python.tokens")
```

---

## Test Strategy

### Unit Tests

**Manifest loading (10 tests):**
1. Load valid manifest with capabilities
2. Load valid manifest with empty capabilities array
3. Missing manifest file returns zero-capability manifest
4. Invalid JSON raises ManifestError
5. Missing required field raises ManifestError
6. Invalid category enum value raises ManifestError
7. Invalid action enum value raises ManifestError
8. Schema version mismatch raises ManifestError
9. Manifest with banned_construct_exceptions parses correctly
10. Environment variable override for manifest path

**Manifest discovery (5 tests):**
11. Discovers manifest in current directory
12. Discovers manifest in parent directory
13. Discovers manifest in grandparent directory
14. Stops at git root (directory containing `.git/`)
15. Returns zero-capability manifest if not found

**Capability checking (8 tests):**
16. Exact target match succeeds
17. Glob target match succeeds (`*.tokens` matches `python.tokens`)
18. Wildcard target `*` matches everything
19. Unmatched target raises CapabilityViolationError
20. Unmatched action raises CapabilityViolationError
21. Unmatched category raises CapabilityViolationError
22. Multiple capabilities — first match wins
23. Empty capabilities array denies everything

**Glob matching (8 tests):**
24. Literal string matches exactly
25. `*` at end matches any suffix
26. `*` at beginning matches any prefix
27. `*` in middle matches any infix
28. `*` as entire target matches everything
29. Multiple `*` characters match correctly
30. Literal with no `*` requires exact match
31. Empty pattern matches nothing

**Path normalization (5 tests):**
32. `../` segments resolved before matching
33. `./` segments removed
34. Trailing slashes normalized
35. Double slashes normalized
36. Traversal attack `../../safe/../../../etc/passwd` normalized correctly

**SecureFile (10 tests):**
37. Read with valid capability returns file contents
38. Read without capability raises CapabilityViolationError
39. Write with valid capability succeeds
40. Write without capability raises error
41. Create with valid capability succeeds
42. Create without capability raises error
43. Delete with valid capability succeeds
44. Delete without capability raises error
45. List with valid capability returns entries
46. List without capability raises error

**SecureNet (6 tests):**
47. Connect with valid host:port capability succeeds
48. Connect to undeclared host:port raises error
49. Listen with valid capability succeeds
50. Listen without capability raises error
51. DNS lookup with valid capability succeeds
52. DNS lookup without capability raises error

**SecureProc (6 tests):**
53. Exec with declared command succeeds
54. Exec with undeclared command raises error
55. Fork with declared capability succeeds
56. Fork without capability raises error
57. Signal with declared capability succeeds
58. Signal without capability raises error

**SecureEnv (4 tests):**
59. Read declared env var succeeds
60. Read undeclared env var raises error
61. Write declared env var succeeds
62. Write undeclared env var raises error

**Backend (6 tests):**
63. OpenBackend.read_file delegates to stdlib
64. OpenBackend.write_file delegates to stdlib
65. OpenBackend.list_dir delegates to stdlib
66. CageBackend trait/interface exists and is implementable
67. CageBackend method signatures match Backend contract
68. Custom backend can be injected into SecureFile

**Error messages (5 tests):**
69. Error contains denied category:action:target triple
70. Error contains manifest path
71. Error contains human-readable fix instructions
72. Error message mentions `required_capabilities.json`
73. Error is catchable as language-specific exception type

**Integration (5 tests):**
74. End-to-end: load manifest → create SecureFile → read file → verify contents
75. End-to-end: empty manifest → SecureFile.read → CapabilityViolationError
76. End-to-end: manifest with glob → SecureFile.read matching file → success
77. End-to-end: manifest with glob → SecureFile.read non-matching file → error
78. Linter rules JSON loads and validates correctly

**Edge cases (5 tests):**
79. Zero-capability manifest denies all operations
80. Missing manifest file denies all operations
81. Manifest with only net capabilities denies fs operations
82. Target with special characters handled correctly
83. Very long target string handled correctly

### Coverage Target

95%+ line coverage for all library code across all 7 languages.

---

## Dependencies

```
D21 Capability Cage
│
├── uses ──► JSON Parser (existing)
│             └── Parses required_capabilities.json from JSON text
│
├── uses ──► JSON Value (D20)
│             └── Typed access to parsed manifest fields
│
├── extends ──► Spec 13 (Capability Security)
│                └── Taxonomy, wrapper design, linter rules
│
├── used by ──► Every package (~572 packages, 7 languages)
│                └── Each declares required_capabilities.json
│
├── used by ──► Chief of Staff (D18)
│                └── CageBackend for hard enforcement
│                └── Agent manifests extend capability taxonomy
│
└── used by ──► CI Pipeline
                 └── Static analysis gate
                 └── Linter integration
```

---

## Trade-Offs

1. **Runtime overhead.** Every OS operation goes through a manifest check. For the kinds
   of operations packages in this monorepo perform — reading a grammar file once at
   initialization, writing a cache file — this adds microseconds. For hot-path file I/O
   in a benchmark, use the raw stdlib with a linter suppression comment.

2. **Not a true sandbox.** In open mode, a developer who ignores the linter and calls
   `File.read()` directly bypasses the wrapper. The linter catches this statically, and
   the CI gate catches it independently. But at runtime, the bypass succeeds. For true
   sandboxing, use cage mode (Chief of Staff).

3. **Manifest maintenance.** Every package needs a manifest. For pure computation packages,
   this is a one-line empty-capabilities file. For OS-touching packages, the manifest must
   be kept in sync with actual usage. The linter's cross-reference check (manifest vs.
   actual wrapper calls) catches drift.

4. **JSON parsing dependency.** The manifest loader depends on the monorepo's json-parser
   and json-value packages. This is a non-trivial dependency chain (lexer → parser →
   json-lexer → json-parser → json-value). For a bootstrap-level security package, this
   is acceptable because these are all internal, zero-external-dependency packages.

5. **No dynamic capabilities.** The manifest is static — you cannot request new capabilities
   at runtime. This is a feature: it means the security profile of a package is knowable
   from its manifest alone, without running the code.
