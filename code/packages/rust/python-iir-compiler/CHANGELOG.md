# Changelog

## Unreleased — grammar-tools automatic insertion (E6 `@asi`)

### Changed
- `tests/cst_contract.rs`'s exhaustive `referenced_tokens` match covers the
  new `GrammarElement::AutoInsert`, whose inserted token and closers are
  token names.
## Unreleased - `main` is sized to vm-core's frame limit (#18419)

### Fixed

- **A program whose `main` needs more registers than a vm-core frame holds
  is refused by the compiler.** vm-core refuses such a function at load time
  (`VMError::RegisterLimitExceeded`, more than 65,536 registers). The
  default budgets cannot reach that, but the trusted ones (1 MiB, 256 Ki
  tokens, as PR7c-py, #18391, allows) can: 257 lines of
  `print(1+…+1)` with 128 ones and one `print(1)` compiled, then failed at
  VM load. It is now `FrontendError::Budget { which: "registers per
  function", limit: 65536, function: Some("main"), start: Some((258, 1)) }`.
- The walker counts `main`'s registers as vm-core does (parameters, none for
  `main`, plus distinct `dest`s) as it emits them, and reserves the one its
  closing `return 0` needs, so the refusal names the statement that left no
  room, not the epilogue. The check is exact: 65,536 registers compile and
  run on vm-core, 65,537 do not.
- `finish` then puts every function of the module (the float helpers too)
  through vm-core's own `VMFrame::check_register_limit`, so a module the
  compiler returns is one vm-core will load.

### Added

- `MAX_REGISTERS_PER_FUNCTION`, re-exported from vm-core's
  `VMFrame::MAX_FRAME_REGISTERS` rather than restated.
- `FrontendError::Budget` gains `function: Option<String>` and
  `start: Option<(usize, usize)>`, set for the register budget and `None`
  for every other budget. `Display` appends `` in function `main` at 258:1 ``
  when they are present; the other budgets read as before. A caller that
  builds a `Budget` must set both (no caller outside this crate does).
- `tests/register_limit.rs`: the boundary on the VM column (65,536 compile
  and run, 65,537 refused with the position, a statement mid-line), and every
  function of a float module fits a frame. The trusted-budget step test in
  `release_api` now also requires one more float line than fits to be
  refused, and no longer says the VM drops the 65,537th register (it has
  refused it since #18402).

## Unreleased - the crate owns its `repr` oracle (LANG83 PR7e)

- `write_float_matches_the_pr6a_repr_byte_for_byte` (was
  `…_pr6a_python_repr_…`) compares the 1,248 values with a test-local
  `repr_oracle` over `vm_core::float_format::shortest_digits`, because LANG83
  PR7e removed vm-core's `python_repr` with the `write_f64_py` builtin; it
  still also runs host `python3` when installed.
  `the_repr_oracle_matches_host_python3` pins the oracle to strings host
  `python3` printed. No code change: the release path has emitted no
  `write_f64_py` since 0.4.0.

## 0.4.0 - Unreleased - the compiler writes `repr(float)` (LANG83 PR7c-py)

LANG83 PR7 says the IIR names no source language. This release takes
Python's float layout out of every backend and puts it in the module, as
0.7.0 of `javascript-iir-compiler` did for `String(x)`.

### Changed

- **`write_f64_py` is no longer emitted.** A release-path module that prints
  a float carries four functions this compiler writes (new `float_repr`
  module): `__py_write_float(x: f64) -> i64`, which is CPython 3's
  `float_repr` (`nan`, `inf`/`-inf`, `0.0`/`-0.0`, fixed notation when
  `-4 <= e < 16` with `.0` on an integral value, else `d.ddde±XX` with at
  least two exponent digits), and the integer helpers `__py_digit_count`,
  `__py_write_digits` and `__py_write_uint`. They are built from the
  neutral `f64_class`, `f64_signbit`, `f64_digits` and `f64_exp10` plus
  `add`/`sub`/`mul`/`div`, `cmp_*`, branches, `call` and `putchar`, so no
  backend needs Python code. Every float `print` is one
  `call __py_write_float` (its unused `0` always lands in the register
  `py_repr`, so a print costs `main` no extra register) followed by
  `putchar 10`. A module that prints no float (every `py-int-print` and
  `py2-int-print` module) is exactly `main`, byte-identical to 0.3.0.
  Python 2 and 3.0a1 (`py2`, `py30`) never print a float (their profile is
  `Floats::None`), so nothing changes for them.
- **The release runner's step budget is derived from the module it runs**
  (`len(main) + 1 + (calls of __py_write_float in main) × 1,024`, capped
  at `MAX_IIR_INSTRUCTIONS × (1 + 1,024 / 4)`, 257M; it was a flat
  `MAX_IIR_INSTRUCTIONS`, 1M). A float print now runs the helper, up to
  700 VM steps (`-1.7976931348623157e+308`) instead of one builtin call,
  so the old budget refused valid programs: the 2,048-line, 64 KiB
  `print(-1.2345678901234567e-308)` program the default frontend budgets
  accept needs 1,423,362 steps. `MAX_STEPS_PER_FLOAT_PRINT` (1,024) bounds
  one print, and the compiler's `main` is straight-line, so the per-module
  bound is sound for its output. A hand-built looping module gets only
  its own size's worth (`label L; jmp L` stops after 3 steps); a flat
  257M ceiling would have let it spin for about 26 s in a release build
  (security review).
- `run_module` registers only `print_i64` and `putchar`.
- `code/grammars/python/release_classes.csv` is regenerated: the 77
  `py-float-print` rows have new digests. The classes partition the
  releases as before ({3.6.0, 3.12.3, 3.13.16, 3.11.17, 3.16.0a1} and
  {3.5.10}); no `py-int-print` or `py2-int-print` row changed.

### Added

- `write_float_matches_the_pr6a_python_repr_byte_for_byte`: 1,248 values
  (every layout boundary, the specials and a xorshift sweep of bit
  patterns, each with both signs) printed by the module's helper on
  vm-core, byte for byte against vm-core's PR6a `python_repr`, and (when a
  host `python3` is installed) against CPython running the same programs.
- `one_float_print_stays_within_its_step_bound` measures the dearest print
  over that sweep (700 steps) and pins it between half the bound and the
  bound.
- `the_step_budget_covers_the_largest_float_printing_program` runs the
  largest program the default budgets accept for the dearest print line,
  and checks it needs more than the old budget; it also runs a
  trusted-budget module at vm-core's 65,536-register frame limit.
- `the_float_helpers_are_emitted_only_into_modules_that_print_a_float`,
  and the `float_repr` unit tests (neutral ops only, one `ret` per helper,
  fully typed, no `mod`, only the entry helper reads the float).
- `release_subsets`'s C7 check walks every function and accepts the
  helpers' branches and `call`s between them; it rejects `write_f64_py`.

## Unreleased — LANG84 phase 7c, slice 3: `py-float-print`

### Added
- **Floats on the release path.** A new `ReleaseProfile` field, `floats`
  (`Floats::Repr` or `Floats::None`), and a new profile, `py30`, for 3.0a1,
  whose `str(float)` is `%.12g` (`PREC_STR`). `py3` (3.5.10 and later)
  gets `Floats::Repr`: `print` writes `repr`, `float_repr` being `tp_str`
  since 3.2. Under it the walker types every value `int` or `float` and
  lowers floats to LANG83 ops: a `FLOAT` token to an `f64` `const`, an
  `int` operand to `int_to_real`, `+ - * /` to the PR6c `f64_add`/
  `f64_sub`/`f64_mul`/`f64_div`, unary minus to `f64_neg`, and
  `print(float)` to `write_f64_py` plus `putchar 10`. `/` is true division.
  A divisor known to be `±0` emits `trap "Trap.DivZero"` before the
  division, as CPython raises ZeroDivisionError.
- **Refusals** (`Unsupported`): float `//` and `%`, `int / int` with an
  operand above 2^53, a literal that overflows to `inf`, and (under `py30`
  and `py2`) any float or true `/`. `decode_float_text` re-checks a
  caller-supplied `FLOAT` token against the Python 3 float grammar, so
  only digits, `.`, an exponent and its sign reach Rust's parser.
- `ReleaseProfile::subsets` (a list, narrowest first) replaces `subset`;
  `py3` claims `py-int-print` and `py-float-print`. `subset()` names the
  widest. `PROFILE_TOKENS` (`py3` adds `FLOAT`) joins `PROFILE_RULES`.
- `run_module` captures `write_f64_py` (the VM's own `python_repr`) in the
  same ordered output buffer as `print_i64` and `putchar`.
- **Corpus.** `corpus/py-float-print/` has 24 cases (13 positive, five of
  which trap; 11 refusals), each checked against host CPython 3.11.17,
  3.12.3 and 3.13.16; `float_underscores` has `.reject@3.5.10`.
  `py-int-print`'s `reject_float` and `reject_true_division` left that
  corpus, since six of its seven releases now compile them; 3.0a1's
  refusal is pinned by `release_3_0a1_refuses_floats_and_true_division`.
- **Tests.** Stage I and IIR identity for `py-float-print` on its six
  releases; the sweep's two float classes ({3.6.0, 3.12.3, 3.13.16,
  3.11.17, 3.16.0a1} and {3.5.10}); the oracle over both Python 3
  corpora; IIR-shape tests (`write_f64_py` and the newline, `int_to_real`,
  the IEEE ops, `f64_neg`, the trap before a zero divisor, the 2^53 rule,
  forged `FLOAT` tokens); `cst_contract` checks `FLOAT` exists in exactly
  the `py3` releases.

## Unreleased — LANG84 phase 7c, slice 2: Python 2 literals; 2.4, 2.6, 3.5.10, 3.6.0

### Added
- **Python 2 integer literals.** A new `ReleaseProfile` field,
  `int_literals` (`IntLiterals::Decimal` for `py3`, `IntLiterals::Python2`
  for `py2`), picks the decoder. The `py2` decoder reads every spelling the
  tags' `tok_get` lexes and `parsenumber` reads: decimal, octal `0[0-7]*`
  (`010` is 8), hex `0x`, PEP 3127 `0o`/`0b`, and an `l`/`L` long suffix,
  which changes no value (`str()` of a long has no `L`; a long that fits
  `i64` does the same arithmetic as an int). The lexer decides which
  prefixes a release has: `0o17` never reaches the decoder from a 2.2 or
  2.4 tree. A literal above `i64::MAX` is refused as `Unsupported`, which
  also avoids the one spelling two Python 2 releases read differently (a hex
  or octal literal between `LONG_MAX` and `ULONG_MAX` is negative at 2.2
  and a positive long from 2.4, PEP 237). The decoders are free functions
  (`decode_decimal`, `decode_python2`, `digits_value`), so a forged token is
  re-checked by the same code.
- **Four more profiled releases:** 2.4 and 2.6 (`py2`), 3.5.10 and 3.6.0
  (`py3`). `profile_for` lists them.
- **Corpus.** `py2-int-print` grows to 49 cases (24 positive):
  `octal_literals` and `long_suffix` (formerly `reject_octal` and
  `reject_long_suffix`), `hex_literals`, `pep3127_literals`
  (`.reject@2.2`, `.reject@2.4`) and `overflow_long_literal`, plus eight
  refusals (`08`, `0o8`, `0b2`, `0x`, `1LL`, and hex, octal and long
  literals above `i64`). `py-int-print`'s `int_underscores` gains
  `.reject@3.5.10`.
- **Tests.** `python_2_integer_literals_follow_each_release` (prefixes per
  release, refusals, and eight spellings of 8 that lower to identical IIR);
  `the_translation_keeps_every_literal_value`; py2 forged-token checks in
  `forged_trees_are_refused`. The translate-to-Python-3 oracle now rewrites
  `0NNN` to `0oNNN` and drops the `L` suffix; it agrees on all 24 positive
  cases.

### Changed
- **Release sweep.** `release_classes.csv` now has four classes. Python 3:
  {3.0a1, 3.5.10} (no PEP 515 `_`), represented by 3.0a1, and {3.6.0,
  3.12.3, 3.13.16, 3.11.17, 3.16.0a1}, now represented by 3.6.0 (it
  replaces 3.12.3 as the earliest member). Python 2: {2.2, 2.4} and {2.6,
  2.7.18}, split by `pep3127_literals`; represented by 2.2 and 2.6.
- The release lists and counts in `release_subsets`, `release_sweep` and
  `cst_contract` (11 profiled releases).
- The decimal decoder's leading-zero message now says "a SyntaxError in
  Python 3 (Python 2 read them as octal)", since one subset now has octal.

## Unreleased — LANG84 phase 7c, slice 1: Python 2 (`py2-int-print`)

### Added
- **The `py2` profile** (`profiles::PY2`, `Slash::Classic`) for releases 2.2
  and 2.7.18, and a `subset` field on `ReleaseProfile` (`py-int-print` for
  `py3`, `py2-int-print` for `py2`). The profile's doc comment cites the
  tags' `Objects/intobject.c` (classic `/` floors through `i_divmod`; a zero
  divisor raises `ZeroDivisionError`; a result past the C `long` is promoted
  to `long`), `Python/ceval.c` (`PRINT_ITEM`/`PRINT_NEWLINE`) and
  `Python/pythonrun.c` (`Py_FlushLine`). Blob IDs are in
  `pair-evidence.csv`.
- **Python 2's `print` statement.** `int_lower` dispatches on the shape of
  the tree: a `small_stmt` whose child is a `print_stmt` node is the Python 2
  statement, never a release ID. `print <expr>` lowers to `print_i64` and a
  bare `print` to `putchar 10`. `print a, b` and `print a,` are refused
  (`Unsupported`): CPython 2 flushes a pending softspace as `\n` before an
  uncaught exception, so `print 1, 1/0` writes `1\n`, and only a trap
  handler could reproduce that.
- **Classic `/`** lowers exactly like `//`, including the `-1` divisor fold.
  Before this it was refused as "no release pair yet".
- `SUBSETS` (both subsets) and `profiles::PROFILE_RULES` (rules a profile's
  grammars add). `cst_contract` checks that `print_stmt` exists in exactly
  the `py2` releases.
- **Tests.** The corpus, identity-class and sweep tests run once per subset;
  `tests/common` takes the subset by name.
  - `py2_int_print_corpus_holds_on_every_claiming_release`: 38 cases × 2
    releases.
  - `python_2_and_python_3_disagree_where_the_languages_do`.
  - `py2_corpus_agrees_with_host_python3_after_translation`: there is no
    host `python2`, so each accepted case is translated (`print X` →
    `print(X)`, `/` → `//`) and run on host `python3`. The translation is
    exact on ints, because both languages floor and neither overflows.
  - `release_classes.csv` gains the `py2-int-print` rows: one class,
    {2.2, 2.7.18}, represented by 2.2.

### Security

- `compile_cst` refuses a tree whose `print` shape belongs to the other
  profile's grammar. A `print_stmt` lowers only under `py2`, and a call-form
  `print(…)` only under `py3`. Before this, a Python 3 tree compiled under a
  Python 2 release lowered `print()` as a newline, but Python 2 prints `()`.
  Source paths always parse with the release's own grammar, so only direct
  `compile_cst` callers could reach this. The security review found it, and
  the test is `a_tree_from_the_other_profiles_grammar_is_refused`.

### Changed
- Error messages name the release's subset (`py-int-print` or
  `py2-int-print`) instead of always `py-int-print`. A release with no
  profile is refused with `no IIR profile for any of py-int-print,
  py2-int-print`. The leading-zero message covers both Python 2 (octal) and
  Python 3 (SyntaxError).

## Unreleased — grammar-tools adjacency predicates (E6)

### Changed
- `tests/cst_contract.rs`'s `referenced_tokens` covers the new
  `GrammarElement::AdjacencyPredicate` (`@no_space_before` …), which tests
  the trivia between tokens and names no token. The exhaustive `match`
  stopped compiling when grammar-tools gained the variant.

## Unreleased — LANG84 phase 7b, slice 2: the release sweep

### Added
- `tests/release_sweep.rs`: compiles every positive `py-int-print` case
  under every release whose `iir_status` claims it (3.0a1, 3.12.3, 3.13.16,
  3.11.17, 3.16.0a1, inventory order), digests each module's
  `interpreter_ir::serialise` bytes (SHA-256; the `release` field is not on
  the wire), classes each release by its whole outcome vector, and compares
  the result with the committed `code/grammars/python/release_classes.csv`
  (`subset,case,release,class`, 90 rows). A cell is `rejected` only where a
  `.reject@<release>` override says so and the compile error has its
  category (`int_underscores` on 3.0a1); any other refusal fails. Two
  classes: 3.0a1 alone (`e6a72b170747738e`), and 3.12.3, 3.13.16, 3.11.17,
  3.16.0a1 (`5afb074f1e9583ba`, representative 3.12.3). Every release that
  accepts a case lowers it to the same IIR. `PYTHON_RELEASE_SWEEP_WRITE=1`
  rewrites the CSV; that run fails on purpose and refuses
  `CI`/`GITHUB_ACTIONS`. Tests:
  `the_sweep_covers_every_claiming_release_in_inventory_order`,
  `release_classes_csv_matches_the_sweep`,
  `every_accepting_release_lowers_a_case_to_the_same_iir`,
  `the_boundary_releases_are_the_class_representatives`.
- Dev-dependency `coding_adventures_sha256` for the class digests.

### Changed
- The inventory and corpus readers moved from `tests/release_subsets.rs` to
  `tests/common/mod.rs`, shared with the sweep. They now fail on an override
  other than `.reject@<release>` (an `.expect@<x>` would otherwise be
  ignored).
- `the_default_release_claims_py_int_print_on_the_backends` no longer
  requires the default to be the only `backend_status` claim. It requires
  the default to claim B, and the releases claiming B to be exactly the
  releases claiming I. `releases.csv` now claims B for all five releases,
  because `lang-aot`'s matrix runs the corpus on each class representative.

## Unreleased — LANG84 phase 7b, slice 1: release recorded and selectable through lang-aot

### Added
- The compiled module records the release that produced it:
  `IIRModule::release` is the resolved ID from `compile_source_at` and
  `compile_cst` alike (`3.13.16` for `None`). lang-aot's `--lang
  python@<release>` relies on it. Test:
  `the_module_records_the_release_that_produced_it`.

### Changed
- `FrontendError::ReleaseNotAvailable.stage` is self-describing. A row with
  no grammar pair now reads `no grammar pair: grammar_stage none` instead of
  the bare `none`; python-parser's regeneration-bug phrases (`token table
  missing (regenerate python-lexer)`) are kept as they are. Test:
  `release_not_available_reasons_are_self_describing`.
- `claiming_releases_form_one_iir_identity_class` checks each module's
  `release`, then sets that (intentionally differing) field aside before
  comparing the releases' IIR.

## 0.3.0 — 2026-10-20

### Added — LANG84 phase 7a: `py-int-print` on the release path
- `compile_source_at(source, release, module_name, &FrontendBudgets)`
  resolves an exact `code/grammars/python/releases.csv` ID (`None` =
  `DEFAULT_RELEASE`, `3.13.16`). It looks up the release's `ReleaseProfile`
  (`profiles.rs`, profile `py3`), parses with that release's pair through
  `python-parser::parse_python_at`, and walks the CST in place (`int_lower.rs`).
  The walk uses borrowed nodes and no owned AST; the flat `arith`/`term`
  chains and runs of unary signs are folded in loops. `compile_cst` takes a
  caller-supplied tree after measuring it (items, depth, text).
- LANG83 ops only, plus `const`/`ret`:
  - `+ - *` and unary `-` → `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`,
    `i64_neg_ovf`. A result outside `i64` traps `Trap.Overflow` where
    CPython would make a bigger `int` (stated divergence; bignum promotion
    is later).
  - `//` and `%` → `i64_div_floor` and `i64_mod_floor`. A zero divisor
    traps `Trap.DivZero`.
  - `print(x)` → `print_i64`, `print()` → `putchar 10`.
  - `main` returns `i64` 0, not `ret_void`, the void-entry lesson from the
    CLR and BEAM.
- A divisor known to be `-1` is folded: `a // -1` → `i64_neg_ovf a`,
  `a % -1` → `const 0`. So `i64::MIN // -1` and `i64::MIN % -1` never reach
  a backend's divide.
- Refusals, as typed `Unsupported` errors naming the rule and position:
  - `/` (true division, `Slash::True` in the profile);
  - names, another callee, more than one argument;
  - a call used as a value, or a bare expression statement;
  - a literal outside `i64`.
- `FrontendError`: the LANG84 C6 categories.
- `run_source_at` / `run_module`: run on vm-core, keeping the output before
  a trap and its kind.
- Tests: `release_subsets` (stage I, IIR identity, CSV/profile agreement,
  host CPython oracle, folding Windows' `\r\n` stdout to `\n`),
  `release_lowering`, `cst_contract` (C8),
  `hostile_release` (C5, child process), `iir_path` (C2/C4.5).

### Unchanged
- LANG79's float pilot: `compile_source`, `compile_ast`, `run_source` and
  `pyvm` keep their API, behaviour and the legacy `"3.12"` minor-line
  grammar. LANG84 wants `compile_source` to delegate to
  `compile_source_at`. That waits until the float subset moves to release
  pairs (7b/7c, after LANG83 float formatting).

## 0.2.0 — 2026-10-19

### Module-level float bindings

- Lower a single-target `NAME = <float expression>` statement and rebinding.
  The value is evaluated into temporaries first and then bound with one IIR
  `mov` into the mangled register `py:NAME`, so `x = x + 1.0` reads the old
  value and user names cannot collide with `pyN` temporaries or builtins.
- Accept NAME atoms in standalone expressions, binding values and every
  zero-to-three `print` argument position.
- A read with no earlier binding (statically known in this branch-free
  subset) lowers to a raising `py_name_error` builtin call at that point, so
  `run_source` reports `NameError: name 'x' is not defined` with earlier
  print output retained and later statements not executed, matching CPython.
- Reject with specific compile errors: binding `print` or `__debug__`,
  chained, tuple/list/parenthesized/starred, annotated, augmented, attribute,
  subscript and keyword targets, tuple values, binding a `print()` result,
  integer values, `print` used as a value, and unbound reads of Python 3.12
  builtin or dunder names (which CPython resolves rather than raising).
- Direct-AST callers must supply the lexer's canonical EQUALS token and
  canonical NAME tokens with an ASCII, non-keyword identifier spelling;
  forged kinds, overrides, spellings and node substitutions fail.

### Bounded Python print calls and token validation (previously unreleased)

- Validate direct-AST float literal enum kind and the complete versioned ASCII
  spelling before underscore removal/f64 conversion. Reject malformed digit
  groups, integer-only spellings, signs, special names and trailing bytes
  without changing valid exponent overflow or runtime output behavior.

- Validate canonical unary/binary arithmetic operator tokens and grouping
  delimiters in caller-supplied ASTs. Reject forged kinds, effective types,
  spellings and token/node substitutions while preserving source behavior.

- Lower an exact zero-argument `print()` grammar call to a Rust `vm-core`
  builtin that writes one newline and retains it if a later VM instruction
  fails. Other call forms remain bounded by the existing pilot.
- Lower exactly two positional float expressions from the Python grammar tree
  to IIR and print them through a Rust VM builtin with one separating space.
  Preserve source-order evaluation, output limits, and earlier completed
  output if a later argument or statement fails.
- Lower exactly three positional float expressions with two separating spaces
  through a single bounded Rust VM builtin call. Validate both comma token
  kinds for direct AST callers and preserve earlier output on later failure.

## 0.1.0 — 2026-10-07

- Add Python 3.12 float expressions and one-argument `print` lowering to IIR.
- Execute the emitted module on Rust `vm-core` with Python-specific division
  and float display hooks.
- Bound text in caller-supplied ASTs and preserve earlier stdout when a later
  VM instruction fails.
