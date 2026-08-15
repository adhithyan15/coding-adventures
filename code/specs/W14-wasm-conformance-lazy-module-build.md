# W14 — a per-module build failure should not abort a whole `.wast` file

## Purpose

Logged as task #76 while working SIMD PR1b-3 (`code/specs/
W13-wasm-simd-v128-first-slice.md`'s follow-up scope): vendoring any real
root-level `simd_*.wast` file was investigated and found to be currently
impossible, for a structural reason that has nothing to do with SIMD
specifically and will recur for every future opcode-family epic (threads
beyond plain atomics, exceptions, GC, the component model, ...). This spec
fixes the structural reason, not the SIMD coverage gap itself (that stays
its own, separate, ongoing effort — `code/specs/
W07-wasm-post-mvp-epics.md`'s epic list).

## The concrete problem, confirmed by direct inspection

`code/packages/rust/wasm-wast-parser/src/script.rs:125-128`:

```rust
pub fn parse_script(src: &str) -> Result<Vec<Directive>, WastParseError> {
    let exprs = parse_source(src)?;
    exprs.iter().map(parse_directive).collect()
}
```

`.collect::<Result<Vec<_>, _>>()` short-circuits on the first `Err`. Every
`(module ...)` top-level form is built **eagerly** inside `parse_directive`
(script.rs:142): `"module" => Ok(Directive::Module(build_module_directive(e)?))`.
`build_module_directive` real-parses the module's full instruction stream —
so if directive #40 out of 60 in a file is a module using one instruction
this repo hasn't implemented yet, `parse_script` returns `Err` for the
**whole file**, and `wasm-conformance` never sees any of the 60 directives —
not even the 39 that would have graded fine.

This is not hypothetical for SIMD specifically. Checked against the real,
pinned-commit corpus (confirmed by grep, not assumed):

- `simd_const.wast` uses `i64x2.add` exactly once (its `i64x2.inc_smin`
  test, one line out of 1668) — an opcode this repo's SIMD first slice
  doesn't implement. That one line aborts grading the file's other ~445
  `assert_return`/`assert_malformed` directives, most of which exercise
  `v128.const` parsing edge cases this repo's `wasm-wast-parser` DOES
  already handle correctly (SIMD PR1b-2).
- `simd_splat.wast`, `simd_i32x4_arith.wast`, `simd_i32x4_cmp.wast` each
  reference 15-20 further opcode families beyond this repo's 5-opcode
  slice, for the same reason.

The "partial opcode coverage, grade the rest `NotYetSupported`" pattern
that worked cleanly for every prior WASM epic's first PR (WASM17's
funcref/externref, WASM18's plain atomics, WASM16's tail calls) does not
apply here, because those gaps were all caught **inside** `wasm-execution`
or `wasm-validator` at *directive-grading* time — never inside
`wasm-wast-parser` at *whole-file-parse* time. `Directive::Module`'s eager
build is the one place in this pipeline where a per-directive capability
gap becomes a whole-file failure.

## Design

### `Directive::Module` wraps a `Result`, not a bare value

`code/packages/rust/wasm-wast-parser/src/script.rs:112-123` — currently:

```rust
pub enum Directive {
    Module(WasmModule),
    ...
}
```

becomes:

```rust
pub enum Directive {
    Module(Result<WasmModule, String>),
    ...
}
```

`parse_directive`'s `"module"` arm changes from `build_module_directive(e)?`
(propagate) to `build_module_directive(e).map_err(|e| e.to_string())` (capture).
`parse_script`'s `.collect()` now only ever short-circuits on a genuine
**syntax**-level failure from `parse_source(src)?` (unbalanced parens,
invalid UTF-8, an unrecognized top-level form) — a truly malformed script,
where directive boundaries themselves can't be reliably identified, still
can't be partially graded, and that's unchanged and correct. Only a
`(module ...)` form's own instruction-stream build failure — a real,
well-formed S-expression that names an opcode/construct this repo doesn't
support — degrades to per-directive data instead of a hard abort.

This is a breaking change to `wasm-wast-parser`'s public `Directive` enum.
Per this repo's own stated preference (break compatibility freely, no
back-compat shims), this ships as a plain signature change with the one
real consumer (`wasm-conformance`) updated in the same PR — confirmed via
grep that no other crate in the workspace matches on `Directive::Module`.

### `wasm-conformance`'s `Executor` grades a failed `Directive::Module` as `NotYetSupported`, and poisons the "current module" slot on ANY non-success outcome

`code/packages/rust/wasm-conformance/src/lib.rs:241-264`'s existing
`Directive::Module(module)` arm gains an outer match on the new `Result`:

```rust
Directive::Module(module_result) => {
    self.current_module_status = None;
    self.registry.borrow_mut().remove(&None);
    match module_result {
        Err(e) => {
            let reason = format!(
                "module failed to parse/build (real capability gap, not a bug): {e}"
            );
            self.current_module_status = Some(reason.clone());
            DirectiveOutcome::NotYetSupported(reason)
        }
        Ok(module) => {
            // existing validate/instantiate logic, unchanged in shape --
            // see "poisoning on every non-success path" below for what
            // each of its 3 existing outcomes additionally does now.
        }
    }
}
```

A directly-adjacent, pre-existing correctness gap the investigation
surfaced (not introduced by this change, but now exercised far more often
once module-build failures are common instead of whole-file-aborting):
today, the registry's `None` ("current module") slot is **only** written
on the success path (`self.registry.borrow_mut().insert(None, ...)`) — it
is never cleared on any failure path. A module that fails structural
validation, fails to link, or traps during instantiation currently leaves
a **stale previous module** registered as "current," so a later `invoke`
or `register` with no explicit module name silently operates on the wrong
module instead of failing loudly. This spec fixes that as part of the same
change, since the new build-failure path needs the identical clearing
behavior anyway: `self.registry.borrow_mut().remove(&None)` runs
unconditionally at the top of the `Directive::Module` arm (shown above),
before the result is matched, so every one of this arm's 4 possible
outcomes (build failure, structural-validation failure, link failure,
instantiation trap) starts from a clean slate — `register`'s existing
`registry.borrow().get(&None)` (lib.rs:266-283) then naturally sees "no
current module" and hits its existing `Fail("register: no current module
to register")` path with zero changes to `Register`'s own logic, for the
structural-validation-`Fail` and instantiation-`Trap` cases (genuine
failures, not capability gaps — `register` failing loudly here is
correct).

`current_link_failed: Option<String>` (lib.rs:219-229) is renamed to
`current_module_status: Option<String>` and broadened to cover all 3
"capability gap, not a real failure" cases uniformly (build failure, link
failure) rather than only link failure — its existing `is_link_error`
string-matching stays exactly as-is for classifying instantiation errors,
just feeding the same broadened field. `run_action`'s existing two read
sites (lib.rs:432-437, :463-468) are updated for the rename only, no
behavior change there. `Directive::Register`'s `None`-module-name arm gains
one new check: if `current_module_status.is_some()` when the registry slot
is empty, grade `NotYetSupported(reason)` (the current module is missing
*because of* a capability gap, and that should propagate, matching this
repo's established philosophy of not converting "we haven't built X yet"
into a hard `Fail`) instead of the existing hardcoded `Fail` (reserved for
the genuine case: no `current_module_status` set, registry slot empty,
because no module directive ever ran, or ran and was never a "current
module" to begin with — a real test-script-structure issue).

### What does NOT change

- `AssertInvalid`/`AssertMalformed`/`AssertUnlinkable`'s module handling —
  confirmed by direct inspection (script.rs's own module doc comment,
  verified against the actual code) to already be lazy (`ModuleSource`,
  built only at grade time inside `wasm-conformance`'s own
  `grade_assert_*` functions, which already treat a build failure as a
  graded outcome, never a script-abort). This spec touches none of that.
- Genuine tokenizer/S-expression syntax errors (`parse_source`) still hard-
  abort the whole file. Not a regression — a truly malformed `.wast` file
  can't have its directive boundaries reliably identified at all, so
  there is nothing meaningful to partially grade.
- Every other directive kind's own literal/argument parsing (e.g. a
  malformed numeric literal inside an `assert_return`'s own args) is
  **out of scope** — confirmed the SIMD corpus files driving this spec
  don't exercise that path; if a future corpus does, it's a separate,
  narrower follow-up to this same design, not bundled in here.

## Staged commits

1. This spec (sign-off only).
2. Implementation: `wasm-wast-parser`'s `Directive::Module` shape change +
   `parse_directive`'s `map_err` change (`wasm-wast-parser`'s own test
   suite gains a case proving a script with one unbuildable module and
   several buildable directives around it now parses successfully, with
   the failure captured per-directive); `wasm-conformance`'s `Executor`
   update (registry-clearing, `current_module_status` rename/broadening,
   `Register`'s new NotYetSupported branch) with new tests proving: (a) a
   script with an unbuildable module followed by unrelated, independently
   buildable directives now grades the independent ones for real instead
   of reporting zero results for the whole file; (b) `invoke`/`register`
   against a broken current module now grade `NotYetSupported`, not a
   silent pass against a stale prior module or a hard `Fail`.
3. Once merged, vendoring at least one real root-level `simd_*.wast` file
   becomes viable for real: its own unsupported-opcode directives grade
   `NotYetSupported` per-directive (or the individual module directives
   using them do), while the directives this repo's 5-opcode slice
   already covers grade for real. That vendoring pass itself is follow-on
   work, tracked separately (task #76's remaining scope), not part of
   this spec's own implementation PR.

## Verification

- `wasm-wast-parser`: a hand-written `.wast` snippet with 3 modules — one
  buildable, one using a deliberately unrecognized instruction name, one
  buildable again, each followed by an `assert_return`/`invoke` targeting
  it — parses successfully as a whole (`parse_script` returns `Ok`), with
  the middle module's `Directive::Module` payload being `Err(_)` and the
  other two being `Ok(_)`.
- `wasm-conformance`: the same 3-module script, run through the real
  `Executor`, produces `Pass` for the two buildable modules' own
  `assert_return`s and `NotYetSupported` for everything targeting the
  broken one (its own module directive, any bare `invoke`/`register`
  against it before the next real module directive) — never a `Fail` for
  a capability gap, never a silent pass against the wrong module.
- A TEMP-REVERT-CHECK-style regression test proving the pre-existing
  stale-registry bug is real and now fixed: temporarily reverting the
  `registry.borrow_mut().remove(&None)` line reproduces a concrete false
  result (an `invoke` against a module that failed to build/link
  succeeding against the PREVIOUS module's instance instead of grading
  `NotYetSupported`), confirming the fix is load-bearing, then restore it.
- `cargo test -p wasm-wast-parser -p wasm-conformance` and
  `cargo clippy` clean.
