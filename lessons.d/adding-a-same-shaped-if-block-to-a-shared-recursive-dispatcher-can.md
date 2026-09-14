# Adding a same-shaped `if` block to a shared recursive dispatcher can overflow the stack on ONE platform only, even with correct logic and green local tests

`adj-lang`'s `expand_rec` (`code/packages/rust/adj-lang/src/lower.rs`) recurses
one native stack frame per AST level, guarded by `FORMULA_MAX_NODE_DEPTH`
(descend-then-check, so recursion never exceeds the cap). Its own doc comment
already flagged the margin as tight: "a *few hundred* debug-build frames
already approach the default ~2 MiB worker-thread stack." Adding FL-11's
`min(a, b)`/`max(a, b)` built-ins as two more `if name == "max" { .. let a =
..; let b = ..; }` / `if name == "min" { .. }` blocks — each an exact structural
copy of the pre-existing `mod` block, just with different names and a different
wrapped node — compiled clean, passed `cargo clippy`, and passed all 252 unit
tests locally (Windows). It still made macOS CI's `build` job fail for real:
`deep_operator_spine_trips_the_nesting_guard_not_the_stack` (a test asserting
the depth guard fires *before* the native stack does, on a 400-level spine)
aborted with `SIGABRT: process abort signal` / "has overflowed its stack" — the
guard never got a chance to fire, because 96 recursion frames of the new,
slightly larger `expand_rec` no longer fit in the runner's ~2 MiB thread stack.
Linux and Windows builds passed; only macOS's `build` job failed, and it was a
REAL failure (`gh api .../jobs/<id> --jq '{status,conclusion}'` showed
`"conclusion": "failure"`, not `"cancelled"`), so [[Check cancelled vs failed CI
jobs before debugging]] correctly routed to log-reading instead of a rerun.

Root cause: in an unoptimized (`cargo test`/debug) build, rustc/LLVM does not
reliably coalesce stack slots across sibling `if`-blocks within one function —
each block's locals (here, two `ExprAst` bindings) can each claim their own
space in the function's frame, so three near-identical blocks (`mod`, `max`,
`min`) cost roughly 3× one block's worth of frame size, not 1×. Fix: merge
same-shaped dispatch blocks that recognize different built-in NAMES but expand
identically (exactly two args, expand both, wrap in the built-in's own node)
into ONE `if name == "mod" || name == "max" || name == "min" { .. }` block with
ONE shared pair of locals, dispatching on `name` only at the point of
constructing the result node. This restored the exact frame footprint `mod`
alone had before the change — no logic change, same test outcomes, macOS green
on the next CI run.

Lessons:
1. **A function that recurses through itself (a walker/interpreter/expander)
   has a stack-frame-size budget shared by ALL its branches, not just the one
   your change touches.** Adding a new `if`/`match` arm with its own locals to
   such a function is not "adding code," it's "growing every future call's
   frame" — measure against the SAME kind of margin a recursion-depth cap
   documents (see the "8 MB vs 1 MB stack" lesson above; this is that lesson's
   sibling for "more local variables per frame" instead of "more frames").
2. **When several `if`/`match` arms share an identical shape (same arity check,
   same recurse-both-children pattern, different only in which node they
   construct), merge them into one arm with a final dispatch on the
   discriminant.** This is not just DRY — in a debug build it can be the
   difference between a stack-safety margin holding and not.
3. **A green local test suite does not confirm a recursion-adjacent change is
   stack-safe on every target platform.** Different OS default thread stack
   sizes (and different compiler stack-slot-reuse behavior per platform) mean
   the same source can pass on Linux/Windows and abort on macOS (or vice
   versa) purely from a stack-frame-size change, with zero logic difference.
   If a crate has an existing depth/recursion guard whose doc comment already
   calls out a tight stack margin, treat any change to the guarded walker's
   own function as touching that margin, and re-run (or at least reason
   about) its dedicated stack-overflow regression test specifically.
4. `SIGABRT` / "has overflowed its stack" from a `#[test]`-harness thread
   (Rust's default per-test-thread stack is ~2 MiB unless `RUST_MIN_STACK` or
   an explicit `Builder::stack_size` overrides it) is the same failure class as
   Windows' `0xC00000FD` — a real stack overflow, not a flaky/cancelled CI job.
