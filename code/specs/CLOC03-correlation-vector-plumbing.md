# CLOC03 — Correlation-Vector Plumbing Through the JavaScript Pipeline

## What this spec locks down

The correlation vector (CV) is what makes the Closure Compiler clone debuggable
and the V8 clone (later) traceable. This spec defines, for every stage in the
JavaScript pipeline, exactly what CV operations happen, on what data, and what
contributions are appended. It does **not** describe the CV crate itself
(`correlation-vector` already exists and is documented in its own README); it
describes how every CLOC stage **uses** that crate.

The structural invariant from this spec onward, applied with no exceptions:

> Every byte of input text is reachable from at least one output byte via the
> CV log. Conversely, every byte of output text is reachable from at least one
> root `Origin` (file, line, column). Any pass that breaks this invariant is a
> bug in the pass, not a feature.

## The CV crate, in two sentences

`correlation-vector` provides a `CorrelationLog` that issues `CvId`s and stores
`Contribution`s appended by stages. IDs are minted via `create(Origin)` (for
roots), `derive(parent_id, Origin)` (for one-parent children), or
`merge(parents, Origin)` (for nodes built from multiple inputs). Contributions
record what a named stage did to an entity: `contribute(id, Contribution{source,
tag, meta})`.

The log has an `enabled` flag. When `false`, IDs are still issued but no
contributions or origins are stored. This is the production fast path — every
stage runs the same code; only the storage gets dropped.

## Where the log lives

A single `CorrelationLog` is created at the start of a compile by `closure-cli`
(or by the V8-clone's driver) and passed by `&mut` reference into every stage.
Stages do not create their own logs. They do not buffer contributions; they
append directly.

```rust
// closure-cli pseudocode
let mut cv = CorrelationLog::new();
cv.set_enabled(opts.trace);

let program = parse_javascript_with_cv(&source, opts.version, &mut cv)?;
let typed   = run_typechecker(program, &sidecar, &mut cv)?;
let passes  = run_pass_pipeline(typed, &mut cv)?;
let bytes   = emit_with_cv(&passes, &mut cv)?;
let map     = source_map_from_log(&bytes, &cv);
```

The `&mut cv` thread is intentional: it is the *one* mutable thing in the
compile. The AST, sidecar, and pass outputs are all immutable.

## Stage 1 — Lexer (`javascript-lexer`)

For every token the lexer emits, it calls `cv.create(Origin)` with:

```rust
Origin {
    source:   <filename or "stdin">,
    location: format!("{}:{}-{}", line, col_start, col_end),
    timestamp: None,
    meta:     {} // empty by default; passes that re-lex may add tags
}
```

The resulting `CvId` is stored on the token. Today's `lexer::Token` carries a
span; after this spec lands, it also carries `cv: CvId`. The span field stays
(for debugging) but `cv` is the authoritative identifier.

The lexer appends **no contributions**. Lexing is the act of *creation*; there
is nothing yet to contribute about.

### Skipped trivia

Whitespace and comments are produced by the lexer's skip patterns. They still
get a `CvId` each, but they are not attached to tokens. The JSDoc extractor
(CLOC05) walks the log to find comment-origin IDs and pairs them with the
nearest declaration's CV.

## Stage 2 — Parser (`javascript-parser`)

The parser reduces token sequences into `javascript-ast` nodes. For each
production:

1. Collect the `CvId`s of the consumed tokens.
2. Mint a node `CvId` via `cv.merge(parent_ids, Origin{...})`. The `Origin`
   on a parsed node uses the *source* of the leftmost token and a `location`
   computed from the span union.
3. Store that `CvId` on the constructed `javascript-ast` node.
4. Append one contribution: `Contribution{source: "parser", tag: "constructed",
   meta: {"rule": "<grammar_rule_name>"}}`.

The `tag: "constructed"` contribution is the parser's way of recording *which
grammar production* built the node. This is invaluable for debugging the
parser, but it's also useful at runtime: a CLI flag like `closurec --dump-cv`
can print the production for each node.

### Synthetic semicolons (ASI)

Automatic semicolon insertion produces tokens the lexer did not emit. For each
inserted semicolon, the parser calls `cv.create(Origin{source: "asi", location:
"<position>", meta: {"reason": "newline" | "eof" | "restricted"}})`. The
synthetic semicolon's `CvId` is then used in the production.

The contribution `tag: "asi-inserted"` is appended. Debuggers and source maps
that hit an ASI position can show the user where the JavaScript engine would
have placed a semicolon.

## Stage 3 — Typechecker (`closure-typechecker`)

For every node it judges:

```rust
cv.contribute(node.cv, Contribution {
    source: "typechecker",
    tag:    "judged",
    meta:   json!({
        "type":     "<resolved-type>",      // optional
        "narrowed": [...],                  // narrowing applied
        "errors":   [...]                   // type errors found here
    }),
});
```

The typechecker also writes a parallel `Sidecar` entry keyed by `node.cv`
(see CLOC04). The CV contribution is the *audit trail*; the sidecar entry is
the *result*. A consumer can ask "what's the type of node X?" by looking up X
in the sidecar; the CV contribution explains *why* that type was chosen.

## Stage 4 — Optimization passes (the `closure-pass-*` family)

Every pass declares a `pass_name: &'static str` (e.g., `"dce"`, `"rename"`,
`"constant-fold"`). For every node a pass touches, it appends a contribution
keyed by that name.

### When a pass keeps a node unchanged

A pass that visits a node but decides to leave it alone appends *nothing*. CV
entries are for changes, not for visits. Otherwise the log would balloon.

### When a pass mutates a node's shape

When a pass produces a new node from old ones, the new node's `CvId` is minted
via `cv.merge(old_ids, Origin{source: "<pass_name>", location: "synthesized",
meta: {...}})`. The contribution `tag: "synthesized"` is appended.

Example: constant folding of `2 + 3` into `5`:

```rust
let original_left  = expr.left.cv;
let original_right = expr.right.cv;
let original_op    = expr.cv;

let new_cv = cv.merge(
    &[original_left, original_right, original_op],
    Origin {
        source:   "constant-fold",
        location: "synthesized",
        meta:     json!({ "from": "BinaryExpression", "to": "NumberLiteral" }),
        timestamp: None,
    },
);

cv.contribute(new_cv, Contribution {
    source: "constant-fold",
    tag:    "folded",
    meta:   json!({ "value": 5, "op": "+" }),
});
```

The CV of the original tokens `2`, `+`, `3` remains in the log — they are still
reachable as parents of `new_cv`. The source map can therefore still attribute
the `5` byte back to the original source range.

### When a pass deletes a node

```rust
cv.contribute(deleted_node.cv, Contribution {
    source: "dce",
    tag:    "deleted",
    meta:   json!({ "reason": "unreachable" | "unused" | "side-effect-free" }),
});
cv.delete(deleted_node.cv);  // marks the entity as removed; log entries persist
```

`delete` does *not* remove the node's CV record from the log. It marks it as
deleted so traversals can distinguish "still present in the tree" from
"removed by a pass." The contribution explains why.

### When a pass synthesizes a new node from nothing

Rare, but happens (e.g., a runtime guard insertion). The pass calls
`cv.create(Origin{source: "<pass_name>", location: "synthesized", meta: ...})`
to get a root `CvId` with no parents. Source maps will resolve such bytes to
the pass that created them, not to any source line — which is correct: the
output byte has no source-line ancestor.

### Pass contribution conventions

| Pass | Common tags |
| --- | --- |
| `constant-fold` | `folded`, `simplified` |
| `dce` | `deleted` |
| `rename` | `renamed` (meta: `{from, to}`) |
| `inline` | `inlined` (meta: `{callee_cv}`), `outlined` |
| `treeshake` | `deleted` (meta: `{reason: "unexported"}`) |
| `collapse-properties` | `flattened` (meta: `{from_path, to_name}`) |
| `fold-control-flow` | `branch-eliminated`, `dead-arm-removed` |
| `remove-unused-vars` | `deleted` |

Tags are conventions, not enforced. Any pass can append any tag; consistency is
maintained by code review and the pass's own README.

## Stage 5 — Emitter (`closure-emitter`)

The emitter walks the optimized AST and produces a byte stream. For every
contiguous range of output bytes that come from a single AST node, the emitter
records a mapping `(byte_offset_start, byte_offset_end, node.cv)` in an output
side-table.

The emitter itself appends one contribution per node: `Contribution{source:
"emitter", tag: "emitted", meta: {"start": <byte>, "end": <byte>}}`. This is
both useful for debugging and used directly by the source-map generator.

The emitter does **not** call `cv.create` or `cv.merge`. It is the last
read-only pass; no new entities are born here.

## Stage 6 — Source map generator (`closure-source-map`)

The source map is built by walking the emitter's byte-to-CV side-table:

```text
for each (byte_range, cv_id) in emitter_output:
    root = cv.resolve_root(cv_id)
    // root_origin = (source: "app.js", location: "42:13-42:18")
    add_source_map_segment(byte_range, root.source, root.line, root.column)
```

`cv.resolve_root(id)` walks parents of `id` until it hits an entity whose
`Origin.source` is a real file (not `"asi"`, `"constant-fold"`, etc.). Multiple
parents (from `merge`) are walked depth-first; the leftmost ancestor wins for
the primary mapping. The source map v3 `names` field optionally records the
chain.

### Multi-source mappings

When a single output byte has multiple source-byte ancestors (e.g., a folded
`5` that came from `2 + 3`), the source map can either:

- (default) Map to the leftmost ancestor (`2`).
- (`--source-map-multi`) Emit additional `extensions` entries (per the source
  map v3 extension spec) listing all ancestors.

The default keeps the source map compatible with every existing tool. The
multi flag is for advanced debugging.

## Side: the `enabled` fast path

In production, `closurec --no-trace` sets `cv.set_enabled(false)`. Then:

- `cv.create`, `cv.derive`, `cv.merge` still return IDs (counters increment),
  so node structs remain shape-identical.
- `cv.contribute` becomes a no-op.
- `cv.delete` becomes a no-op.
- The log stays empty; memory cost is just the counter state.

The source map cannot be generated when tracing is off. `closurec` either
warns and emits no map, or errors out if `--source-map` was passed explicitly
alongside `--no-trace`.

## What every CLOC PR must include

This is the closing checklist that any CLOC-series PR touching a pipeline
stage will be reviewed against:

1. Does the stage take `&mut cv: CorrelationLog`?
2. Does it create/derive/merge IDs for every entity it produces?
3. Does it append contributions for every change?
4. Does it never crash when `cv.enabled == false`?
5. Is the contribution `source` field the stage's canonical name (e.g.,
   `"dce"`, not `"dead_code_elimination"`)?
6. Are tags from the conventional list, or — if not — documented in the
   crate's README?
7. Do tests assert at least one CV contribution for at least one transformation
   the stage performs?

If a PR fails any of these, it does not merge. The CV invariant is what makes
this whole project worth doing; we do not let it rot.

## Open questions

1. **Log size in advanced mode.** A full advanced-mode Closure compile of a
   large bundle can produce millions of contributions. We may need a streaming
   log that writes to disk past a threshold. Not blocking for MVP; flagged for
   CLOC04 sidecar work where the same scaling concern applies.
2. **Cross-compile reuse.** If two compiles share a source file, can their
   CV logs be merged for cross-compile analysis? The crate supports `merge` at
   the log level; we will spec the exact merge semantics in a follow-up if and
   when a cross-compile use case appears.
3. **Pass-internal CV.** Some passes (e.g., inliner) need to track interior
   dataflow state. Should that state use CV, or use a pass-local map keyed by
   CV? Default is the latter (pass-local map). CV is for the durable lineage,
   not for transient analysis state.
