# CLOC07 — Source-Map Generation From the CV Log

## What this spec locks down

Source maps make minified, transformed JavaScript debuggable. They map every
byte of the output back to a byte of the original source. CLOC07 defines how
the `closure-source-map` crate produces a **source-map v3** artifact from the
state already accumulated in the CV log by every prior pipeline stage
(CLOC03).

Concretely: nothing in the optimizer needs to know about source-map format.
Every pass already appends contributions per CLOC03. The source-map
generator is the *consumer* — it walks the CV log and emits the standard
output the rest of the JavaScript world expects.

## Source-map v3 recap (just enough to anchor terms)

A source-map v3 file is a JSON document:

```json
{
  "version": 3,
  "file": "out.min.js",
  "sourceRoot": "",
  "sources": ["src/api.js", "src/util.js"],
  "sourcesContent": ["...optional source text...", "..."],
  "names": ["userPreferences", "logger", "render"],
  "mappings": "AAAA;AACA;AACA,IAAI..."
}
```

- `version` — always `3`.
- `file` — the emitted file this map describes.
- `sources` — list of input files referenced. Indices appear in mappings.
- `sourcesContent` — optional embedded copies of source files (lets
  debuggers work without source-file access at runtime).
- `names` — symbol names used in mappings, for "this byte came from a
  variable formerly named `foo`."
- `mappings` — a VLQ-encoded packed string. Each "segment" is up to five
  integers: `(generatedColumn, sourceIndex, sourceLine, sourceColumn,
  nameIndex)`. Segments separated by `,`; lines separated by `;`. All
  integers except the first are relative deltas (this is why VLQ is used —
  positives and negatives, variable length).

We rebuild this format from scratch. We do **not** depend on the Mozilla
`source-map` Rust crate at runtime — it's only used as a test oracle.

## Crate location and layout

```text
code/packages/rust/closure-source-map/
  BUILD
  BUILD_windows
  CHANGELOG.md
  Cargo.toml
  README.md
  required_capabilities.json
  src/
    lib.rs
    generator.rs        # public API: from_cv_log + write
    resolver.rs         # cv.resolve_root and friends
    vlq.rs              # base64 VLQ encoder
    rope.rs             # rope-based mapping accumulator for big files
    consume.rs          # input source-map composition (--consume-input-map)
```

Crate name: `coding-adventures-closure-source-map`.

### Dependency whitelist

- `coding-adventures-correlation-vector` — reads the CV log.
- `coding-adventures-javascript-ast` — needs `Program` to associate output
  bytes with AST nodes (and through them, with CvIds).
- `serde` + `serde_json` — for the output JSON.
- `base64` — only the alphabet table; VLQ encoding is hand-rolled.

Explicitly **not**:

- `closure-emitter` — emitter calls *into* this crate via a public function,
  not the other way around. Avoids cycles.
- The Mozilla `source-map` crate at runtime. It appears only in
  `dev-dependencies` for the testing oracle.

## How the emitter feeds the generator

Per CLOC03 §"Stage 5 — Emitter," `closure-emitter` keeps a
**byte-range-to-CvId side table** as it writes the output:

```rust
pub struct EmittedSegment {
    pub start: u32,      // byte offset in output (inclusive)
    pub end:   u32,      // byte offset in output (exclusive)
    pub cv:    CvId,     // the AST node these bytes came from
}

pub struct EmittedFile {
    pub bytes: String,
    pub segments: Vec<EmittedSegment>,  // sorted by start, non-overlapping
}
```

The emitter publishes `EmittedFile` alongside the raw bytes. The generator
consumes `(EmittedFile, &CorrelationLog)` and produces a source map. The
emitter itself does no source-map work.

This split has two payoffs:

1. The emitter stays small — its only side-output is a `Vec<EmittedSegment>`.
2. The generator can run after-the-fact on stored compile artifacts; it
   doesn't require the live emitter.

## The `cv.resolve_root` algorithm

For every segment, the generator needs the *original* `Origin` for the CvId:
the source file, line, column. Walking the CV log:

```rust
pub fn resolve_root(log: &CorrelationLog, id: CvId, opts: ResolveOpts)
    -> Vec<Anchor>
{
    // Anchor = { source_file, line, column, name: Option<String> }

    let mut anchors = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![id];

    while let Some(cur) = stack.pop() {
        if !visited.insert(cur) { continue; }                  // cycle guard

        let entry = log.get(cur).expect("CvId in segment must exist");

        if entry.origin.is_real_file_origin() {
            anchors.push(Anchor::from_origin(&entry.origin));
            if !opts.multi { return anchors; }                 // first-wins
            continue;
        }

        // Synthetic origin (e.g., "constant-fold", "asi", "rename").
        // Walk parents, leftmost-first (matches CLOC03 default).
        for &parent in entry.parents.iter().rev() {            // rev so leftmost pops first
            stack.push(parent);
        }
    }

    anchors
}
```

Key properties:

- **Leftmost-first** by default (the leftmost-source-byte ancestor wins).
- **Depth-first**, so we resolve as deep as possible before considering
  siblings.
- **Cycle-safe**: `visited` prevents infinite loops if the log ever has a
  cycle (which CLOC03 disallows but we don't assume).
- **`opts.multi`**: when set, collects *all* root anchors instead of just
  the first. Used by `--source-map-multi`.

`is_real_file_origin()` returns `true` when `origin.source` matches a known
input file. Origins like `"asi"`, `"constant-fold"`, `"rename"`,
`"synthesized"` return `false` — those are pass-internal sources, not source
files.

## Synthetic-origin output bytes

Some output bytes have no source-file ancestor. The constant-fold pass that
produces `5` from `2 + 3` has parent `CvId`s whose origins are real source
positions, so resolution works. But a runtime guard inserted by a pass with
**no** source ancestor — `cv.create(Origin{source: "guard-pass", location:
"synthesized", ...})` — has no real-file ancestor at all.

Encoding rule for source-map v3:

- If `resolve_root` returns at least one real-file anchor → encode a
  4-or-5-field segment (`generatedColumn, sourceIndex, sourceLine,
  sourceColumn[, nameIndex]`).
- If it returns no real-file anchor → encode a **1-field segment**
  (`generatedColumn` only). This is the standard's way of saying "this
  generated position has no source mapping" without breaking the column
  delta chain.

A 1-field segment renders gracefully in Chrome/Firefox/VS Code debuggers —
hovering shows no source position but stepping past it still works.

## The `--source-map-multi` flag (extension mode)

When set, the generator emits a source map that includes an
`x_closure_multi_origins` extension field:

```json
{
  "version": 3,
  ...
  "x_closure_multi_origins": [
    { "generatedLine": 0, "generatedColumn": 12,
      "anchors": [
        { "source": "src/api.js", "line": 41, "column": 4 },
        { "source": "src/api.js", "line": 41, "column": 12 },
        { "source": "src/api.js", "line": 41, "column": 18 }
      ]
    },
    ...
  ]
}
```

The `mappings` field is still single-anchor (leftmost), preserving
compatibility with every existing tool. Tools that recognize the extension
can show all ancestors when stepping through a folded expression. Tools
that don't ignore it silently.

The `x_` prefix is the source-map v3 convention for unofficial extensions.

## The `names` field

Source-map v3's `names` lets a debugger show "the variable here is named
`userPreferences` in the original source even though it's called `a` in
the output." Populating it requires walking the CV chain looking for
contributions of the form:

```text
Contribution { source: "rename", tag: "renamed", meta: { from: "userPrefs", to: "a" } }
```

When a `rename` contribution is present on the leaf anchor's CvId chain,
the generator:

1. Adds `from` to the `names` array (deduped).
2. Emits a 5-field segment with the corresponding name index.

This is what makes minified code debuggable — the source map "remembers" the
original identifier names.

For non-identifier bytes (operators, punctuation), the `names` field stays
empty. The generator only attaches names when an identifier-shaped AST
node is involved.

## Input source-map composition (`--consume-input-map`)

Closure Compiler often runs **after** another tool (Babel, TypeScript,
Webpack). If the input JS file has a prior source map, the generator must
*compose* it with the new one so the final output still traces back to the
*original* source — not the transpiled-but-not-yet-optimized intermediate.

Composition algorithm:

```text
For each segment in our new map:
    output_anchor   = our resolved (source, line, col)
    composed_anchor = lookup(input_map, output_anchor)
        # input_map answers "what's the original source for this position
        # in our INPUT (which was the OUTPUT of the prior tool)?"
    emit composed_anchor as the segment's anchor
```

`closure-source-map::consume` parses an input source map (v3) into a
lookup structure and exposes `lookup(file, line, col) -> Option<Anchor>`.
When the lookup fails (input map is incomplete), the original anchor is
used unchanged.

CLI flag: `--consume-input-map=path/to/input.js.map`. The CLI also auto-
detects inline `# sourceMappingURL=` comments at the end of input files.

This is the same pattern the Mozilla source-map library calls
`SourceMapConsumer + SourceMapGenerator.applySourceMap`.

## Behavior when `cv.enabled == false`

If the CV log is disabled (production fast path per CLOC03), no
contributions are stored. The generator cannot produce a meaningful source
map.

CLI behavior:

- `closurec` with neither `--source-map` nor `--no-trace`: defaults to
  `trace=true`, source map produced.
- `closurec --source-map=out.js.map`: forces `trace=true` even if config
  said otherwise. Source map produced.
- `closurec --no-trace`: source map disabled. Generator returns
  `Err(SourceMapError::TraceDisabled)`. CLI exits 0 with no map file
  written.
- `closurec --no-trace --source-map=out.js.map`: hard error. The CLI
  refuses to start.

A warning is emitted whenever a source-map request is silently dropped due
to `enabled=false` (this would catch misconfigured production deploys).

## Inline vs external source maps

Two output modes:

1. **External** (default): write `out.js.map` next to the output file. The
   output file gets a trailing `//# sourceMappingURL=out.js.map` comment.
   This is what most build tools produce.

2. **Inline**: with `--source-map-inline`, the entire map is base64-encoded
   and embedded in the output as `//# sourceMappingURL=data:application/
   json;base64,...`. Useful for development; not for production (it
   roughly triples the output file size).

The closure-emitter handles writing the `sourceMappingURL` comment — the
generator just hands it the URL string.

## Large-file performance: rope-based VLQ

A typical bundle has hundreds of thousands of mapping segments. Naive
`String` accumulation with `push_str` reallocates frequently. The generator
uses a **rope-of-segments** internal representation:

```rust
pub struct MappingRope {
    chunks: Vec<String>,         // each chunk = one output line's mappings
    current_line: String,
    last_emitted_column: u32,
    last_source_index: u32,
    last_source_line: u32,
    last_source_column: u32,
    last_name_index: u32,
}
```

- Each chunk corresponds to one output line. Lines separated by `;` in the
  final `mappings` string.
- Segment deltas are tracked against the rope-local state, so emitting a
  new segment is O(1) (push a few VLQ bytes).
- Finalization joins chunks with `;`.

VLQ encoding is a single `vlq::encode(i32, &mut String)` function — about
30 lines. Implemented inline rather than via a third-party crate because
the format is small and we want zero external surface.

Expected throughput on a modern laptop: ~5M segments/sec single-threaded.
A bundle with 1M segments encodes in ~200 ms.

## Public API

```rust
pub fn generate(
    emitted: &EmittedFile,
    log: &CorrelationLog,
    opts: GeneratorOptions,
) -> Result<SourceMap, SourceMapError>;

pub struct GeneratorOptions {
    pub file: String,
    pub source_root: String,
    pub include_sources_content: bool,
    pub multi_origins: bool,                    // --source-map-multi
    pub consume_input_map: Option<ConsumedMap>, // from consume::parse
    pub names_from_rename_passes: bool,         // default true
}

pub struct SourceMap {
    pub version: u8,                            // always 3
    pub file: String,
    pub source_root: String,
    pub sources: Vec<String>,
    pub sources_content: Option<Vec<String>>,
    pub names: Vec<String>,
    pub mappings: String,                       // VLQ-packed
    pub x_closure_multi_origins: Option<Vec<MultiOriginSegment>>,
}

impl SourceMap {
    pub fn to_json(&self) -> String;
    pub fn write_external(&self, path: &Path) -> io::Result<()>;
    pub fn to_inline_url(&self) -> String;
}
```

The shape is stable; consumers (the CLI, in particular) pin against this
API.

## Testing strategy

| Layer | Tests |
| --- | --- |
| VLQ encoder | Round-trip against Mozilla `source-map` crate for ±1, ±63, ±64, ±i32::MAX. |
| Resolver | Synthetic CV logs with known shapes (chain, fan-out, cycle, synthetic-only). |
| Generator end-to-end | Small JS file → optimize → emit → generate → parse with Mozilla `source-map` → assert each output byte maps to the expected source byte. |
| `--consume-input-map` | Two-step: TS → JS (with map) → minified JS. Final map must point at the TS source. Uses a known-good TS→JS map as input. |
| `--source-map-multi` | A constant-fold of `2 + 3 + 4` should produce a segment with three anchors. |
| Performance | A 100k-segment synthetic input must encode in < 100 ms. (Microbench, not asserted on slow CI.) |
| `--source-map-inline` | Output contains a `data:` URL whose decoded JSON parses as a valid v3 map. |

Mozilla's `source-map` Rust crate is the testing oracle. We require: every
map we emit parses back to the same `(generatedLine, generatedColumn,
source, line, column, name)` tuples we computed before encoding.

Coverage target: 95% for the encoder/resolver, 90% for the I/O paths.

## Open questions

1. **Source content embedding by default?** Closure Compiler does not embed
   sources by default; webpack does. We default to *not* embedding
   (smaller maps, no risk of leaking proprietary source). `--source-map-
   include-sources` opts in.
2. **`null` in `sources` for synthetic origins.** The v3 spec is ambiguous
   about whether `sources` can include `null` for "no source." We avoid the
   ambiguity by using 1-field segments instead.
3. **Index maps.** v3 supports a "section" or "index" map that composes
   multiple sub-maps. We don't emit them today (a single bundle = a single
   map). May need them if we add multi-output support later.
4. **Hash for cache-busting.** Some build pipelines want the source-map URL
   to include a content hash. Out of scope; the CLI can append it post-
   generation.
5. **DWARF / PDB integration.** The user's `project_dev_tools_for_free`
   memory says LANG-VM languages get DWARF/PDB debug info for free. For
   JS-to-JS optimization, the natural debug format is source-map v3 only.
   DWARF would only matter when the V8 clone emits native code, at which
   point we'd write a separate spec for that backend.

## What this spec does **not** cover

- The `closure-emitter` crate's internals — only its `EmittedSegment` /
  `EmittedFile` contract.
- The CLI flags themselves in detail — those belong to CLOC08, which will
  cross-reference this spec.
- DWARF/PDB generation (covered by future V8-clone specs).
- Bundle splitting / multi-output. The MVP assumes one input bundle, one
  output file, one source map.
