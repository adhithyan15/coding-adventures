# Package Parity Roadmap

## Goal

Close package gaps intelligently across language buckets without pretending
that directory equality is the same as useful parity.

The repo should aim for portable API parity where a package is naturally
language-agnostic, and for honest native or web specialization where the
package is tied to a runtime, operating system, browser API, or FFI boundary.

## Current Baseline

The most useful baseline today is not "everything everywhere." It is:

- **Portable core:** packages present in both Rust and Python.
- **Native source of truth:** Rust packages that expose platform, FFI, graphics,
  runtime, or C ABI behavior.
- **Python prototyping/runtime lane:** Python packages that carry language
  runtime experiments, compiler tracks, and native-wrapper facades.
- **Web lane:** TypeScript and WASM packages that depend on browser, DOM,
  Canvas, IndexedDB, Vite, or Web Audio behavior.

After normalizing folder naming conventions, the current inventory is:

| Baseline | Count |
|---|---:|
| All normalized package names, excluding Starlark | 540 |
| Rust/Python union | 493 |
| Rust/Python shared core | 261 |
| Rust packages | 377 |
| Python packages | 377 |
| Rust-only packages | 116 |
| Python-only packages | 116 |
| Packages outside Rust/Python | 47 |

Core parity coverage against the Rust/Python shared core:

| Language | Missing Core | Coverage |
|---|---:|---:|
| TypeScript | 19 | 92.7% |
| Go | 23 | 91.2% |
| Ruby | 23 | 91.2% |
| Elixir | 32 | 87.7% |
| Perl | 56 | 78.5% |
| Lua | 57 | 78.2% |
| Swift | 136 | 47.9% |
| Haskell | 138 | 47.1% |
| Java | 192 | 26.4% |
| Kotlin | 193 | 26.1% |
| WASM | 200 | 23.4% |
| F# | 209 | 19.9% |
| C# | 210 | 19.5% |
| Dart | 228 | 12.6% |

Regenerate the numbers with:

```sh
python scripts/package_parity_report.py --format markdown
```

## Parity Rules

### Portable Packages Should Converge

Packages should converge across all serious implementation languages when they
are pure algorithms, data structures, grammar frontends, codecs, IR models,
validators, encoders, simulators, or deterministic transforms.

Examples:

- `hash-functions`
- `trie`
- `bloom-filter`
- `lexer`
- `parser`
- `wasm-module-encoder`
- `json-value`
- `zip`

### Native Packages Should Not Be Reimplemented Blindly

Rust should remain the implementation source of truth for packages that are
really platform or ABI surfaces. Other languages should usually receive thin,
tested wrappers instead of independent reimplementations.

Examples:

- `python-bridge`, `ruby-bridge`, `node-bridge`, `lua-bridge`
- `epoll`, `kqueue`, `iocp`
- `audio-device-coreaudio`
- `paint-vm-direct2d`, `paint-vm-gdi`, `paint-metal`
- `cuda-compute`, `metal-compute`
- `*-c`, `*-native`, and runtime extension packages

### Web Packages Stay Web-First

Browser and app-shell packages should not become parity blockers for backend
languages.

Examples:

- `indexeddb`
- `ui-components`
- `vite-plugin-lattice`
- `paint-vm-canvas`
- `window-canvas`
- `web-audio-sink`
- `browser-extension-toolkit`

### Runtime-Specific Compiler Backends Need Dependency Readiness

Compiler and VM packages should be ported in dependency waves, not one package
at a time. A backend package is only ready when its local lexer, parser, IR,
validator, encoder, and runtime dependencies are already present or included in
the same wave.

## Optimal Implementation Order

### Phase 0: Keep the Map Fresh

Status: started by this roadmap and `scripts/package_parity_report.py`.

- Keep `scripts/package_parity_report.py` as the canonical quick inventory.
- Use the Rust/Python shared core as the portable parity target.
- Classify every apparent gap before implementing it.
- Do not count Starlark as an implementation language; it is a build/config
  rule lane.

### Phase 1: Close Near-Parity Languages First

Target languages:

- TypeScript
- Go
- Ruby
- Elixir

These buckets are already above 87% core coverage. Closing them first gives the
repo fast wins and creates templates for Lua, Perl, Swift, and Haskell.

Recommended first wave:

| Package | Languages | Why first |
|---|---|---|
| `hash-functions` | TypeScript, Go, Ruby, Elixir | Leaf-like utility used by later structures |
| `trie` | TypeScript, Go, Ruby, Elixir | Pure data structure with clear tests |
| `bloom-filter` | TypeScript, Go, Ruby, Elixir | Depends naturally on hash behavior |

The `hash-functions` row is implemented in this branch. The next best cut is
`trie`, followed by `bloom-filter`.

Recommended second wave:

| Package | Languages | Why second |
|---|---|---|
| `css-lexer` | TypeScript, Go | Grammar-generated and browser-relevant |
| `css-parser` | TypeScript, Go, Elixir | Completes the CSS frontend pair |
| `haskell-lexer` | Go, Perl | Follow existing grammar-generated package pattern |
| `haskell-parser` | Go, Perl | Follows lexer support |

Hold for classification:

- `audio-device-sink`: likely portable facade plus native backends.
- `tcp-server`: portable contract, but runtime behavior must be shaped per
  language.
- `ml-framework-*`: probably educational facades; confirm desired depth before
  porting.
- `jit-compiler`, `lisp-*`, `starlark-compiler`: port as compiler waves after
  dependency checks.

### Phase 2: Bring Lua and Perl to Near-Parity

Lua and Perl are around 78% core coverage and share many missing packages. Use
the Phase 1 implementations as templates after they pass in TypeScript, Go,
Ruby, and Elixir.

Prioritize:

- `hash-functions`
- `bloom-filter`
- `fenwick-tree`
- `hash-map`
- `hyperloglog`
- `radix-tree`
- `skip-list`
- `tree-set`
- `trie`
- `resp-protocol`
- `tcp-server`

Then continue with compiler/runtime packages from the existing convergence
specs.

### Phase 3: Treat Haskell and Swift as Medium Catch-Up Tracks

Haskell and Swift sit near 47% core coverage, so their next work should be
dependency-shaped rather than gap-count-shaped.

Haskell:

- Refresh `haskell-catch-up-survey.md` with the new merged baseline.
- Prioritize tooling/core packages that make future Haskell package creation
  cheaper.
- Continue the Brainfuck/Nib/WASM convergence wave in
  `04n-haskell-wasm-convergence.md`.
- Be strict about `cabal.project` transitive local dependencies.

Swift:

- Add a Swift catch-up survey similar to the Haskell one.
- Prioritize portable algorithm/data-structure packages before native app
  surfaces.
- Port grammar frontends only after package scaffolding and build support are
  predictable.

### Phase 4: Work in Paired Ecosystems

Java and Kotlin should move together. C# and F# should move together.

The best initial paired waves are:

- generated lexer/parser packages
- data structures
- crypto and compression primitives
- WASM encoder/runtime slices
- existing Brainfuck/Nib convergence specs

Avoid one-off ports that make Java diverge from Kotlin or C# diverge from F#.

### Phase 5: Keep Dart and WASM Selective

Dart has low coverage but should not blindly mirror systems packages. Favor
portable packages that fit the Dart ecosystem:

- data structures
- codecs
- QR/barcode work
- grammar frontends
- document and paint transforms

WASM packages should stay focused on wrappers, runtime targets, and browser or
portable execution units. They do not need to mirror every source-language
package.

## First Implementation Tranche

Start with 12 packages:

- `code/packages/typescript/hash-functions` - done in this branch
- `code/packages/go/hash-functions` - done in this branch
- `code/packages/ruby/hash_functions` - done in this branch
- `code/packages/elixir/hash_functions` - done in this branch
- `code/packages/typescript/trie`
- `code/packages/go/trie`
- `code/packages/ruby/trie`
- `code/packages/elixir/trie`
- `code/packages/typescript/bloom-filter`
- `code/packages/go/bloom-filter`
- `code/packages/ruby/bloom_filter`
- `code/packages/elixir/bloom_filter`

Build order:

1. `hash-functions`
2. `trie`
3. `bloom-filter`

Validation rules:

- Each package needs `BUILD`, README, CHANGELOG, package metadata, and tests.
- BUILD files must install transitive local dependencies in leaf-to-root order.
- Ruby requires dependency `require` ordering before local modules.
- Elixir implementations must avoid reserved words as variables.
- Go packages must run `go mod tidy` after adding local module dependencies.
- TypeScript packages must avoid committing generated `.js`, `.d.ts`, or source
  map outputs.

## Completion Definition

This roadmap is working when:

- the parity report can be regenerated after every major merge;
- near-parity language gaps shrink in coherent waves;
- native and web-only packages are classified instead of treated as failures;
- new implementations follow existing package conventions;
- specs, READMEs, CHANGELOGs, and tests move together.
