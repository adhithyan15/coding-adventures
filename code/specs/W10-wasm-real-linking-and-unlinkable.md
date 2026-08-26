# W10 — Real Module Linking and `assert_unlinkable` (WASM05)

## Why

`wasm-conformance`'s `Executor::execute` currently grades **every**
`assert_unlinkable` directive `NotYetSupported`, unconditionally, with the
reason `"WasmRuntime::instantiate never fails on an unresolved import, so
linking failure can't be observed yet"`. That's literally true:
`WasmRuntime::instantiate` (`wasm-runtime/src/lib.rs:1146`) never returns
`Err` for an import — an unresolved function import just gets
`host_functions.push(None)` (only fails later, at *call* time, if that
specific import is ever actually invoked), and an unresolved memory,
table, or global import silently **fabricates a default value from the
declared type** instead of erroring. There is currently no path through
which a WASM module's imports failing to link can be observed at all.

This spec designs the fix: give `instantiate` a real link-failure path,
and wire `wasm-conformance`'s existing per-script module registry up as a
real, if narrow, host — enough to make `assert_unlinkable` gradable for
real and to make genuinely linkable modules actually link (not just fail
faster).

## What the real corpus actually needs (verified, not assumed)

None of the 60 `.wast` files already vendored in
`wasm-conformance/tests/fixtures/testsuite/` contain any
`assert_unlinkable` directive at all — this is genuinely new vendoring,
not a baseline-regen-only PR. The real upstream `imports.wast` (fetched
at this repo's pinned commit, `28864811cf03bdbf880733786148feaba339582d`,
for real inspection before writing this spec) has **93** `assert_unlinkable`
cases. Reading them directly shows their actual shape is narrower and
more tractable than "arbitrary WASM linking":

```wat
(module
  (func (export "func"))
  (func (export "func-i32") (param i32))
  ...
  (global (export "global-i32") i32 (i32.const 55))
)
(register "test")
...
(assert_unlinkable
  (module (import "test" "unknown" (func)))
  "unknown import"
)
(assert_unlinkable
  (module (import "test" "func-i32" (func)))
  "incompatible import type"
)
```

The overwhelming majority of `imports.wast`'s `assert_unlinkable` cases
import from `"test"` — a module `register`ed **earlier in the same
script** — and are rejected either because the named export doesn't
exist, or because it exists with the wrong kind/type. This is *exactly*
`wasm-conformance`'s existing `Executor.registry: HashMap<Option<String>,
Rc<RefCell<WasmInstance>>>` mechanism (already built for `invoke`/`get`
addressing across `register`ed modules) — it just isn't wired up as a
real `HostInterface` for `instantiate` to consult. A handful of cases
(2 of 93, plus a few `assert_return` cases like `(invoke "print32"
...)` that depend on a module that itself imports from `spectest`)
reference `"spectest"` — the official test harness's own host-provided
fixture module (`print_i32`, memories, tables, globals for test
purposes) — which this repo does not implement and is **explicitly out
of scope** here (see below). Because grading never string-matches the
*expected* message text against ours (`assert_trap`'s own established
precedent: only *that* a trap/link-failure occurred is checked, not its
wording), an import from `"spectest"` still correctly grades as a link
failure (`"unknown import"` in spirit, even without a real spectest
implementation) for the `assert_unlinkable` cases that reference it —
only the `assert_return` cases that need `spectest`'s *real* behavior
are affected, and those are handled by the cascading-`NotYetSupported`
design below, not silently mis-graded.

## Scope

### `wasm-runtime`: `instantiate` gains a real link-failure path

- New error path, distinct from `TrapError` (a *runtime* fault) — reuse
  `TrapError` as the return type (no need for a new error enum; every
  existing caller already handles `Result<_, TrapError>`), but with
  messages that clearly read as *link* failures (`"unknown import:
  {module}.{name}"`, `"incompatible import type for {module}.{name}:
  expected {declared}, found {actual}"`), matching this crate's existing
  convention of self-authored, capability-gap-shaped error text (see
  `wasm-conformance`'s own `NotYetSupported` messages) rather than
  needing a new distinguishing type.
- For each import kind, resolution failure or a declared/actual type
  mismatch now returns `Err`, replacing today's "fabricate a default
  value" and "push `None`, fail later at call time" fallbacks entirely:
  - **Function**: `resolve_function` returning `None` → `Err`. Returning
    `Some(f)` where `f.func_type() != &module.types[*type_idx as
    usize]` → `Err`. (`HostFunction::func_type()` already exists —
    `wasm-execution/src/lib.rs:695` — this check is genuinely cheap.)
  - **Memory**: `resolve_memory` returning `None` → `Err` (no more
    "just allocate one from the declared type instead"). Returning
    `Some(m)` whose actual `(min, max)` don't satisfy the *declared*
    limits (`actual.min >= declared.min &&` `declared.max.is_none() ||
    actual.max.is_some() && actual.max <= declared.max` — the real
    spec's own limits-compatibility rule, not exact equality) → `Err`.
  - **Table**: same shape as memory, plus `element_type` must match
    exactly.
  - **Global**: `resolve_global` returning `None` → `Err`. Returning
    `Some((gtype, _))` where `gtype != declared` → `Err`.
- **Verified safe for existing real callers**: `WasiEnv::resolve_function`
  (`wasm-runtime/src/lib.rs:444`) never actually returns `None` for its
  own module (`"wasi_snapshot_preview1"`) — every name not in its
  explicit match arms falls through to `EnosysFunc`, a real
  `HostFunction` that traps with ENOSYS *if called*, not at resolve
  time. `WasiEnv` only returns `None` when `module_name !=
  "wasi_snapshot_preview1"` — a genuinely unresolvable import, which
  *should* fail to link under the real spec. So none of `brainfuck-wasm-
  compiler`/`nib-wasm-compiler`/`twig-to-wasm`/`twig-demo`/`lang-aot`'s
  existing WASI-based execution paths can regress from this change —
  confirmed by reading `WasiEnv::resolve_function`'s full match directly,
  not assumed.

### `wasm-conformance`: registry-backed `HostInterface` + cascading capability gaps

- New `RegistryHost<'a>` implementing `HostInterface`, borrowing
  `Executor`'s existing `registry: &'a HashMap<Option<String>,
  Rc<RefCell<WasmInstance>>>`. `resolve_function`/`resolve_memory`/
  `resolve_table`/`resolve_global` all follow the same shape: look up
  `registry.get(&Some(module_name.to_string()))`, then find an export by
  `(name, kind)` on that instance. No special-casing for "which reason it
  failed" (unknown module vs. unknown export vs. wrong kind) — every
  failure path just returns `None`, since `assert_unlinkable` grading
  never needs to distinguish *why*, only *that*, matching this crate's
  own established "don't string-match trap/error text" discipline.
- **Function resolution is a real cross-instance call, not just a type
  declaration.** The resolved `HostFunction` wrapper holds the callee's
  own `Rc<RefCell<WasmInstance>>` + export index, and its `call()` method
  invokes `WasmRuntime::call_typed` against the *callee's own* instance
  state (own memory, tables, globals, func_bodies) — reusing existing,
  already-tested machinery rather than building new interpreter
  internals. The `memory: Option<&mut LinearMemory>` parameter
  `HostFunction::call` normally receives (the *caller's* memory, used by
  e.g. `FdWriteFunc` to read/write WASI pointers) is unused here; a
  cross-module call operates entirely on the callee's own state.
- **Known, explicitly out-of-scope limitation**: mutual/circular
  cross-module calls (module A's import resolves into module B, whose
  own body calls back into an import resolving into A) will panic on a
  `RefCell` double-borrow, since both instances are held via
  `Rc<RefCell<..>>` and a re-entrant `borrow_mut()` on the same instance
  aborts. None of `imports.wast`'s real cases are circular (every
  `register`ed module in this file is one used only as a plain, one-way
  import source), so this doesn't block vendoring it, but it's a real
  architectural gap worth naming rather than leaving to surface as a
  confusing panic later. A future PR that needs real mutual recursion
  across instances should revisit this — not a concern this PR needs to
  solve.
- `Executor` builds an ephemeral `WasmRuntime::with_host(Box::new(
  RegistryHost { registry: &self.registry }))` per module instantiation
  (a `WasmRuntime` is cheap and effectively stateless besides its `host`
  field, so constructing one per `(module ...)` directive is the
  simplest way to thread the current registry state through without
  restructuring `WasmRuntime` itself) rather than mutating the
  `Executor`'s existing single `runtime: WasmRuntime` field's host in
  place.
- **`Directive::Module` failure grading needs a real/gap distinction,
  not a blanket `Trap`.** Today `instantiate` never fails, so
  `Directive::Module`'s `Err(e) => DirectiveOutcome::Trap(...)` arm has
  never actually fired for a link reason. Once `instantiate` can return
  a real link error, a module that *should* link under the real spec but
  references an unimplemented host module (`spectest`) will now fail to
  link in a way that is a genuine capability gap, not a bug — grade
  `Directive::Module`'s failure as `NotYetSupported` when the failure
  is a link error (missing/mismatched import), reserving `Trap` for a
  genuine runtime fault during data/element-segment initialization
  (`instantiate`'s existing `evaluate_const_expr`/`write_bytes`/`Table
  ::set` calls, which can still trap for real, unrelated to linking).
  `Executor` needs a new `current_link_failed: bool`
  (generalizing/replacing today's `current_has_imports`) so that
  subsequent `invoke`/`get`/`assert_return` actions targeting a module
  that failed to link for this reason also cascade to
  `NotYetSupported` rather than a confusing "no module registered"
  `Trap`.
- Vendor the real `imports.wast` from the pinned commit. Every directive
  this repo can't yet grade for real (the handful of genuine `spectest`
  dependents) grades `NotYetSupported`, not `Fail` — matching this
  crate's `assert_invalid`/`assert_malformed`/atomics-`notify`/`wait`
  precedent of "capability gap, not a wrong answer."

## Explicitly out of scope

- **A real `spectest` host module.** The official test harness's own
  fixture module (`print_i32`/`print_i64`/etc., plus test memories,
  tables, and globals) — a separate, self-contained feature (a second
  `HostInterface` implementation) with no dependency on this PR's
  registry-linking work. Only ~2 of `imports.wast`'s 93
  `assert_unlinkable` cases and a handful of its `assert_return` cases
  need it; both correctly grade (link-failure via "no host resolves
  `spectest`", or `NotYetSupported` respectively) without it.
- **Mutual/circular cross-instance calls** — see the `RefCell`
  double-borrow limitation above. Not exercised by anything vendored
  this PR; a future PR's problem if it ever needs to be, not a design
  flaw of the specific slice this PR ships.
- **`exports.wast`/`linking.wast`'s deeper module-reinstantiation and
  multi-copy linking semantics** (re-`instantiate`-ing the same module
  multiple times as distinct sibling instances, tests that specifically
  probe *instance identity*) — not examined for this PR; only
  `imports.wast` is vendored here. A future PR can extend the same
  `RegistryHost` mechanism to cover them once actually read and
  scoped the same way this spec scoped `imports.wast`.

## Verification plan

- Unit tests in `wasm-runtime` for the new link-failure path: unresolved
  function/memory/table/global import (each independently), a function
  type mismatch, a memory/table limits mismatch, and a positive case
  (import resolves and type-matches, `instantiate` still succeeds) —
  proving the change is a real failure path addition, not a regression
  on the success path.
- Unit tests in `wasm-conformance` for `RegistryHost`: resolving a
  function/memory/table/global export from a `register`ed sibling
  module, a real cross-instance function *call* round-trip (not just
  resolution), and the `Directive::Module`/cascading-`NotYetSupported`
  behavior for a module that fails to link because of a genuine
  capability gap (an unimplemented `spectest` import).
- Vendor `imports.wast`, regenerate the conformance baseline, and diff
  against the pre-change baseline exactly like every WASM04/06/08/17/18/19
  PR this session — zero regressions on any already-parsing file is the
  primary correctness signal, plus a real, non-zero `assert_unlinkable`
  pass count on the newly vendored file (not just "it parses").
- `/security-review` before push — particular attention to the new
  cross-instance call path: a `HostFunction` wrapper now lets one WASM
  module's execution reach into another instance's memory/tables/globals
  by construction, so confirm there's no way for a *caller* module to
  observe or corrupt a *callee* instance's state beyond the callee's own
  declared exports (e.g. no accidental sharing of the caller's
  `LinearMemory` into the callee's execution context, and the
  `RefCell` double-borrow panic path is a clean, non-exploitable panic
  — not a silent memory-safety issue — if ever triggered by a future
  vendored file).

## W28 Addendum — real shared memory/table storage (2026-08-26)

This spec's original scope covered function and global import linking
end-to-end (real cross-instance call, real type/limits checking) but left
memory and table imports resolving to an independently-CLONED value —
`RegistryHost::resolve_memory`/`resolve_table`'s own doc comments named
this explicitly as a deliberate, deferred limitation at the time ("None
of the corpus vendored so far exercises [shared mutation]... revisit if a
future vendored file needs it"). `elem.wast`/`instance.wast`/
`linking0.wast`/`linking1.wast`/`linking3.wast`/`load1.wast` are exactly
that future need.

**The fix, in one sentence**: `wasm-execution`'s `LinearMemory`/`Table`
moved their mutable storage (`data`+`current_pages` for memory,
`elements` for tables) behind `Rc<RefCell<..>>`, so `#[derive(Clone)]` —
which `HostInterface::resolve_memory`/`resolve_table`'s existing
`.cloned()` call sites already relied on — now shares the underlying
storage instead of deep-copying it. No change was needed to
`wasm-runtime::instantiate()`'s import-resolution path itself, nor to
`wasm-conformance::RegistryHost`: both already just pass through whatever
value `resolve_memory`/`resolve_table` return, so once that value
genuinely shares storage, the existing code became correct for free. See
`wasm-execution`'s own CHANGELOG (0.9.73) for the full field-level design
(including why the two pre-existing raw-pointer `copy_between` primitives
for `memory.copy $dst $src`/`table.copy $dst $src` self-aliasing remain
sound unchanged) and `wasm-runtime`'s own CHANGELOG (0.6.13) for a second,
previously-unobservable bug this same fix surfaced and also fixes
(non-atomic per-segment active element-segment application).

**Still explicitly out of scope, discovered while vendoring
`linking0.wast`/`linking3.wast`**: a table entry is a bare `u32` function
index, meaningful only within whichever instance's OWN function-index
space is currently executing `call_indirect`. A funcref written into a
now-genuinely-SHARED table by one module, then `call_indirect`-invoked
through a DIFFERENT module that imported it, resolves against the WRONG
instance's function space — correct within one instance (the overwhelming
common case), wrong across a shared-table boundary. This needs real
cross-instance function IDENTITY for table entries — the same class of
problem `WasmInstance::tag_identities` (W23) already solves for exception
tags, but requiring genuine cross-instance CALL DISPATCH (like
`wasm-conformance`'s existing `CrossModuleFunction` already provides for
plain function IMPORTS), not just equality comparison. Not designed or
implemented here; a future PR's problem, same as this spec's own original
"mutual/circular cross-instance calls" and "deeper module-reinstantiation"
out-of-scope items above.

`instance.wast` was investigated but not vendored: it needs a `(module
definition $M ...)` / `(module instance $I1 $M)` generative-instantiation
directive form `wasm-wast-parser`'s script grammar has zero support for
at all — a distinct, self-contained grammar-plus-`Executor` feature, not
blocked by the storage-sharing fix above, just out of scope for this
addendum.
