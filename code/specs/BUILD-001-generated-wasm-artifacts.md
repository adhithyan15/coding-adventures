# BUILD-001: Generated WebAssembly artifacts are built, not committed

## Status

Accepted. Supersedes the "committed artifact plus `--check`" scheme introduced
for `spreadsheet-wasm` in #17810 and the "checked-in `pkg/engram_engine.wasm`"
convention of `engram-wasm`.

## Problem

Three Rust crates compile to `wasm32-unknown-unknown` for the browser demos:

| crate            | artifact                                          | consumers |
|------------------|---------------------------------------------------|-----------|
| `spreadsheet-wasm` | `pkg/spreadsheet_engine.wasm` (+ `.inputs` stamp) | VisiCalc demos, via the derived `visicalc-html/vendor/spreadsheet-engine-wasm.js` |
| `task-wasm`      | `pkg/task_engine.wasm`                            | task-app web host, release and Pages workflows |
| `engram-wasm`    | `pkg/engram_engine.wasm`                          | engram-app web/Electron hosts, release and Pages workflows |

All of them, and the derived VisiCalc base64 bundle, were committed. That
caused four recurring problems:

1. **Spurious staleness.** `spreadsheet-wasm/build-wasm.sh --check` hashed every
   crate in `cargo tree -p spreadsheet-wasm`. Any change to a shared crate
   (`lexer`, `grammar-tools`, ...) failed CI on an unrelated PR, and each time
   somebody had to rebuild and recommit a binary (see lessons.d: "Changing a
   crate inside spreadsheet-wasm's dependency tree..."). `Cargo.lock` is
   gitignored, so even an upstream crates.io release could flip the check.
2. **Unreviewable diffs.** A binary change cannot be code-reviewed; the
   reviewer can only trust the author.
3. **History bloat.** Every rebuild adds hundreds of kilobytes (the VisiCalc
   bundle adds the same bytes again as base64) to every clone, forever.
4. **Silent drift where there is no check.** `engram_engine.wasm` went two
   months stale with no failing test, because its smoke test was written
   against the stale behaviour too.

## Policy

> A file that is a pure function of committed sources and a toolchain is
> **built, never committed**.

For these artifacts that means:

- `pkg/` of `spreadsheet-wasm`, `task-wasm` and `engram-wasm`, and
  `visicalc-html/vendor/spreadsheet-engine-wasm.js`, are listed in `.gitignore`
  and must not be tracked.
- Hand-written test inputs are not generated artifacts and stay committed. In
  particular `code/packages/dart/wasm-runtime/test/fixtures/square_nostd.wasm`
  is a fixture whose exact bytes the Dart tests assert on.
- Hand-written source that merely lives next to an artifact (for example
  `spreadsheet-wasm/js/spreadsheet-engine-wasm.mjs`, the loader) stays
  committed.

## How each consumer gets the artifact

There is one local entry point:

```bash
bash code/scripts/build-demo-wasm.sh            # everything
bash code/scripts/build-demo-wasm.sh spreadsheet visicalc   # a subset
```

It adds the `wasm32-unknown-unknown` target when `rustup` is present, runs each
crate's own `build-wasm.sh` (those remain the single place that knows how to
compile a crate) and then `visicalc-html/scripts/bundle-wasm-engine.sh`, which
base64-embeds the spreadsheet `.wasm` plus `scripts/wasm-loader.js` into the
vendor bundle.

| consumer | how it obtains the artifact |
|----------|-----------------------------|
| `spreadsheet-wasm`, `task-wasm`, `engram-wasm` BUILD files | build the `.wasm` (`rustup target add`, `bash build-wasm.sh`) and then run the crate's `node js/smoke.mjs` against the fresh build |
| VisiCalc demos (`visicalc-html`, `visicalc-webcomp`, `visicalc-deno`) | run `build-demo-wasm.sh visicalc` once after a clone; their READMEs say so |
| `mosaic/visicalc` `tests/web_demos_are_current.rs` | the engine tests build the bundle on demand (`bundle-wasm-engine.sh` builds the `.wasm` if absent) before exercising it |
| `task-app` `scripts/build-web.sh`, `build-webcomponent.sh` | already call `task-wasm/build-wasm.sh` first; unchanged |
| `engram-app` `scripts/build-web.sh`, `build-electron.sh`, `build-all.ps1` | already call `engram-wasm/build-wasm.sh` first; unchanged |
| `release-task-app.yml`, `release-engram.yml`, `deploy-task-app.yml`, `deploy-engram.yml` | already build through the scripts above; the `cmp` of the built engine against the bundled copy remains and is now a real "the shipped engine is the one just built" check |

CI never reads a binary from git. It builds, and what it builds is what it
tests, bundles and publishes.

## What replaces the staleness checks

Staleness cannot exist for a file that is not stored. What the checks were
protecting against is covered as follows:

- **"The demo ships an engine that matches the source"**: the demo consumes
  the freshly built artifact. The BUILD files of the three wasm crates build for
  `wasm32-unknown-unknown` and run the Node smoke test against that build, so a
  change that breaks the wasm target, the exported ABI, or the behaviour the
  smoke test pins fails the PR that made it.
- **"The bundle embeds the .wasm"**: the bundle is regenerated, never
  verified. `bundle-wasm-engine.sh --check`, `build-wasm.sh --check`,
  `inputs-digest.py` and the `.inputs` stamp are deleted.
- **Deterministic bytes** are no longer a correctness requirement. The
  `--remap-path-prefix` flags in `spreadsheet-wasm/build-wasm.sh` are kept
  because they are free and make local builds comparable, but nothing compares
  against a committed hash.

## Build-tool inputs

The build tool hashes a package's source files to decide whether to rebuild.
Registries that name these artifacts as exact hash inputs would, after a local
build, make a package's hash depend on an ignored build output. Because they
are listed in `.gitignore` and absent from a clean checkout, the hash on CI is
unaffected; the entries are kept only where removing them would force a
regeneration of every language's registry digest. Where an entry is kept, it
is documented as describing a file that exists only after a build.

## Migration

1. Land this spec alone.
2. `git rm --cached` the four tracked artifacts and add the ignore rules.
3. Delete `inputs-digest.py`, the `--check` modes and the `BUILD` staleness
   lines. Add `code/scripts/build-demo-wasm.sh`.
4. Update every consumer in the table above, its README and CHANGELOG.
5. Existing clones keep their working-tree copies after the merge because the
   files are removed from the index only for the commit; a clone that pulls
   the deletion has the files removed and runs `build-demo-wasm.sh` once.
6. The three lessons about rebuilding the committed wasm are superseded; they
   stay as history and gain a note pointing here.

## Non-goals

Publishing the demos' wasm to a package registry or release asset store is not
part of this change. If a future demo needs a prebuilt binary without a Rust
toolchain, it should be a release asset produced by CI, not a file in git.
