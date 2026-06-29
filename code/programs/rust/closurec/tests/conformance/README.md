# Conformance corpus — golden generation

`tests/conformance.rs` checks that closurec's SIMPLE optimizer is **value-
preserving**: each optimized output, when it folds to a literal, must have the
same runtime value as the original source expression.

The "true value" of each source is recorded as a canonical, `Object.is`-faithful
string in the `CORPUS` table. Those golden strings are produced **offline** by
Node/V8 — CI never runs Node. To (re)generate them:

```bash
cd code/programs/rust/closurec
mise exec -- node tests/conformance/gen_goldens.mjs
```

Each line prints `"<source>"\t<canonical>`. Paste the canonical values into the
`CORPUS` array in `tests/conformance.rs`. The Node `canon()` function and the
Rust literal evaluator in `conformance.rs` must produce identical strings;
numbers are emitted verbatim because closurec's `format_js_number` already
matches V8's `Number.prototype.toString`.

`KNOWN_DIVERGENCES` holds inputs closurec currently miscompiles (with the wrong
value it emits). When such a bug is fixed, its `conformance_known_divergences_*`
assertion fails — move the entry into `CORPUS` at that point.
