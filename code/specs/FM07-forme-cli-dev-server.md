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
| `forme deploy` | Pending | FM-B012 composes the FM08 deploy runner. |
| `forme install` and trust UX | Blocked | Requires the FM02 plugin host and FM-B014/FM-B015. |
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

`forme build [--config PATH] [--reproducible]` loads one project config,
executes it, emits a stable report, and exits non-zero on fatal diagnostics.
`forme run` accepts the same arguments and behavior.

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

### 2.5 Future commands

`forme deploy` belongs to [FM08](FM08-forme-deploy-runner.md) and FM-B012.
Plugin installation and persisted trust decisions belong to
[FM02](FM02-forme-plugin-host.md). Their absence must be reported explicitly;
the CLI must not imply that an unavailable security boundary exists.

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
- [FM08](FM08-forme-deploy-runner.md) — future deploy command implementation
