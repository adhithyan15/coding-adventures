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
| `DuplicateExport`              | two dependencies export the same component name     |
| `Io`                           | filesystem read error                                |

Unknown-tag-at-resolve-time is **not** an error — `resolve()` returns
`None` and lets the *compiler* decide what to do (typically: emit a
"no such component `Foo` (did you mean `Bar`?)" diagnostic).

## Inlining `pkg::P::C` call sites: required slots

`LayoutPackageResolver` replaces each `pkg::P::C ( … )` node in a compiled
layout with `C`'s own layout, rewriting `C`'s `slot:` references to the
values bound at the call site (UI34 §5.1).

Before it substitutes anything, it checks every call site against `C`'s
interface. Each slot `C.mil` declares without a default must be bound
(UI13 §1). Otherwise the result is `SlotBindingMissing` (UI34 §7):

```
// mosaic-pkg-lib/src/Card.mil       // ProbeApp.mll
component Card {                     layout ProbeApp { Column [ app-root ] {
  slot title : text ;                  pkg::mosaic-pkg-lib::Card ( title: slot: heading )
  slot hint  : text ;      ← required } }
  slot tone  : text = "" ; ← optional
}

`ProbeApp` mounts `pkg::mosaic-pkg-lib::Card` at Column[app-root] >
pkg::mosaic-pkg-lib::Card#1 without binding its required slot `hint`;
bind `hint` at the call site, or give it a default in Card.mil
(`slot hint : <type> = <value> ;`)
```

- An omitted slot with a default takes the default.
- `node` and `list<node>` slots are filled by the caller's children, so they
  need no binding.
- An emit the call site does not forward is dropped.
- Every miss in the whole tree is reported at once, as
  `LayoutResolveError::Multiple`. That includes misses inside the layouts
  of the packages it reaches.

The check reads the interface rather than the layout, because an unbound
name that survives substitution means a slot of the *consumer*. It was
then either silently captured by a consumer slot with the same name, or
left in generated code as an undeclared identifier.
(`mosaic-unbound-slot-semantics-v1.md` §1.3 has the probes.)

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
