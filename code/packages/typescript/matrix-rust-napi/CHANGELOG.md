# Changelog

All notable changes to `@coding-adventures/matrix-rust-napi`
(TypeScript wrapper) are documented here.  The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.4.0] — 2026-05-18

### Added — MX07 Phase 4: TypeScript wrapper + end-to-end vitest smoke

Ships the JS-side counterpart of the matrix-rust-napi Rust addon.

**Public API** (typed re-exports of the four addon entry points):

```typescript
import {
  Graph,             // class — wraps Rust Box<matrix_ir::Graph>
  Runtime,           // class — owns the CPU executor
  graphRoundTripJson,
  runGraphOnCpu,
  MATRIX_IR_VERSION,
} from "@coding-adventures/matrix-rust-napi";
```

The wrapper itself is `src/index.ts` — a ~280-line module that:

* declares the TypeScript interface for `Graph`, `Runtime`,
  `GraphConstructor`, `RuntimeConstructor`, and the addon shape;
* lazily `createRequire`s the `.node` binary at its in-repo location
  (`code/packages/rust/matrix-rust-napi/matrix_rust_napi.node`);
* wraps the addon's `Graph` and `Runtime` JS classes in `Proxy`
  shims so the load stays lazy until first instantiation;
* throws a precise actionable error if the `.node` file isn't built
  yet, telling the caller to run `npm run build` in the Rust crate
  directory.

**Smoke test suite** (`tests/smoke.test.ts` — 15 vitest tests):

* Graph class: parse + describe, `toJson()` round-trip,
  `Graph.fromJson` static equivalence, malformed-JSON rejection,
  unsupported-version rejection.
* Runtime class: `Runtime.create()` factory, **element-wise Add of
  f32×3 with numerical assertion** on the output Buffer, **2×2 MatMul
  with the textbook result asserted**, wrong-arity / wrong-byte-length
  / non-Buffer error paths.
* `graphRoundTripJson` and `runGraphOnCpu` JSON-envelope paths still
  exercised here too, so the legacy API doesn't regress.

**Test fixtures construct the matrix-ir-json payload by hand**
(no `GraphBuilder` round-trip) — proves the schema is stable across
the FFI boundary and human-writable for golden-file fixtures, per
MX07 §"How CI proves this works".

### Critical fix that surfaced during Phase 4: addon stored stale `napi_value`

The Phase 4 vitest run was the first time anything actually called
`Graph.fromJson(...)` and `Runtime.create()` through the addon's
static-method entry points.  Both returned `undefined` —
`napi_new_instance` was returning `napi_invalid_arg` (status 1)
because the class constructor was stored as a **`napi_value`** (a
local handle valid only inside the current handle scope) instead
of a persistent **`napi_ref`**.

Fixed in this same PR by switching `GRAPH_CTOR` / `RUNTIME_CTOR` to
store `napi_ref` values via `napi_create_reference(env, class, 1,
&mut ref)`, and resolving them back to a scope-bound `napi_value`
in each static-method callback via `napi_get_reference_value`.
After the fix all 15 vitest tests pass.

A `lessons.md` entry was added so the next napi addon doesn't
re-discover this:  `napi_value` handles are scope-bound; cross-call
persistence requires `napi_ref`.  `font-parser-node` has the same
latent bug in its `FONT_FILE_CTOR` storage but no test there
actually reaches `napi_new_instance` (every input rejects earlier),
so the bug never fired.

### Out of scope

* **Per-platform prebuilt distribution.**  The `optionalDependencies`
  + per-platform npm package pattern (per MX07 §"Distribution
  model") is its own change — out of scope for v0.  This package
  currently resolves the `.node` artifact at its in-repo location;
  publishing to npm requires the wrapper to ship its own copy and
  pick the right one at install time.
* **Browser support.**  ARCH02 explicitly exempts the browser; for
  browser execution see the future `matrix-ir-ts` /
  `matrix-runtime-ts` / `matrix-webgpu-ts` packages.
* **Async `runAsync()`.**  v1+ once profiling shows it matters.
* **The `typescript/matrix` package refactor.**  That's MX08
  (separately scoped) — replaces `typescript/matrix`'s
  `CpuMatrixBackend` with a thin shim into this wrapper.
