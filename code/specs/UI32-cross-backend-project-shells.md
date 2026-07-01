# UI32 — Cross-backend project-shell emission

> **Status.** Draft, gates the UI32-K-* per-backend implementation
> cycle.
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`); reuses the artifact-builder
> coverage groundwork from UI31-M (`code/specs/visicalc-cross-backend-demo-plan.md`).
>
> **Scope.** Generalises the `--emit-project` flag pioneered in the
> XAML backend (`code/packages/rust/mosaic-emit-xaml`, fix B1 / PR
> [#3917](https://github.com/adhithyan15/coding-adventures/pull/3917))
> to every Mosaic backend. After this spec lands, `mosaic-compile
> --backend <any> --emit-project` produces a complete, runnable
> host application alongside the component artifacts, with the
> exact project shape each platform expects.

---

## 1. Why this belongs in the kernel

A Mosaic-compiled component is **not runnable on its own**. The React
backend emits a `Component.tsx` file; the SwiftUI backend emits a
`Component.swift` file; the Qt backend emits a `Component.qml` file.
Each is a *fragment* — the host application that imports, mounts, and
event-binds the component is left to the user to hand-write.

This was a deliberate trade-off in UI29: the kernel ships portable UI
*pieces*, the host owns the application shell. But it has two real
costs we keep paying:

1. **Demo friction.** Every cross-backend demo
   (`code/programs/typescript/visicalc-{html,webcomp,flutter,qt,swiftui,xaml}/`) hand-
   writes the same `main.qml` / `MainWindow.xaml` / `lib/main.dart` /
   `Sources/.../App.swift` / `index.html` shell, then mounts the
   compiled component inside it. That hand-written shell is identical
   between demos for the same backend (Qt demo's main.cpp is structurally
   the same regardless of which component it mounts), and the diff
   between backends is *exactly* the boilerplate a kernel-level
   project emitter should absorb.
2. **First-render barrier for new authors.** A new Mosaic author
   compiling a Hello-World component cannot just run it. They have
   to learn `cargo init` + Vite + `flutter create` + CMake + WinUI
   project layout + xcrun + Swift Package Manager + a `bun create
   web-component` template — *just to see the component on a screen*.
   This is a real adoption tax.

XAML's PR [#3917](https://github.com/adhithyan15/coding-adventures/pull/3917)
proved out the pattern for WinUI 3: `mosaic-compile --backend xaml
--emit-project` produces seven files alongside `Component.xaml` —
`{Component}.csproj`, `App.xaml`, `App.xaml.cs`, `MainWindow.xaml`,
`MainWindow.xaml.cs`, `app.manifest`, `build.ps1` — plus a `README.md`
explaining how to build and run on Windows. The output is a literal
`dotnet build`-able project. The author types one command and gets
both their component and a way to see it run.

This spec generalises that pattern to every Mosaic backend.

---

## 2. The feature surface

### 2.1 CLI flag

`mosaic-compile` adds (or keeps, for XAML) a `--emit-project` boolean
flag. When set, the compiler emits the standard per-backend project
shell to the same output directory as the component artifact.

```sh
$ mosaic-compile --backend react --emit-project \
    --interface Hello.mil --layout Hello.mll --style Hello.msl \
    -o out/Hello.tsx

Written: out/Hello.tsx
Written: out/package.json
Written: out/vite.config.ts
Written: out/index.html
Written: out/src/main.tsx
Written: out/README.md
```

After this command the author can run `cd out && npm install && npm
run dev` and see Hello on `http://localhost:5173`.

### 2.2 Per-backend project shape

| Backend | Project files emitted | Run command |
|---|---|---|
| **React** | `package.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `README.md` | `npm install && npm run dev` |
| **HTML** | `index.html` (complete `<!DOCTYPE>` shell that inlines the component fragment), `README.md` | open `index.html` in a browser |
| **WebComponent** | `index.html` (loads the `.js` module + instantiates the custom element), `README.md` | open `index.html` in a browser (no build step) |
| **Flutter** | `pubspec.yaml`, `lib/main.dart` (MaterialApp shell mounting the component), `README.md` | `flutter pub get && flutter run` |
| **Qt** | `CMakeLists.txt`, `main.cpp` (loads the `.qml` as the root QML), `qmldir`, `README.md` | `cmake -B build && cmake --build build && ./build/<Component>` |
| **SwiftUI** | `Package.swift`, `Sources/App/App.swift` (App + WindowGroup shell), `README.md` | `swift run` |
| **XAML** | (already shipped) `{Component}.csproj`, `App.xaml`, `App.xaml.cs`, `MainWindow.xaml`, `MainWindow.xaml.cs`, `app.manifest`, `build.ps1`, `README.md` | `pwsh build.ps1` |

Notes:
- **HTML** is partially done by UI31-M (PR [#4219](https://github.com/adhithyan15/coding-adventures/pull/4219)) which writes `html/index-shell.html` from `mosaic-package-artifact-builder`. UI32 lifts that to `mosaic-compile --emit-project --backend html` and renames it to `index.html` (the bare manifest stays as `index-shell.html`'s sibling for backward compat).
- **WebComponent** doesn't need a build step in v1 — the emitted `.js` is a self-registering Custom Element; the `index.html` shell just `<script type="module" src="./Component.js"></script>` and `<mosaic-component></mosaic-component>`.
- **Qt** uses `qrc:` resource embedding so the QML lives inside the binary; the `qmldir` makes the component importable.
- **SwiftUI** uses Swift Package Manager (not Xcode .xcodeproj) for parity with the per-platform CLI workflow. iOS-specific scaffolds are out of scope.

### 2.3 Artifact-builder integration

`mosaic-package-artifact-builder` gains a parallel `emit_project: bool`
option on `BuildOptions`. When set, the per-component triple emission
is followed by a single project-shell pass that mounts every component
in a generated shell page. This is the multi-component cousin of the
mosaic-compile single-component path:

- React: `src/main.tsx` mounts the package's first component (or a
  selector UI if multiple — TBD in §6).
- HTML: `index.html` inlines all component fragments inside `<section
  data-component="X">` blocks (already shipped in #4219).
- WebComponent: `index.html` loads `index.js` and instantiates every
  `<mosaic-{name}>`.
- Flutter: `lib/main.dart` MaterialApp with a routes table per
  component.
- Qt: `main.qml` with a TabBar / ColumnLayout that hosts every
  component.
- SwiftUI: `Sources/App/App.swift` with a TabView over components.
- XAML: `MainWindow.xaml` with the existing pattern.

---

## 3. Non-negotiable contracts

The XAML implementation set the bar; every backend must meet it.

### 3.1 Reproducible

The project shell is **byte-for-byte deterministic** given the same
component interface + layout. No timestamps, no UUIDs, no random
filenames. CI must produce identical bytes regardless of build host.
(XAML's `build.ps1` and `app.manifest` already meet this; the
template-replacement for `{Component}` is the only varying input.)

### 3.2 Runnable

The emitted project must build and launch on a freshly-provisioned
target machine with only the documented toolchain installed
(documented in the per-backend `README.md`). No hand-editing required
between `mosaic-compile --emit-project` and `<run command>`.

### 3.3 Minimal

The shell is the **smallest** possible runnable host. No theming
beyond OS-default. No analytics. No accessibility-violating defaults.
No third-party UI libraries (no chakra-ui in the React shell, no
provider in the Flutter shell). The shell exists to demonstrate the
component, not to be a starter kit. Authors who want fancier framing
fork the shell.

### 3.4 Composable with `--emit-project` flag absent

A bare `mosaic-compile --backend react -o Hello.tsx` (no
`--emit-project`) still emits ONLY `Hello.tsx` — no project files.
The flag is opt-in. Existing CI and downstream tools that rely on the
single-file output continue to work unchanged.

### 3.5 Side-files are self-contained and overwrite-on-re-emit

Each emitted side-file lists its dependencies in a header comment so
a human reading `App.swift` knows it pairs with `Package.swift`. If
the user deletes one accidentally, the next `--emit-project` rebuild
regenerates it without error.

**Re-emit semantics.** `--emit-project` always **overwrites** existing
side-files in the output directory (matches XAML's existing
behaviour, required by §3.1 determinism). To make this safe every
emitted file starts with:

```
<comment-syntax-for-this-file-type> AUTO-GENERATED by mosaic-compile
<comment-syntax> --emit-project. Edits will be overwritten on next emit.
<comment-syntax> Fork the file (remove this banner) to customise.
```

The `README.md` of each shell carries the same warning in prose.
Authors who want a long-lived hand-edited shell strip the banner and
move the file out of the regenerable set; the next `--emit-project`
run will recreate the original at its original name (overwrite a
forked file at a different name is not possible).

### 3.6 Substitution + dependency hygiene

This contract consolidates the security-relevant invariants the six
parallel L2–L7 PRs must NOT re-decide.

**3.6.1 Template substitution boundary.** Every interpolation of an
author-controlled name (`{Component}`, `{Package}`, `{Namespace}`,
`{ModuleName}`, …) into an emitted file MUST flow through the
shared validator pipeline:

1. `mosaic-package-artifact-builder::validate_component_name`
   (`[A-Za-z][A-Za-z0-9_]*` — ASCII identifier shape).
2. `mosaic-package-artifact-builder::validate_package_name`
   (kebab-case for npm/Cargo-style names).
3. Per-backend **identifier-shape** validators (3.6.2 below).

Emitters MUST NOT accept a raw `&str` name into template
substitution; the type signature MUST require a previously-validated
wrapper or call the validator inline. Failing the validator MUST
return an error from the emitter (not silently substitute or strip).

**3.6.2 Per-backend identifier-shape constraints.** The upstream
ASCII-identifier regex is necessary but not sufficient for every
emitted context. Each L2–L7 PR MUST implement and test the
backend-specific stricter rules:

| Backend | Constraint | Rationale |
|---|---|---|
| React | `package.json` `"name"` field: lower-case, `≤214` chars, no leading `.` or `_`, URL-safe (npm RFC). Derived from `[package].name`, NOT `{Component}`. | npm rejects PascalCase names. |
| Flutter | `pubspec.yaml` `name:` field: `[a-z][a-z0-9_]*` (Dart pub rules). Derived from `[package].name` (kebab→snake), NOT `{Component}`. | `flutter pub get` rejects PascalCase. |
| SwiftUI | `Package.swift` target names + `App` identifier: ASCII letters/digits/underscore, MUST NOT collide with Swift keywords (`Actor`, `Class`, `Protocol`, `Self`, etc.). Implement as a reject-list of ~20 keywords; suggest a `_` suffix on collision. | Swift compiler errors without backtick-quoting. |
| Qt | `qmldir` `module` line: PascalCase per QML rules. Use the existing `qmldir_module_name` helper from `mosaic-package-artifact-builder` (#4006). CMake target names: `-` rejected (validator already excludes). | QML module resolution. |
| XAML | `csproj <RootNamespace>`: reject C# keywords (`class`, `namespace`, `interface`, etc.). | C# compiler errors without `@`-prefix. |
| HTML | None beyond ASCII (identifier appears only in HTML comments + `data-` attrs). | HTML is permissive. |
| WebComponent | Custom Element name: lower-case with at least one `-` (HTML spec requires hyphen). Derived as `mosaic-{kebab(Component)}`, NOT `{Component}` directly. | `customElements.define` rejects names without `-`. |

**3.6.3 Supply-chain pinning.** Every emitted dependency manifest
(`package.json`, `pubspec.yaml`, `Package.swift`, `CMakeLists.txt`
`FetchContent_*`, `.csproj` `<PackageReference>`) MUST pin to a
specific known-good version. The pinned versions live in a single
constants module per emitter (XAML's `EmitOptions::windows_app_sdk`
defaulting to `"1.7.250606001"` is the reference pattern; mirror
that as `pinned_react: &str`, `pinned_flutter_sdk: &str`,
`pinned_vite: &str`, etc.). Forbidden forms:

| Forbidden | Why |
|---|---|
| `"react": "*"` | Pulls latest, including breaking changes + compromised versions. |
| `"react": "latest"` | Same. |
| `"react": ">=18"` | Unbounded upper. |
| `"react": "^18"` without lockfile | Patch drift between authors. |

Required: pinned exact (`"react": "18.3.1"`) **or** caret-pinned
(`"react": "^18.3.1"`) **with a generated lockfile**
(`package-lock.json`, `pubspec.lock`, `Package.resolved`)
committed by the emitter alongside the manifest. Lockfile emission
is required, not optional.

Each L2–L7 PR MUST document its pinning policy in the per-backend
README and surface the pinned constants for review.

Lockfiles MUST be **pre-computed and vendored** alongside the pinned
constants (shelling out to `npm install` / `pub get` / `swift
package resolve` at emission time violates §3.8). Practically: each
emitter ships a static `pinned_<backend>_lockfile: &str` constant
generated by a separate one-shot regeneration script
(`scripts/regen-pinned-lockfiles.sh`) that the emitter authors
re-run when bumping a pinned version. The regeneration script is
NOT part of the emit-time path.

**3.6.4 Shell-script quoting.** XAML emits `build.ps1`; future
backends may emit `*.sh` / `*.bat` / `*.cmd`. Shell-script emission
MUST quote every interpolated name and SHOULD prefer compile-time
constants over string interpolation (template the shell to use a
project-relative path, not the component name). Per-PR test:
feed the emitter a name with every byte the validator allows
(`"AAA_111"`); assert the script parses under `pwsh -NoLogo -NoProfile
-Command "$null = [scriptblock]::Create((Get-Content build.ps1
-Raw))"` (PowerShell parse-only) or `bash -n build.sh`.

### 3.7 Output-directory write contract

The emitter writes **only** into the `-o` parent directory. Specifically:

- No `../` path traversal in any emitted file's relative path.
- No absolute paths in any emitted file's relative path.
- The set of relative paths each backend writes is **fixed and
  enumerated** in §2.2 above. No dynamic path construction from
  `{Component}` substitutions.
- Existing files outside that fixed enumerated set are never touched
  (the emitter does not delete unrecognised files; that is the user's
  responsibility).

Per-PR test: emit into a tmpdir; assert the set of created files
equals exactly the §2.2 enumeration; assert no created path begins
with `..`, `/`, or `~`.

### 3.8 No environment reads at emission time

The emitter MUST be a pure function of (component interface, layout,
style, `EmitOptions`). It MUST NOT read from `$HOME`, `$USER`,
`$PWD`, `$LANG`, system clocks, random sources, or network. This
makes §3.1 determinism testable (`cmp` two emit runs from different
hosts) and makes tampering detectable.

---

## 4. Implementation plan

One PR per backend. Each PR follows the XAML #3917 template:

1. Extend the backend's `pipeline::EmitOptions` (or equivalent) with
   an `emit_project: bool` field. Default `false`.
2. Add a `Project { ... }` struct holding the emitted files (one
   `String` per file).
3. When `emit_project` is on, populate the struct alongside the
   component result.
4. `mosaic-compile`'s match arm writes the side files when
   `result.project.is_some()`.
5. Tests cover: emitted file names + presence, deterministic shell
   output, runnable shape (file headers, root tags), back-compat
   (no `--emit-project` = no side files).

Sequential order (mirrors UI31 K1–K7 cadence):

- **[L1]** This spec.
- **[L2] UI32-K-react** — Vite project shell. Branch `feat/ui32-k-react`.
- **[L3] UI32-K-html** — Standalone `<!DOCTYPE>` document. Branch `feat/ui32-k-html`. Builds on #4219.
- **[L4] UI32-K-webcomp** — `index.html` + Custom Element instantiation. Branch `feat/ui32-k-webcomp`.
- **[L5] UI32-K-flutter** — `flutter create`-shaped scaffold. Branch `feat/ui32-k-flutter`.
- **[L6] UI32-K-qt** — CMakeLists + main.cpp + qrc. Branch `feat/ui32-k-qt`.
- **[L7] UI32-K-swiftui** — Swift Package Manager scaffold. Branch `feat/ui32-k-swiftui`.
- **[L8] UI32-M** — multi-component artifact-builder integration (lifts UI31-M #4219 from HTML-only to all 7 backends).

### Pre-push gates per item (mirrors UI31's gate format)

- `cargo build -p mosaic-emit-<backend>` clean
- `cargo test -p mosaic-emit-<backend>` passing with new shell tests
- Security review on the diff — particularly the
  template-replacement boundary (`{Component}` substitution must
  use the same name-validation path the artifact-builder uses; an
  unvalidated component name landing in a `.csproj` `<RootNamespace>`
  could escape the file)
- A "runnable" test that compiles the shell on the CI runner where
  the toolchain is available (Windows runner for XAML, macOS runner
  for SwiftUI, etc.) — optional in v1; required in a follow-up

---

## 5. Open questions

1. **Multi-component shell — which component is the "root"?**
   For a package with `{Grid, FormulaBar, Cell}` and `--emit-project`,
   does the React shell mount `Grid` because it's the first export?
   Mount all three side-by-side? Read a `[default-component]` field
   from `mosaic-package.toml`? Recommend: read a manifest field,
   default to first export. (Same answer applies to all 7 backends.)
2. **Output-directory layout — flat or nested?**
   XAML emits everything flat in the output directory. React would
   want `src/main.tsx` nested per Vite convention. Flutter wants
   `lib/main.dart` per `flutter create` convention. Recommend: respect
   each platform's idiom; per-backend project struct decides.
3. **`--emit-project` + `--output` interaction.** If
   `-o out/Hello.tsx` is given, does `--emit-project` emit alongside
   in `out/`? Recommend: yes, alongside (matches XAML behaviour).
4. **`mosaic-package.toml` extension for project metadata.**
   Authors may want to set the app title, window size, default
   theme. v1 punts; defaults come from the component name.
5. **WebComponent without a bundler.** v1 ships the `.js` directly
   via `<script type="module">`, no build step. Some hosts may want
   a Rollup/Vite bundle. Punt to a v2 `--emit-project=bundled` mode.
6. **VC2-* demos.** Once the per-backend project emitter ships,
   the hand-written demos can switch to invoking
   `mosaic-compile --emit-project` and deleting the hand-written
   shells. Tracked as UI32-N (one PR per demo, six total).

---

## 6. Out of scope

- iOS / Android-specific scaffolds for Flutter / SwiftUI (desktop
  only in v1).
- Hot-reload integration (Vite handles it for free; other backends
  punt).
- Multi-target compilation (one backend per `--emit-project` call).
- Test scaffolding inside the project (no Jest / XCTest / pytest
  setup; the shell is for `--dev`-running the component, not for
  testing it).
- Custom branding / theme tokens in the shell (the shell is OS-
  default).
