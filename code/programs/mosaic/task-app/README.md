# task-app

A to-do app with **automatic scheduling**, built entirely on the shared `task-core`
engine. Add tasks (with optional due dates), complete or delete them — and the engine
auto-schedules them into a working-day timeline via the Critical Path Method.

## Architecture

The UI is authored **once in Mosaic**. The web host wires emitted React to
`task-core` through `task-wasm`, retaining idiomatic React state. Generated native
hosts load `task-mosaic-app`, a standard-ABI adapter that owns portable presentation
state and calls the same typed `task-core` operations and projections. The adapter is
not a second task engine: domain validation, scheduling, and task/project invariants
remain in `task-core`.

The two adapters consume the same data-driven presentation lifecycle in
`fixtures/presentation-contract-v1.json`. The web test runs it through the real
`task-wasm` module, and the native test runs it through `task-mosaic-app`, comparing
canonical engine state and user-visible core slots after every step. See
`code/specs/task-app-presentation-contract-v1.md` for coverage and the two explicit
host-only exclusions (theme storage and locale-formatted calendar copy).

```text
TaskApp.mil / .mll / .msl        (Mosaic: interface / layout / style)
        │  mosaic-compile --backend react   (emits ONE component into host/web/src)
        ▼
   host/web/src/TaskApp.tsx       (generated React component: { ...slotProps, dispatch })
        │  host/web/src/main.tsx wires it to…
        ▼
   createTaskEngine (task-wasm)  →  task-core (the pure Rust engine) via WASM
        │  host/web/src/persistence.ts saves/restores via…
        ▼
   @coding-adventures/storage    →  IndexedDB (in-memory fallback)
```

The native path is:

```text
TaskApp.mil / .mll / .msl
        │  mosaic-compile --backend <native> [--profile native-complete]
        ▼
generated native UI + standard host binding
        │  bundled mosaic-app C ABI
        ▼
task-mosaic-app (MIL slots/events + presentation state) → task-core
```

Qt, Flutter, Compose Desktop, and SwiftUI on macOS are gated on this concrete
engine. CI requires zero degradations, builds the generated native project,
verifies the bundled library byte-for-byte, and launches the installed app without
an injected runtime path. The ABI conformance fixture remains a separate gate, so
a passing TaskApp launch cannot mask a regression in the standard host binding.
Task-specific emitted-control acceptance additionally drives the same simple-todo
lifecycle through the generated UI: create with an optional due date, reveal the
Rust schedule, complete and reopen, delete, reject malformed input atomically, and
restore a persisted task in a second process. Compose acceptance also requires the
Rust-owned `100%` completion progress to remain displayed in the default desktop
viewport rather than merely existing in an off-screen semantics tree.
The generated SwiftUI sources also compile for the iOS 16 deployment target; that
gate is source portability rather than a claim that a macOS dylib can run on iOS.

XAML/WinUI also bundles the concrete adapter and verifies it byte-for-byte beside
`TaskApp.exe`. A task-specific console fixture drives startup props and a semantic
event lifecycle plus restart restoration through the generated .NET binding
without an injected path. Its strict
`native-complete` build has zero degradations: the canonical Sheet exposes native
UI Automation table semantics, while board and calendar interactions use native
WinUI pointer/touch drag/drop plus an accessible keyboard path. GitHub-hosted Windows workers
do not provide a reliable
interactive desktop, so visible WinUI launch is deliberately reserved for a local
or self-hosted interactive Windows gate.

## What it does

- Add a task (name + optional `YYYY-MM-DD` due date).
- Tasks are chained into a work queue and **auto-scheduled** — each starts when the
  previous finishes, on working days (weekends skipped), with a projected finish date.
- Tasks scheduled to finish after their due date are flagged **overdue**.
- Click a row to complete it (✓); Delete to remove it.
- **Everything persists** — the whole workspace is saved to IndexedDB after each change
  and restored on reload (see `host/web/`); generated native hosts use their
  platform application-data directory and atomically replace their snapshot after
  each successful event.

Everything above runs on the pure Rust engine — the browser only holds UI state,
persists snapshots, and calls the engine's operations/queries.

## Build & run

```bash
scripts/build-web.sh                            # build wasm + emit TaskApp.tsx into host/web
cd host/web && npm install && npm run dev       # http://localhost:5173
```

## Incremental releases

TaskApp uses product-scoped [Semantic Versioning](https://semver.org/):
`task-app-vMAJOR.MINOR.PATCH`. Compatible fixes and packaging improvements bump
the patch version, new usable capabilities bump the minor version, and breaking
compatibility changes bump the major version. Tags and releases are immutable;
an existing version is never reused or overwritten.

From the repository's **Actions** tab, run **Release TaskApp** from `main` and
provide both the bare version and its matching product tag. The equivalent CLI
command for the first release is:

```bash
gh workflow run release-task-app.yml --ref main \
  -f version=0.1.0 \
  -f tag=task-app-v0.1.0
```

The workflow rejects invalid, mismatched, or previously published identifiers
before it builds artifacts. Release-relevant pull requests run the same artifact
matrix with a reserved CI-only SemVer identity, while the publisher remains
manual-`main`-dispatch-only. The lane tests the Rust/WASM web inputs and production Vite
bundle, generates every native project under the strict `native-complete` profile
with the platform's real `task-mosaic-app` runtime, and checks each emitted-control
contract. On Linux it also builds Qt, Flutter, and Compose Desktop release trees,
compares their installed Rust runtime byte-for-byte with the selected build, and
launches the archived bundle from `/` without a runtime override. On macOS it
assembles the release-runner architecture into an unsigned `Trestle.app`, validates
its stable metadata and bundled dylib, and checks state restoration across a
replacement-style second extraction. One publisher job then creates checksums, a
source-commit manifest, product-scoped notes from merged `task-app` pull requests,
and one GitHub Release. On Windows it publishes a self-contained x64 folder,
drives the original and replacement `Trestle.exe` copies through UI Automation,
and verifies the stable LocalApplicationData state plus console binding contract.

Release payloads distinguish directly runnable archives from generated projects:

| Artifact | Platform | What it is |
| --- | --- | --- |
| Web ZIP | Modern browsers | Tested, ready-to-serve production bundle |
| Qt, Flutter, Compose TAR.GZ files | Linux x86_64 | Verified portable bundles with a `launch-trestle` entrypoint |
| Qt, Flutter, Compose ZIPs | Linux x86_64 | Native-complete generated projects with the Rust runtime |
| Trestle.app ZIP | macOS 13+ | Verified unsigned SwiftUI app for the architecture recorded in `BUNDLE.json` |
| SwiftUI ZIP | macOS | Native-complete generated project with the Rust runtime |
| Trestle.exe ZIP | Windows 10 2004+ / x64 | Verified self-contained portable app; unsigned and unpackaged |
| XAML ZIP | Windows | Native-complete generated WinUI project with the Rust runtime |

The Linux archives are unpack-and-run bundles for compatible x86_64 systems, not
signed distribution packages. Each includes exact prerequisites and a launcher
that takes one version-specific pre-upgrade snapshot of existing local state. The
macOS app is neither Developer ID signed nor notarized, and its bundled dylib is
not an iOS artifact. The Windows folder is self-contained but unsigned and is not
an MSIX installer. Signing and installation lifecycle work remains tracked under
[#13522](https://github.com/adhithyan15/coding-adventures/issues/13522), so release
notes never imply broader coverage. To cut the next release, first move the relevant
entries into this changelog's version section, choose the SemVer bump from the policy
above, and dispatch the workflow with a new matching version and tag.

## Files

- `src/TaskApp.{mil,mll,light.msl}` — the Mosaic UI (interface, layout, style).
- `host/web/` — the committed web-host npm package (`src/main.tsx`, `src/persistence.ts`,
  its own `package.json`/tests); see `host/web/README.md`.
- `../../../packages/rust/task-mosaic-app/` — native standard-ABI application adapter.
- `mosaic-package.toml` — the package manifest.
- `tests/package_compiles.rs` — `cargo test` verifies the Mosaic sources compile.
- `scripts/build-web.{sh,ps1}` — build wasm + emit the component into `host/web`.
