# B02 — Hermetic Build Sandbox

## 1. Overview

A build system is only as trustworthy as its isolation guarantees. If a package
can silently read files it never declared as inputs, you get "works on my
machine" builds — the output depends on whatever happens to be lying around on
the host filesystem. The fix is simple in principle and subtle in practice:
**run every build action in a directory that contains nothing except declared
inputs.**

This specification describes the *copy-based hermetic sandbox* used by the
coding-adventures build tool. The core idea:

1. Create a fresh temporary directory.
2. Physically copy only the files the action declared as inputs.
3. Execute the build commands inside that directory.
4. Copy the declared outputs into a content-addressable cache.
5. Delete the temporary directory.

If a source file, dependency output, or vendored library isn't explicitly listed
in the BUILD file, it simply does not exist inside the sandbox. An undeclared
dependency produces an immediate "file not found" error — not a silent,
unreproducible success.

This is the same philosophy used by Bazel, Buck2, and Nix, scaled down to
something a single developer can understand and maintain.

---

## 2. Why Copy, Not Symlink?

The most common shortcut for build sandboxing is to symlink inputs rather than
copy them. This is faster on paper, but it introduces several categories of
bugs that are painful to diagnose:

| Concern                       | Symlink                                    | Physical Copy                        |
|-------------------------------|--------------------------------------------|--------------------------------------|
| **Windows compatibility**     | Requires admin privileges or Developer Mode | Works everywhere, no special perms   |
| **Implicit dependency leak**  | Symlink can resolve to parent directories  | Missing file = immediate failure     |
| **Build tool mutations**      | Tool could write through symlink to source | Source tree is never touched         |
| **Parallel build safety**     | Symlink races when multiple actions share  | Each sandbox is fully independent    |
| **Reproducibility**           | Depends on host filesystem layout          | Identical sandbox on every machine   |
| **Debugging clarity**         | `ls -la` shows symlinks, confusing output  | What you see is what you have        |

The symlink approach is an optimization. We start with copies because
correctness matters more than speed — and as we'll see in Section 11,
modern filesystems make copies nearly free anyway.

> **Design principle:** Choose the approach where bugs are loud. A missing
> physical file crashes immediately. A dangling symlink might work on one
> machine and fail on another, depending on directory structure.

---

## 3. Sandbox Directory Layout

Every build action gets its own sandbox directory. The layout is predictable
and uniform across all languages:

```
$SANDBOX_ROOT/
└── <action-hash>/
    ├── workspace/
    │   ├── src/          # physically copied declared source files
    │   ├── deps/         # physically copied outputs from internal dependencies
    │   │   ├── arithmetic/
    │   │   │   └── lib/
    │   │   └── logic_gates/
    │   │       └── lib/
    │   └── vendor/       # physically copied vendored external packages
    │       ├── pip/
    │       ├── npm/
    │       └── cargo/
    └── output/           # the action writes its outputs here
```

Where `$SANDBOX_ROOT` defaults to:

- **macOS / Linux:** `$TMPDIR/coding-adventures-build/` (typically `/tmp/`)
- **Windows:** `os.TempDir()\coding-adventures-build\`

The `<action-hash>` is a SHA-256 digest that uniquely identifies the action
(see Section 6 for how it's computed). This means two identical actions produce
the same sandbox path — useful for debugging, though the sandbox is normally
deleted after execution.

### Why this structure?

The three subdirectories under `workspace/` mirror the three categories of
build inputs:

- **`src/`** — Files the package itself owns. These come from the `srcs` field
  in the BUILD file.
- **`deps/`** — Outputs produced by other packages in this monorepo. These come
  from the `deps` field. Each dependency's outputs are placed in a subdirectory
  named after the package.
- **`vendor/`** — External packages fetched from registries (PyPI, npm, crates.io).
  These come from the `external_deps` field and are pre-downloaded into
  `.build/vendor/`.

The `output/` directory is where the action must write its results. Only files
in `output/` are captured after execution. Anything written to `workspace/` is
discarded — this prevents build actions from accidentally depending on
intermediate artifacts.

---

## 4. Build Action Lifecycle

A build action moves through nine distinct phases. Each phase either succeeds
completely or aborts the entire action. There is no partial success.

### Phase 1: Create Sandbox Directory

```go
sandboxDir, err := os.MkdirTemp(sandboxRoot, "action-"+actionHash[:12]+"-")
```

The directory name includes a prefix of the action hash for easy identification
when debugging. `MkdirTemp` guarantees uniqueness even under parallel execution.

### Phase 2: Copy Declared Sources

For each glob pattern in the `srcs` field, expand it against the package
directory and physically copy matching files into `workspace/src/`, preserving
relative paths:

```
BUILD file:  srcs = ["lib/**/*.ex", "mix.exs"]
Source tree:  code/packages/elixir/arithmetic/lib/arithmetic.ex
Sandbox:      workspace/src/lib/arithmetic.ex
```

Files that don't match any `srcs` glob are invisible to the build. This is the
mechanism that catches undeclared dependencies.

### Phase 3: Copy Internal Dependency Outputs

For each package listed in `deps`, locate its cached build outputs and copy
them into `workspace/deps/<package-name>/`:

```
BUILD file:  deps = ["//elixir/logic_gates"]
Cache:        .build/cache/cas/<sha256-of-logic-gates-output>
Sandbox:      workspace/deps/logic_gates/lib/logic_gates.ex
```

If a dependency hasn't been built yet, the scheduler must build it first. The
build tool's DAG traversal (specified in B01) guarantees this ordering.

### Phase 4: Copy Vendored Externals

External packages are pre-fetched into `.build/vendor/` by a separate fetch
step. The sandbox copies the relevant vendored directories:

```
BUILD file:  external_deps = ["pip:pytest@8.0"]
Vendor dir:   .build/vendor/pip/pytest@8.0/
Sandbox:      workspace/vendor/pip/pytest@8.0/
```

### Phase 5: Set Working Directory

The build command's working directory is set to the sandbox workspace:

```go
cmd := exec.Command(buildCmd[0], buildCmd[1:]...)
cmd.Dir = filepath.Join(sandboxDir, "workspace")
```

This is critical. The build tool never `cd`s into the sandbox — it sets `Dir`
on the command struct, which is both safer and more explicit.

### Phase 6: Set Clean Environment

The sandbox provides a **minimal, controlled environment**. No variables from
the host leak in:

```go
cmd.Env = []string{
    "PATH=" + minimalPath,
    "HOME=" + sandboxDir,
    "TMPDIR=" + filepath.Join(sandboxDir, "tmp"),
    "LANG=en_US.UTF-8",
    // Language-specific vars added per action type
}
```

See Section 7 for the full environment isolation specification.

### Phase 7: Execute Build Commands

Commands from `build_cmds` run sequentially. Each must exit 0 or the action
fails:

```go
for _, cmdLine := range action.BuildCmds {
    cmd := exec.Command("sh", "-c", cmdLine)
    cmd.Dir = workspaceDir
    cmd.Env = sandboxEnv
    cmd.Stdout = actionLog
    cmd.Stderr = actionLog
    if err := cmd.Run(); err != nil {
        return fmt.Errorf("command failed: %s: %w", cmdLine, err)
    }
}
```

Standard output and standard error are captured to an action log, which is
printed on failure and optionally retained on success (with `--verbose`).

### Phase 8: Capture Outputs

After successful execution, declared output files are copied from
`workspace/output/` into the content-addressable store:

```
Sandbox:  workspace/output/lib/arithmetic.beam
CAS:      .build/cache/cas/a1b2c3d4...  (SHA-256 of file contents)
Manifest: .build/cache/actions/<action-hash>.json
```

The action manifest records the mapping from output paths to content hashes,
so future cache hits can reconstruct the outputs without re-executing.

### Phase 9: Delete Sandbox

```go
if !keepSandbox {
    os.RemoveAll(sandboxDir)
}
```

The `--keep-sandbox` flag skips this step, leaving the sandbox directory intact
for post-mortem inspection.

---

## 5. Platform-Specific Isolation

The copy-based sandbox is the universal foundation. On platforms that support
it, additional isolation layers can be applied for defense in depth.

### macOS

**Copy performance:** APFS supports `clonefile(2)`, a copy-on-write clone that
completes in constant time regardless of file size. The kernel shares the
underlying data blocks until one side writes, at which point only the modified
blocks are duplicated. This makes our "copy everything" approach nearly as fast
as symlinking on APFS volumes.

```go
// Attempt clonefile first, fall back to regular copy
err := unix.Clonefile(src, dst, unix.CLONE_NOFOLLOW)
if err != nil {
    // Fallback: regular file copy (e.g., cross-device or non-APFS)
    return copyFileRegular(src, dst)
}
```

**Optional sandbox-exec profile:** macOS provides `sandbox-exec(1)`, which can
restrict a process to a specific set of filesystem paths and deny network
access. A future enhancement could wrap build commands in a sandbox profile:

```scheme
(version 1)
(deny default)
(allow file-read* (subpath "/sandbox/workspace"))
(allow file-write* (subpath "/sandbox/output"))
(allow process-exec (subpath "/usr/bin"))
(deny network*)
```

This is not required for correctness — the copy-based sandbox already prevents
undeclared dependencies — but it adds a second layer of protection against
build tools that reach outside their sandbox.

### Linux

**Copy performance:** If `$TMPDIR` points to a tmpfs mount (common on modern
distributions), file copies are memory-to-memory operations. Even without
tmpfs, modern ext4 and btrfs handle small file copies efficiently.

**Optional namespace isolation:** Linux namespaces provide kernel-level
isolation without requiring root (via user namespaces):

```go
cmd.SysProcAttr = &syscall.SysProcAttr{
    Cloneflags: syscall.CLONE_NEWNS |   // mount namespace
                syscall.CLONE_NEWNET |  // network namespace (no internet)
                syscall.CLONE_NEWPID,   // PID namespace (can't see others)
    UidMappings: []syscall.SysProcIDMap{{
        ContainerID: 0, HostID: os.Getuid(), Size: 1,
    }},
}
```

- **CLONE_NEWNS:** The sandbox directory is the only writable mount point.
  The rest of the filesystem is mounted read-only (or not at all).
- **CLONE_NEWNET:** The build process has no network access. If it tries to
  `pip install` or `npm install` during the build, it fails immediately.
- **CLONE_NEWPID:** The build process can't see or signal other processes on
  the system.

**Fallback:** If namespaces aren't available (containers, older kernels), the
build falls back to plain copy + cwd isolation. A warning is printed:

```
warning: namespace isolation unavailable, using copy-only sandbox
```

### Windows

Windows has the most limited isolation options, which is precisely why the
copy-based approach was chosen as the universal foundation:

- **Sandbox root:** `os.TempDir()` returns an appropriate location that avoids
  the 260-character path length limit on older Windows APIs.
- **Clean environment:** `cmd.Env` is set to an explicit allowlist. Unlike Unix
  where env inheritance is opt-out, Go's `exec.Cmd` with a non-nil `Env` field
  replaces the entire environment — exactly what we want.
- **Future: Docker-based CI isolation.** For stricter sandboxing on Windows CI
  runners, build actions can optionally execute inside a Docker container (see
  Section 10).

---

## 6. Content-Addressable Cache

The cache is what makes sandboxing fast. Without it, every build would copy
files and re-execute from scratch. With it, unchanged actions are skipped
entirely.

### Cache Directory Structure

```
.build/
├── cache/
│   ├── actions/
│   │   └── <action-hash>.json    # maps action → output content hashes
│   └── cas/
│       └── <sha256>              # content-addressed file blobs
└── vendor/
    ├── pip/<package>@<version>/
    ├── npm/<package>@<version>/
    ├── cargo/<crate>@<version>/
    ├── mix/<package>@<version>/
    ├── go/<module>@<version>/
    ├── bundler/<gem>@<version>/
    └── gradle/<artifact>@<version>/
```

### Action Hash Computation

The action hash uniquely identifies a build action based on everything that
could affect its output:

```
action_hash = SHA256(
    sorted(command_strings)          +
    sorted(input_content_hashes)     +
    sorted(env_key=value_pairs)
)
```

Each component is serialized as a length-prefixed byte string to prevent
ambiguity. The sort ensures that declaration order doesn't affect the hash.

**Example:** If you have two source files and one build command:

```
SHA256(
    "cmd:mix compile\n" +
    "input:a1b2c3d4:lib/arithmetic.ex\n" +
    "input:e5f6a7b8:mix.exs\n" +
    "env:MIX_ENV=prod\n"
)
```

### Cache Lookup

```
1. Compute action hash from current inputs
2. Look up .build/cache/actions/<hash>.json
3. If found (cache HIT):
   a. Read output manifest (path → content hash)
   b. Copy each output from .build/cache/cas/<hash> to destination
   c. Skip sandbox creation entirely
4. If not found (cache MISS):
   a. Create sandbox
   b. Execute action
   c. Hash each output file
   d. Store outputs in .build/cache/cas/
   e. Write action manifest to .build/cache/actions/
```

### Cache Invalidation

There is no explicit invalidation step. The cache is automatically invalidated
by the action hash:

- **Source file changes** produce a different input content hash, which changes
  the action hash, which causes a cache miss.
- **Command changes** (e.g., adding a compiler flag) change the command strings,
  which changes the action hash.
- **Environment changes** change the env vars component of the hash.

To force a full rebuild, delete `.build/cache/` or use `--force`.

---

## 7. Environment Isolation

A hermetic build requires a hermetic environment. If a build action inherits
the developer's `$GOPATH` or `$PYTHONPATH`, the output may differ from machine
to machine.

### Base Environment (All Actions)

Every sandbox action receives exactly these variables and no others:

| Variable   | Value                              | Rationale                          |
|------------|------------------------------------|------------------------------------|
| `PATH`     | `/usr/bin:/bin` + toolchain paths  | Minimal system utilities           |
| `HOME`     | `<sandbox-dir>`                    | Prevent reading user dotfiles      |
| `TMPDIR`   | `<sandbox-dir>/tmp`                | Temp files stay inside sandbox     |
| `LANG`     | `en_US.UTF-8`                      | Consistent locale across machines  |
| `LC_ALL`   | `en_US.UTF-8`                      | Override any locale shenanigans    |
| `SOURCE_DATE_EPOCH` | `315532800`             | Reproducible timestamps (Jan 1, 1980) |

### Language-Specific Extensions

Each language's build rule adds variables appropriate for that toolchain:

**Elixir / Erlang:**
```
MIX_ENV=prod
MIX_HOME=<sandbox>/mix_home
HEX_HOME=<sandbox>/hex_home
```

**Go:**
```
GOPATH=<sandbox>/gopath
GOMODCACHE=<sandbox>/gomodcache
GOCACHE=<sandbox>/gocache
```

**Python:**
```
PYTHONDONTWRITEBYTECODE=1
PYTHONHASHSEED=0
VIRTUAL_ENV=<sandbox>/venv
```

**Rust:**
```
CARGO_HOME=<sandbox>/cargo_home
RUSTUP_HOME=<sandbox>/rustup_home
```

**Node.js / TypeScript:**
```
NODE_PATH=<sandbox>/node_modules
npm_config_cache=<sandbox>/npm_cache
```

The principle: every language has directories it reads configuration and
caches from. In the sandbox, all of those point inside the sandbox directory.

---

## 8. Failure Modes and Diagnostics

Sandboxed builds fail in predictable, diagnosable ways. This section catalogs
the failure modes and their resolutions.

### Undeclared Input Access

**Symptom:** `file not found: ../../../some/other/package/util.ex`

**Cause:** The build action tried to read a file that wasn't listed in `srcs`
or `deps`. In an unsandboxed build, this would silently succeed if the file
happened to exist in the right relative location.

**Fix:** Add the missing file to `srcs` or add the missing package to `deps`.

This is the sandbox working as designed. The failure is the feature.

### Sandbox Creation Failure

**Symptom:** `failed to create sandbox: permission denied` or `no space left`

**Cause:** The sandbox root directory doesn't exist, isn't writable, or the
filesystem is full.

**Fix:** Check `$TMPDIR` permissions. Use `--sandbox-root /path/to/other/volume`
to point to a different location with more space.

### Output Not Produced

**Symptom:** `expected output not found: workspace/output/lib/foo.beam`

**Cause:** The build command succeeded (exit 0) but didn't write the expected
output files to the `output/` directory.

**Fix:** Check that the build command's output path is configured to write into
`$OUTPUT_DIR` (which the sandbox sets to `workspace/output/`).

### Network Access Denied

**Symptom:** `getaddrinfo: Name or service not known` (with namespace isolation)

**Cause:** The build action tried to access the network, but the sandbox has
network isolation enabled.

**Fix:** External dependencies must be vendored before the build (see the
`vendor/` directory). Build actions should never download anything at runtime.

---

## 9. Debugging

When a sandboxed build fails, these flags help diagnose the problem:

### `--keep-sandbox`

Prevents cleanup of the sandbox directory after execution. The build output
includes the sandbox path:

```
[arithmetic] sandbox: /tmp/coding-adventures-build/action-a1b2c3d4-XYZ123/
[arithmetic] FAILED (keeping sandbox for inspection)
```

You can then `cd` into the sandbox and inspect exactly what files were
available, run commands manually, and understand why the build failed.

### `--sandbox-root <path>`

Overrides the default sandbox location. Useful when `/tmp` is too small or
on a slow filesystem:

```
./build-tool --sandbox-root /mnt/fast-ssd/build-sandbox
```

### `--no-sandbox`

Disables sandboxing entirely. The build runs directly in the source tree, as if
there were no sandbox at all. This is an escape hatch for debugging — it prints
a prominent warning:

```
WARNING: sandboxing disabled — build may use undeclared inputs
WARNING: results are NOT reproducible in this mode
```

Never use `--no-sandbox` in CI. It exists solely for local debugging when you
need to determine whether a failure is sandbox-related or build-related.

### `--verbose`

Prints the full action log (stdout + stderr) for every action, not just
failures. Combined with `--keep-sandbox`, this gives complete visibility into
what happened inside the sandbox.

---

## 10. Future: Container-Based Isolation

The copy-based sandbox prevents undeclared file dependencies. Namespace
isolation (Linux) prevents network access and process visibility. But for
maximum reproducibility — identical system libraries, identical kernel
interfaces — container-based isolation is the gold standard.

A future enhancement will allow BUILD files to opt into container execution:

```starlark
python_library(
    name = "stats",
    srcs = ["lib/**/*.py"],
    deps = ["//python/matrix"],
    sandbox = "container",
    container_image = "python:3.12-slim",
)
```

When `sandbox = "container"`, the build tool:

1. Creates the sandbox directory as usual (Phases 1-4).
2. Mounts the sandbox directory into a Docker container.
3. Executes the build commands inside the container.
4. Captures outputs from the sandbox directory (which is a bind mount).

The container provides:
- **Reproducible system libraries:** Every build sees the same libc, openssl, etc.
- **Architecture isolation:** Cross-compilation via multi-platform images.
- **Complete network lockdown:** Docker's default bridge can be disabled entirely.

The tradeoff is startup latency (hundreds of milliseconds per container) and
Docker as a required dependency. This is acceptable for CI but likely too slow
for local development iteration. The copy-based sandbox remains the default.

---

## 11. Performance Considerations

"Copying every file for every build sounds slow." It isn't, for three reasons:

### APFS clonefile(2) on macOS

Apple's APFS filesystem implements `clonefile(2)`, a system call that creates
a copy-on-write clone of a file in constant time — O(1) regardless of file
size. The kernel shares the underlying data blocks between source and clone.
Only when one side writes does the modified block get duplicated.

For our sandbox, this means "copying" a 10MB compiled artifact takes the same
time as copying a 10-byte config file: essentially zero. The Go standard
library's `os.Link` doesn't use clonefile, so we call it directly via
`unix.Clonefile`.

### tmpfs on Linux

Most modern Linux distributions mount `/tmp` as tmpfs — a filesystem backed by
RAM (and swap). File copies to tmpfs are memory-to-memory operations, limited
only by memory bandwidth. A typical developer machine can copy hundreds of
megabytes per second to tmpfs.

### Parallel Sandbox Creation

Each build action's sandbox is fully independent. When the build tool executes
N actions in parallel (as determined by the DAG scheduler from B01), it creates
N sandboxes simultaneously. There is no lock contention because the sandboxes
share no state.

### Cache Amortization

The most important performance optimization is the content-addressable cache.
On a typical incremental build:

- 90%+ of actions hit the cache (inputs unchanged).
- Cache hits skip sandbox creation entirely.
- Only changed actions pay the copy cost.

For a clean build of the entire monorepo, the copy overhead is typically under
2 seconds — dominated by the actual compilation time, not file I/O.

### Benchmark Guidance

When evaluating sandbox performance, measure these independently:

| Phase              | Expected Cost           | Optimization             |
|--------------------|-------------------------|--------------------------|
| Sandbox creation   | < 10ms per action       | tmpfs, clonefile         |
| Input copying      | < 50ms for typical pkg  | clonefile, parallel I/O  |
| Command execution  | Varies by language      | Not sandbox-related      |
| Output capture     | < 20ms per action       | Content hash in parallel |
| Sandbox cleanup    | < 5ms per action        | os.RemoveAll             |

Total sandbox overhead per action: under 100ms. Compilation time for even a
small Elixir package is 500ms+. The sandbox is not the bottleneck.

---

## 12. Summary

The hermetic build sandbox is the foundation that makes every other build
system guarantee possible:

- **Reproducibility** comes from isolation: same inputs, same environment,
  same outputs.
- **Cacheability** comes from hermeticity: if the action hash matches, the
  outputs are identical.
- **Parallelism** comes from independence: sandboxes share nothing, so they
  can't interfere.
- **Correctness** comes from enforcement: undeclared dependencies are errors,
  not silent successes.

The copy-based approach is deliberately simple. It works on every platform,
requires no special privileges, and fails loudly when something is wrong.
Optimizations like clonefile, tmpfs, and namespace isolation are layered on
top — they make it faster and more secure, but the correctness guarantees come
from the copies alone.
