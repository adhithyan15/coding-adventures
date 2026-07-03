# mosaic-pkg-card

The **simplest possible** Mosaic userland component package — a one-component
package that proves the UI29 userland-package architecture works end-to-end
TODAY, using only kernel primitives that have been stable in every backend
since UI29 landed.

## What it is

A single `Card` component:

```text
Card
├─ title   (text)
├─ body    (text)
└─ footer  (text)
```

No events.  No loops.  No conditionals.  No host tables.  Just three text
slots, laid out as a titled panel and themed with a dark stylesheet.

## What it proves

The Mosaic project ships kernel primitives (`Box`, `Column`, `Row`, `Text`,
`HostInput`, `If`, `For`, `HostTable`, …) and a three-language frontend
(`.mil` for component interfaces, `.mll` for layout, `.msl` for style).  The
UI29 spec promises that **userland packages can be authored on top of those
primitives** without touching emitter code in any backend.

`mosaic-pkg-card` is the smallest possible demonstration of that promise:

| Layer                     | File                | Compiles via                |
| ------------------------- | ------------------- | --------------------------- |
| Package manifest          | `mosaic-package.toml` | TOML (UI29 §4.2 shape)    |
| Component interface       | `src/Card.mil`      | `mosmodel-compiler`          |
| Layout (kernel-only tree) | `src/Card.mll`      | `moslayout-compiler`         |
| Dark theme                | `src/Card.dark.msl` | `mosstyle-compiler`          |

The integration test `tests/package_compiles.rs` runs all four files
through their compilers and asserts the resulting IR shape — see the test
for the precise contract.

## Relationship to mosaic-pkg-grid (#3647)

The two packages divide the design space deliberately:

- **`mosaic-pkg-grid`** pushes on the kernel's *expressive surface* — it
  uses `For` to iterate over `viewport-rows`, `If` to switch between
  display and edit cells, and `HostTable` / `HostInput` to delegate the
  hard parts to the platform.  It's the test that the rich primitives
  hold together as a system.
- **`mosaic-pkg-card`** pushes on the *plumbing*.  By stripping the
  component down to Box + Column + Text — three primitives that every
  backend has shipped since UI29 day one — it isolates the question
  "does the package architecture (manifest + .mil + .mll + .msl) actually
  work?" from the question "do the advanced primitives all lower
  correctly?"

If Grid's smoke test ever regresses for kernel-primitive reasons, Card's
should still pass — making Card a useful tripwire for distinguishing
architecture-level breakage from primitive-level breakage.

## Layout

```text
mosaic-pkg-card/
├── mosaic-package.toml      # UI29 §4.2 manifest
├── Cargo.toml               # standalone smoke-test crate (empty [workspace])
├── README.md                # this file
├── CHANGELOG.md
├── src/
│   ├── Card.mil             # interface — 3 text slots, 0 emits
│   ├── Card.mll             # layout   — Column[card-root] of three Boxes
│   └── Card.dark.msl        # theme    — 4 parts (root + title + body + footer)
└── tests/
    └── package_compiles.rs  # asserts the whole package compiles
```

## Running the smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-card
cargo test
```

The smoke test is self-contained.  It reads each source file relative to
`CARGO_MANIFEST_DIR`, calls the appropriate compiler crate (which it picks
up from `../rust/<name>` via path dependencies), and asserts on the
resulting IR shape rather than on any emitted string — that way the test
keeps passing as backends evolve, as long as the language frontend stays
true to its contracts.

## License

MIT OR Apache-2.0
