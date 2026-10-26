# mosaic-package-resolver

Component-reference resolver for Mosaic packages, implementing **UI29 §4.4**
(Mosaic Primitive Kernel — "Resolving a component reference").

The Mosaic compiler is told, by some user component's `.mll` file, that
*"the tag `Grid` appears here."*  The resolver answers exactly one question:

> *What is `Grid`?*

There are exactly three answers:

| Answer                 | Meaning                                                          |
|------------------------|------------------------------------------------------------------|
| `Resolution::Kernel`   | a UI29 §2.1 kernel primitive (`Box`, `Row`, `For`, `HostInput`…) |
| `Resolution::Component`| a component exported by a package this user depends on           |
| `None`                 | the tag is unknown — neither kernel nor any declared dependency  |

The third case is the compiler's cue to emit a *"no such tag"* error to the
user.  The first two are the cue to lower the tag (kernel) or recurse into
the package's compiled artifact (component).

## How it fits in the stack

```
                ┌──────────────────────────────────┐
                │  user-component .mll             │
                │    Grid (rows: slot: data, …)    │
                └──────────────┬───────────────────┘
                               │  "what is `Grid`?"
                               ▼
                ┌──────────────────────────────────┐
                │  mosaic-package-resolver         │  ← this crate
                │  (kernel set ∪ deps' exports)    │
                └──────────────┬───────────────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
        Kernel → backend emitter   Component → recurse into pkg-grid
```

The resolver consumes:

1. The user's package root (a directory containing a `mosaic-package.toml`).
   If no manifest exists, the resolver still answers kernel questions — it
   just has no component table to consult.
2. A **package search path**: a list of directories where Mosaic packages
   live.  In this repo that's typically `code/packages/`.

It produces a `Resolver` whose `resolve(tag)` is a fast `HashMap` lookup.

## Worked example

```rust
use std::path::PathBuf;
use mosaic_package_resolver::{build, Resolution};

let user_root = PathBuf::from("path/to/user/package");          // has its own mosaic-package.toml
let search    = vec![PathBuf::from("code/packages")];

let resolver = build(&user_root, &search).expect("resolver builds");

// Kernel primitive — always resolves.
assert!(matches!(resolver.resolve("Box"), Some(Resolution::Kernel)));

// Component declared by a dependency.
if let Some(Resolution::Component { package, component, .. }) = resolver.resolve("Grid") {
    println!("`Grid` lives in {package}, exported as {component}");
}

// Unknown tag.
assert!(resolver.resolve("Definitely-Not-A-Real-Tag").is_none());
```

## What the resolver detects as an error at *build* time

| Error                          | When it fires                                        |
|--------------------------------|------------------------------------------------------|
| `DependencyNotFound`           | a name in `[dependencies]` was not in any search path |
| `BadDependencyManifest`        | dependency's `mosaic-package.toml` failed to parse   |
| `VersionMismatch`              | dependency's `[package].version` fails the declared requirement |
| `DuplicateExport`              | two dependencies export the same component name     |
| `Io`                           | filesystem read error                                |

Unknown-tag-at-resolve-time is **not** an error — `resolve()` returns
`None` and lets the *compiler* decide what to do (typically: emit a
"no such component `Foo` (did you mean `Bar`?)" diagnostic).

## Version requirements

A `[dependencies]` value is checked, not ignored
(mosaic-package-publishing-v1 §7.4). A bare `"0.16.0"` is a caret
requirement (`>=0.16.0, <0.17.0`; `>=1.2.3, <2.0.0` from 1.0.0 on) and
`"=0.16.0"` is exact; `mosaic-package-manifest` refuses every other form.
The resolver compares the requirement with the `[package].version` of the
package it actually found, and fails on a miss:

```text
task-app requires mosaic-pkg-toolkit "0.15.0" (>=0.15.0, <0.16.0), but found
mosaic-pkg-toolkit 0.16.0 at …/mosaic-pkg-toolkit; help: update the
requirement in …/task-app/mosaic-package.toml
```

Both locators check:

- `build()` checks the consumer's direct dependencies
  (`ResolveError::VersionMismatch`).
- `LayoutPackageResolver` checks the `[dependencies]` of every package it
  inlines from, and `check_dependencies(&manifest, &path)` checks the
  consumer itself (`LayoutResolveError::VersionMismatch`). A declared
  dependency missing from the search path is not a version error there;
  a layout that references it still fails with `PackageNotFound`.

Lookup is by name, so a build holds one version of each package and every
requirer is checked against it. The `every_repository_requirement_is_satisfied`
test walks every `mosaic-package.toml` in the repository and runs both
checks, so a stale requirement fails this crate's tests with the file to
fix. (The build tool reruns those tests only when this crate or its
dependencies change; a package build reaches the same check through
`mosaic-package-artifact-builder`.)

## Kernel set

The kernel is frozen per UI29 §2.4.  This crate hard-codes it as
`KERNEL_PRIMITIVES`.  UI29 §2.1 lists 15 primitives; we also include
`Else` here because the `moslayout-compiler` tokenizes it as its own tag
even though UI29 treats it as a continuation of `If`.  See the comment on
the constant in `src/lib.rs` for details.

## Relationship to other crates

- **mosaic-package-manifest** (UI29-R1): the resolver depends on it to
  parse each dependency's `mosaic-package.toml`.
- **mosaic-compile** (future): will call `build()` once per user package
  and consult `resolve()` on every tag encountered by the moslayout AST
  walker.
- **mosaic-emit-react / -swiftui / -qt / …**: never see the resolver
  directly; they only see lowered kernel primitives or pre-compiled
  component references the resolver mediated.
