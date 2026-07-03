# mosaic-dev — Storybook-like dev environment for Mosaic components

`mosaic-dev` is a single-command preview tool for Mosaic packages.  You
point it at a package directory, pick a backend, name one component, and
it boots that backend's runtime against a temp-dir build of your
package.  As you edit the source files (`.mil` / `.mll` / `.msl`),
`mosaic-dev` re-runs the package compiler and the runtime picks up the
new artifacts.

It is the "Storybook" of the Mosaic stack: an authoring loop with
zero per-component boilerplate.  All the host-wrapper code that would
otherwise live in a `code/programs/typescript/visicalc/`-style app is auto-generated from
your component's `.mil` interface.

## Synopsis

```bash
mosaic-dev <PACKAGE_ROOT> \
    --backend <react|swiftui|qt|webcomponent|html|xaml> \
    --component <NAME> \
    [--port 5173] \
    [--no-open]
```

Arguments:

| Flag            | Required | Meaning                                                  |
|-----------------|----------|----------------------------------------------------------|
| `PACKAGE_ROOT`  | yes      | Directory containing `mosaic-package.toml`.              |
| `--backend`     | yes      | Which runtime to spawn (see *Backends* below).           |
| `--component`   | yes      | The exported component name to preview.                  |
| `--port`        | no       | HTTP port for web backends (default `5173`).             |
| `--no-open`     | no       | Don't auto-open the browser after the runtime starts.    |

## Example

```bash
mosaic-dev code/packages/mosaic/mosaic-pkg-grid \
    --backend react \
    --component Grid
```

This will:

1. Read `code/packages/mosaic/mosaic-pkg-grid/mosaic-package.toml`.
2. Build the package for React into a temp dir (`<tmp>/react/Grid.tsx`,
   etc.).
3. Auto-generate `<tmp>/main.tsx` and `<tmp>/index.html` that mount the
   `<Grid>` component with dummy props derived from `Grid.mil`.
4. Launch `npx vite` against the temp dir on port `5173`.
5. Open `http://127.0.0.1:5173` in your browser.
6. Watch `code/packages/mosaic/mosaic-pkg-grid/src/` and re-build on every
   `.mil` / `.mll` / `.msl` change.

## Backends

| Backend          | Runtime spawned                  | Update mode                       | Platform notes |
|------------------|----------------------------------|-----------------------------------|----------------|
| `react`          | `npx vite`                       | Vite HMR (no restart)             | Requires `npx`, internet on first run |
| `html`           | Built-in `tiny_http` server      | Manual refresh                    | None |
| `webcomponent`   | Built-in `tiny_http` server      | Manual refresh                    | None |
| `swiftui`        | `swift run` (SwiftPM)            | Process restart                   | macOS / Linux; Swift 5.9+ |
| `qt`             | `qmlscene`                       | Process restart                   | Qt 5/6 dev tools on `PATH` |
| `xaml`           | _(not yet supported)_            | —                                 | Windows-only; planned for a follow-up PR |

## Dummy-prop generation

Every slot in your `.mil` file gets a sensible placeholder value:

| Mosaic slot type | Dummy value                       |
|------------------|-----------------------------------|
| `text`           | the slot name as a string         |
| `number`         | `0`                               |
| `bool`           | `false`                           |
| `image`          | `""` (empty URL)                  |
| `color`          | `"#cccccc"` (neutral grey)        |
| `node`           | `null`                            |
| `list<T>`        | `[]` (empty list)                 |
| `Component(_)`   | `null`                            |

If a slot declares an inline default (e.g. `slot disabled : bool = false ;`),
the default value wins — that's what the host would see if it omitted the
prop.

## File watching

`mosaic-dev` uses the [`notify`] crate (which routes to FSEvents on
macOS, inotify on Linux, and ReadDirectoryChangesW on Windows) to watch
`<PACKAGE_ROOT>/src/`.  Changes to `.mil` / `.mll` / `.msl` files
trigger a rebuild after a 100ms debounce; other extensions are ignored.

For SwiftUI and Qt — neither of which has HMR — a successful rebuild
kills the running process with `SIGTERM` and respawns it.  Vite handles
its own watching for the React backend; `tiny_http` just serves static
files for HTML / WebComponent and you'll need to refresh the page
yourself.

## Platform requirements

The tool itself is pure Rust and has no runtime dependencies beyond the
crates in `Cargo.toml`.  Each backend has its own external dependency:

- **React**: a working `npx` and internet access on first run (so Vite
  can be fetched).
- **SwiftUI**: Swift 5.9 or newer with SwiftPM (`swift run`).
- **Qt**: Qt 5 or Qt 6 dev tools providing the `qmlscene` binary on
  `PATH`.
- **HTML / WebComponent**: none — the tool serves everything from an
  embedded `tiny_http` server.

## What this v0.1.0 cannot do (yet)

The following are intentional follow-ups, not bugs:

1. **Real HMR for native backends.** SwiftUI and Qt full-restart on
   every change.
2. **Browser auto-refresh for HTML / WebComponent.** SSE-based
   auto-reload is a separate PR.
3. **Multiple components per session.** One `--component` per
   `mosaic-dev` invocation.
4. **Custom dummy-prop overrides.** A `.mosaic-dev.toml` config file is
   planned.
5. **XAML.** Requires a Windows host and the dotnet/MSBuild
   integration.

## How it fits the stack

```text
┌───────────────────────────┐
│  mosaic-dev (this crate)  │  ← Storybook-shaped dev runner
└─────────────┬─────────────┘
              │ delegates to
              ▼
┌──────────────────────────────────────┐
│ mosaic-package-artifact-builder      │  ← .mil/.mll/.msl → backend dir
└─────────────┬────────────────────────┘
              │ delegates to
              ▼
┌──────────────────────────────────────┐
│ mosaic-emit-{react,swiftui,qt,...}   │  ← per-backend lowering
└──────────────────────────────────────┘
```

The package builder does the actual lowering; `mosaic-dev` is a thin
shell that loops the builder over file-system events and spawns the
right runtime.

[`notify`]: https://docs.rs/notify
