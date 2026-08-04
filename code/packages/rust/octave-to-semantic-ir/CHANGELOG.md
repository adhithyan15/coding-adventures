# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- `tests/oracle.rs`: HML01 §7 oracle/golden testing against native
  `octave-runtime`, the direct sibling of `matlab-to-semantic-ir`'s own
  `tests/oracle.rs` (same `Case`/`ground_truth`/`compiled`/`normalize`
  shape, same `OpenOptions::create_new(true)` temp-file handling). Confirms
  `coding_adventures_octave_runtime::eval` uses the identical ground-truth
  convention as `matlab-runtime`'s (`disp` is a no-op; the unsuppressed
  `name = value` echo is the only working display path) since
  `octave-runtime::eval` delegates to the very same
  `matlab_runtime::Interpreter::feed` after its `octavify` pre-pass.
- 6-case corpus, deliberately restricted to Octave-only syntax `octavify`
  actually rewrites (re-testing plain MATLAB arithmetic would prove nothing
  this crate's existing shim-then-delegate unit tests don't already prove
  structurally):
  - `hash_comment_literal_arithmetic` — a `#` comment (MATLAB uses `%`).
  - `bang_equals_not_equal_comparison` — `!=` (MATLAB uses `~=`).
  - `bang_negation_on_comparison` — `!` (MATLAB uses `~`), applied to a
    parenthesized comparison. At the time this test was added, negating a
    *bare numeric variable* directly was a confirmed, deliberately-excluded
    gap this case sidestepped — since fixed; see the "Fixed" section
    below.
  - `if_else_endif` — Octave's `endif` block terminator.
  - `for_loop_accumulator_endfor` — Octave's `endfor` block terminator.
  - `while_loop_accumulator_endwhile` — Octave's `endwhile` block
    terminator; also a direct regression check, through Octave syntax, that
    the while-loop-accumulator bug fixed in `semantic-ir-to-javascript`'s
    shared `numOf` helper (matlab-to-semantic-ir PR #8572) benefits this
    frontend too, since it shares 100% of its lowering/codegen with
    `matlab-to-semantic-ir`.
  - All 6 cases pass against `octave-runtime`'s ground truth; no new bug in
    the `octavify` shim itself was found.

### Fixed

- **FIXED (was: "Found", below): negating a bare numeric variable through
  `!`/`~` disagreed with Octave semantics for zero.** `~x`/`!x` with
  `x = 0` used to compile to `false` instead of Octave's real `1` (true).
  Fixed entirely in `matlab-to-semantic-ir` (this crate has no `src/
  lower.rs` of its own, so the fix — a new `to_matlab_condition` lowering
  helper applied at `~`, `if`, `while`, and `&&`/`||` — applies here
  unchanged); see that crate's own `CHANGELOG.md` entry for the full
  root-cause writeup, including confirmation that Ruby/Python/JS's own
  truthiness reliance is unaffected. Two new corpus cases exercise the fix
  through Octave's own `!` spelling specifically (proving the `octavify`
  `!` → `~` rewrite composes correctly with the fix, not just the `~`
  spelling MATLAB's own oracle file already covers):
  `bang_negation_on_bare_zero_is_true` (`!0` → `1`) and
  `bang_negation_on_bare_nonzero_is_false` (`!5` → `0`).
  `bang_negation_on_comparison`'s doc comment is trimmed to drop the
  "confirmed, excluded gap" framing it no longer needs.

### Found (confirmed, not fixed here — test infrastructure only)

- Confirmed, by inheritance from `matlab-to-semantic-ir` (unchanged, since
  this crate shares 100% of its lowering/codegen): the integer-literal-
  division-floors bug and the missing `Feature::ShortCircuit` declaration
  for `&&`/`||`/`&`/`|`, both still open per that crate's own
  `tests/oracle.rs` module doc and `CHANGELOG.md`. `CORPUS` avoids both.

## [0.1.0] - 2026-07-12

### Added

- Initial release — Stream A rollout item 5
  ([`HML01`](../../../specs/HML01-math-to-semantic-ir.md) §5): a thin
  wrapper reusing `octave-runtime`'s `octavify` source-compatibility shim
  and `matlab-to-semantic-ir::compile_source` wholesale. No new grammar, no
  new SIR node kinds — Octave's departure from MATLAB is a small, local set
  of surface forms (`#` comments, `endif`/`endfor`/`endwhile`/
  `endfunction`/`endswitch`/`end_try_catch`, `!=`/`!`), not a different
  language.
- Single public entry point, `compile_source(source, module_name) ->
  Result<semantic_ir::Module, MatlabLowerError>` — deliberately no
  `compile(tree, ...)` variant, since there is no Octave-specific CST to
  hand in (the shim normalizes text, not a tree).
- 9 tests: the shim-then-delegate wiring for every normalized construct
  (`#` comments, all six `endX` terminators, `!=`/`!`), a string/comment-
  awareness regression (`#`/`!` inside a string literal must not be
  rewritten), a plain-MATLAB-passthrough sanity check, error propagation
  for both an out-of-scope MATLAB construct and an Octave-only construct
  `octavify` does not normalize (`do...until`), and one full end-to-end
  test exercising every shim rule at once through `semantic_ir::validate`.
