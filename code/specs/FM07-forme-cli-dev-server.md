# FM07 — Forme CLI and Development Server

> **Status:** Headless v0 implemented; extensible and authoring commands pending.
> **Scope:** User-facing command semantics, configuration loading, diagnostics,
> watch mode, preview serving, cancellation, and command composition.
> **Packages:** `forme-cli` and `forme-dev-server`.

## Implementation status

| Surface | Status | Evidence / next step |
|---|---|---|
| `forme build` and `forme run` alias | Implemented | Both live sites build through `forme-cli`. |
| `forme check` | Implemented | Loads and validates configuration without running the pipeline. |
| `forme clean` | Implemented | Removes only containment-checked output and cache targets. |
| `forme watch` preview server | Implemented | Coalesced rebuilds, SSE reload, last-good output, and clean cancellation are tested. |
| `forme deploy` | Implemented in FM-B047/FM-B012 | Uses CLI Builder for one manifest, exactly one content-store shape, one explicit target config, target-aware dry-run, and publication through the FM08 adapters. |
| `forme install` and trust UX | Implemented in FM-B056 | Composes the completed FM02 authority and atomic-installer cores for bounded local package directories. |
| Installed plugin runtime | Implemented in FM-B057 | Manifest-bound grants, language selection, and native platform sandboxes are composed into CLI/orchestrator execution; FM-B058 closes live storage-watch mediation. |
| Authoring shell integration | Blocked | FM-B016 owns the non-developer product shell. |

## 1. Purpose

FM07 is the canonical specification location for the already implemented
headless CLI and preview server. `forme build` is the preferred static-site
vocabulary; `forme run` is an exact compatibility alias retained for older
FM03 examples and scripts.

The CLI is a product adapter around [FM03](FM03-forme-orchestrator.md). It may
load configuration, select commands, format diagnostics, manage signals, and
compose explicitly authorized services. It must not reimplement DAG execution,
cache policy, or stage semantics.

## 2. Commands

### 2.1 Build and compatibility run

`forme build [--config PATH] [--reproducible] [--deploy-input DIR]` loads one project config,
executes it, emits a stable report, and exits non-zero on fatal diagnostics.
`forme run` accepts the same arguments and behavior.

When `--deploy-input DIR` is present, a successful build merges the exact named
`dist-tree` outputs into one collision-checked complete set, writes one
digest-keyed content store, and writes its strict deploy manifest last. The
directory must remain beneath the selected project root. No implicit scan of a
stage output directory may substitute for these in-memory artifacts.

### 2.2 Check

`forme check [--config PATH]` resolves the configuration and validates the
typed DAG without invoking stages or mutating project output.

### 2.3 Clean

`forme clean [--config PATH]` considers only declared direct-stage
`DeployArtifact` output directories plus `settings.cacheDir`. Targets must be
inside the project, must not equal the project root, and are deduplicated before
removal.

### 2.4 Watch and preview

`forme watch [--config PATH]` runs an initial build, coalesces project changes,
and serves only the last successful in-memory artifact set. Successful rebuilds
notify browsers over server-sent events; failed rebuilds retain the last good
site. SIGINT cancels the build, watcher, and server without orphaned handles.

### 2.5 Deploy

`forme deploy` accepts the FM08 manifest, exactly one of directory, canonical
bundle, or explicit-descriptor inline content, a target and strict target
configuration, and optional previous-manifest, dry-run, bootstrap, retry, and
report inputs. CLI Builder owns parsing, required flags, and the exclusive
content-store group. The command never loads a project config or changes the
invocation working directory.

### 2.6 Install

`forme install <package-directory>` accepts one local directory as the v1
registry adapter. It loads the selected project config, snapshots only bounded
regular files, reads the user trust store, and resolves capability templates
against the same configured storage/cache roots used at runtime. A side-effect-
free installer preflight validates and classifies the exact snapshot before the
CLI displays every required and optional capability with the plugin identity,
trust tier, manifest reason, and a sensitive marker where applicable. Terminal
controls in package-authored text are escaped. Required capabilities have no
default; optional capabilities default to deny. Every grant therefore comes
from an explicit interactive answer, not a bypass flag. The complete reviewed
snapshot is committed by the FM02 atomic installer beneath the current
project's `forme-plugins/` discovery root.

The source tree and install root must be canonical directories. Symlinks,
non-regular entries, racing file identities, unsafe install-root permissions,
and malformed authority files fail closed. On Windows the command remains
unavailable unless FM-B057's native verifier proves that the complete root or
existing target tree excludes untrusted writers.

### 2.7 Installed plugin runtime composition

Pipelines containing only direct first-party stages retain the existing
process-free path. When a pipeline contains a plugin `StageRef`, the CLI opens
the canonical `<project>/forme-plugins` discovery root and loads only the
current manifest-bound grants stored beside each installed plugin. Missing or
stale grants grant nothing; malformed authority data is fatal.

The CLI selects the native sandbox factory for the current operating system;
there is no ambient subprocess fallback. `node` plugins use the current
canonical Node executable, `binary` plugins execute the exact selected entry,
and `deno`, `bun`, and `python` require explicitly configured absolute runtime
executables and roots in `settings.pluginRuntimes`. The product host supplies
the bounded storage adapter and the grant-mediated network, environment, and
broad user-filesystem adapters defined by FM01; third-party shell execution
remains forbidden. Host-owned plugin installation and cache roots MUST be
reserved from storage and broad filesystem adapters even when the configured
storage root is the project root. The resulting plugin host is passed to the
orchestrator and disposed on every success, failure, and cancellation path.

A mixed direct/`StageRef` DAG must validate without launching a subprocess and
must launch the referenced runner lazily for build and watch. Platform product
tests execute the same mixed pipeline through the real Linux, macOS, or Windows
sandbox instead of a process-free test double.

On Windows, a native ACL verifier canonicalizes the install root and existing
target tree without following reparse points, proves that only the current user
or trusted administrators retain write authority, and binds its result to the
opened identities. Inherit-only write rules are rejected on the install root
where transaction children are created, while every ancestor through the
volume root is checked for effective replacement authority. Product discovery
rechecks the root identity after the cancellable host snapshot pass. Any
unprovable owner, DACL, inheritance, ancestor, cancellation, or identity change
fails closed before installation or launch.

## 3. Diagnostics and reproducibility

Machine-readable reports use canonical recursive key ordering so live and
checkpoint-restored values serialize identically. Human output may add
presentation, but command names, exit codes, diagnostic codes, artifact hashes,
and build IDs remain stable inputs to automation.

## 4. Related specifications

- [FM01](FM01-forme-kernel.md) — diagnostics, cancellation, and capabilities
- [FM02](FM02-forme-plugin-host.md) — future installation and trust commands
- [FM03](FM03-forme-orchestrator.md) — pipeline execution contract
- [FM06](FM06-forme-aot-compiler.md) — static artifact production
- [FM08](FM08-forme-deploy-runner.md) — deploy content, adapter, bootstrap, and publication contract
