//! CLOC14 end-to-end byte-identity test harness.
//!
//! ## Why this exists
//!
//! Until this harness, every CLOC12 gap-fix PR was theoretical —
//! we'd patch a fold rule, the unit tests would go green, but we
//! had no measurement of whether the output of `closurec` actually
//! matches what Google's Closure Compiler emits. The termination
//! condition of this project ("drop-in binary-compatible
//! closurec") is a behavioural property, not a feature checklist;
//! without an end-to-end test that *measures* byte-divergence, we
//! can ship correct-in-isolation passes that compose into a
//! diverging compiler.
//!
//! ## How this harness differs from the existing `diff_*.rs` files
//!
//! The legacy `diff_<flag>.rs` files each test ONE CLI flag's
//! shape (`--charset`, `--output_wrapper`, etc.). They're
//! flag-shaped: each fixture exercises a flag, not the
//! optimization pipeline.
//!
//! CLOC14's fixtures live under `tests/diff/minify_<name>/` and
//! exercise the optimization pipeline end-to-end: an input JS
//! file goes through `closurec`'s actual `--compilation_level`
//! pipeline, and the stdout is compared against the output that
//! Google Closure Compiler produces on the same input with the
//! same flags. A failing minify fixture is a *real* divergence
//! that ships incorrect behaviour to users.
//!
//! ## Discovery + single-runner design
//!
//! Rather than one `diff_minify_<name>.rs` per fixture (which
//! creates linear boilerplate growth), this single runner walks
//! `tests/diff/minify_*/` at test time and executes every
//! fixture. Failures are collected per-fixture and reported
//! together — `cargo test diff_minify` lists every divergent
//! fixture, not just the first failure.
//!
//! ## Fixture format
//!
//! ```text
//! tests/diff/minify_<name>/
//! ├── flags.txt          # one CLI flag per line
//! ├── input/             # input files referenced by flags.txt
//! │   └── a.js
//! ├── expected.stdout    # the expected stdout, captured from
//! │                      # Google Closure Compiler on the same
//! │                      # input + flags
//! └── README.md          # (optional) what this fixture pins
//!                        # and where the golden was captured
//! ```
//!
//! ## Status of each fixture
//!
//! Each fixture's `README.md` should document:
//!   1. The Google Closure Compiler version that produced the
//!      golden (e.g. `v20240317`).
//!   2. The exact command line used to capture the golden.
//!   3. Any caveats — e.g. "expected to fail until gap-014
//!      lands" (mark such tests with `#[ignore]` in the
//!      `IGNORE_FIXTURES` list below).
//!
//! ## Authoring a new fixture
//!
//! 1. Pick the smallest input that exercises the behaviour.
//! 2. Run upstream Closure to capture the golden:
//!    ```
//!    java -jar closure-compiler.jar \
//!        --compilation_level WHITESPACE_ONLY \
//!        --js input/a.js > expected.stdout
//!    ```
//! 3. Add the fixture directory. The next `cargo test
//!    diff_minify_walk` run picks it up automatically.

use std::path::Path;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Fixtures intentionally left failing — usually because they pin
/// a behaviour we KNOW we don't yet match (e.g. a CLOC12 gap that
/// hasn't shipped). Listing the fixture here documents the gap
/// while keeping CI green; the fixture is still useful as a
/// future-target.
///
/// Format: fixture name (without the `minify_` prefix) → reason.
const IGNORE_FIXTURES: &[(&str, &str)] = &[
    // gap-099 RESOLVED in CLOC12.102 — paren elision around a
    // COMPUTED-MEMBER object: `(b)[c]` -> `b[c]`, `(b.c)[d]` -> `b.c[d]`.
    // The `[index]` sibling of gap-065/gap-057; only a safe simple
    // reference is unwrapped (`(a+b)[c]`, `(b||c)[d]`, `f(b)[c]` keep
    // their parens).
    // gap-100 RESOLVED in CLOC12.103 — paren elision around a
    // function/class EXPRESSION in expression position:
    // `a=(function(){})()` -> `a=function(){}()`, `a=(class{})()` ->
    // `a=class{}()`. Minimal safe slice — fires on a statement-level
    // assignment `IDENT=(function/class…)` or after `,`; the
    // statement-position IIFE `(function(){})();` and default-param
    // position keep their parens.
    // gap-097 RESOLVED in CLOC12.101 — an async generator method
    // (`async*m(){}`) now gets the separating space between `async` and
    // `*` that upstream emits (`async *m(){}`), in both class and object
    // bodies. A `needs_separator`-style helper recognises the full
    // method signature `async * NAME ( … ) {` so the arithmetic form
    // `a=async*b` (and `async*f()`) is left untouched.
    // gap-098 RESOLVED in CLOC12.100 — a trailing bare decimal point on
    // an integer (`5.` -> `5`, `5.+1` -> `5+1`) is now dropped by the
    // gap-093 pre-pass's complementary branch: when a NUMBER is followed
    // by a DOT whose follower is NOT a property name (the dot can't be a
    // member access), the redundant decimal-point dot is removed.
    // gap-093 RESOLVED in CLOC12.98 — an integer/float NUMBER literal
    // that is the object of a `.member` access is now paren-wrapped:
    // `1..toString()` -> `(1).toString()`, `1 .x` -> `(1).x`,
    // `1.5.toString()` -> `(1.5).toString()`. A pre-pass in
    // `whitespace_only.rs` wraps the number so the `.` reads as member
    // access, not the number's decimal point. (The `1 .x` -> `1.x`
    // case was a CORRECTNESS bug — `1.x` does not parse.)
    // gap-094 RESOLVED in CLOC12.97 — the gap-046 array trailing-comma
    // drop is now guarded so it only fires when the comma follows a
    // REAL element (not a hole: a preceding `,` or `[`). `[1,,]`
    // (length 2) is preserved; `[1,2,]` -> `[1,2]` still drops.
    // `minify_array_hole_trail` enforced.
    // gap-095 (CLOC14.41): a chained `new new A` is wrapped by upstream
    // to `new (new A)` (disambiguates the inner NewExpression as the
    // callee). closurec leaves `new new A` (valid, but not byte-id).
    ("chained_new", "gap-095: chained new -> new (new A)"),
    // gap-096 RESOLVED in CLOC12.99 — the es2024/es2025 REGEX token's
    // flag character class was `[dgimsvy]`, accidentally omitting the
    // ES2015 `u` (unicode) flag (a typo when `v` was added for ES2024).
    // So `/x/gimsuy` stopped at `u`, lexing as `/x/gims` + a stray `uy`
    // identifier emitted as the corrupt `/x/gims uy`. Fixed in the
    // source grammars (`[dgimsuvy]`) + regenerated lexer pattern.
    // gap-090 (CLOC14.40) — CORRECTNESS: a string with a backslash
    // escape that closurec does NOT explicitly handle (`\u{…}` code-
    // point, `\xNN` hex, `\0` null, legacy octal) has its backslash
    // DROPPED, mangling the string value (`"\x41"` -> `"x41"` instead
    // of `"A"`; `"\u{1F600}"` -> `"u{1F600}"`). Upstream decodes and
    // re-escapes (`\x41` -> `A`, `\u{1F600}` -> `😀`,
    // `\0` -> `\x00`). Lexer/emitter string-escape handling. HIGH
    // PRIORITY — corrupts output, not just byte-identity.
    ("str_codepoint_esc", "gap-090: \\u{...} code-point escape mangled"),
    ("str_unicode4_esc",  "gap-090: \\uNNNN 4-hex unicode escape mangled"),
    ("str_hex_esc",       "gap-090: \\xNN hex escape mangled"),
    ("str_hex27_esc",     "gap-090: \\x27 hex escape mangled (-> apostrophe)"),
    ("str_null_esc",      "gap-090: \\0 null escape mangled"),
    // gap-091 RESOLVED in CLOC12.96 — a BigInt RADIX literal is now
    // canonicalised to decimal (`0xFFn` -> `255n`, `0o17n` -> `15n`,
    // `0b101n` -> `5n`) by parsing the radix body to u128 in the BigInt
    // branch of `normalize_number_value`. Over-u128 magnitudes stay
    // verbatim (residual). `minify_bigint_hex` / `minify_bigint_bin`
    // enforced.
    // gap-092 (CLOC14.40): a `/` DIVISION operator in `a/b/c` is mis-
    // lexed as a REGEX literal `/b/`, producing spurious separating
    // spaces (`a /b/ c`). Still valid JS (same grouping) but not byte-
    // identical. Regex-vs-division disambiguation is a lexer-level
    // gap (needs parser context to know `/` after a value is division).
    ("regex_div", "gap-092: division mis-lexed as regex (spacing)"),
    // gap-044: JavaScript lexer does not yet support
    // template literal SUBSTITUTIONS (`${expr}` inside
    // a backtick-delimited string). Lexer-level gap.
    ("template_subst", "gap-044: lexer does not support `${...}`"),
    ("tagged_subst",   "gap-044: lexer does not support `${...}` (tagged variant)"),
    // gap-072 (CLOC14.35): `await` OPERAND paren elision — strip
    // redundant parens around a simple-reference operand
    // (`await(x)` → `await x`). `await` binds at UNARY precedence,
    // exactly like `typeof`/`void`/`delete` (gap-101's
    // `is_safe_unary_kw_operand`), so it is now tractable by adding
    // `await` to that keyword block (NOT the gap-056 return/throw
    // block — `await` binds TIGHTER than binary operators, so a binary
    // operand keeps its parens). CLOC14.46 detail: a kept binary
    // operand is emitted WITH a separating space — `await(a+b)` →
    // `await (a+b)` — so the fix must add that space, not just keep the
    // parens. `minify_await_binary_kept` pins that case.
    ("await_paren_elide",  "gap-072: await operand paren elision"),
    ("await_binary_kept",  "gap-072: await binary operand keeps parens with a space"),
    // gap-102 RESOLVED in CLOC12.105 — a `yield` operand's grouping
    // parens (`yield(a)` → `yield a`, `yield(a+b)` → `yield a+b`,
    // `a=yield(b)` → `a=yield b`) now elide via the gap-055/056
    // prefix-classification block, which gained a `yield` anchor
    // (`is_yield_prefix`). `yield` takes an AssignmentExpression, so
    // the parens never carry meaning except around a comma operand
    // (`yield(a,b)` stays wrapped via the shared top-level-comma
    // guard); the property guard keeps `o.yield(x)` a method call, and
    // the `yield*` delegate is excluded for free. `minify_yield_paren_*`
    // enforced; `minify_yield_comma_kept` / `minify_yield_star_pass`
    // guard the keep cases.
    // gap-101 RESOLVED in CLOC12.104 — a prefix unary operator
    // (`typeof`/`void`/`delete`/`!`/`-`/…) with a PARENTHESISED
    // higher-arity operand (a unary-expression or a call) now drops the
    // grouping parens: `typeof(void 0)` → `typeof void 0`,
    // `typeof(-b)` → `typeof-b`, `typeof(b())` → `typeof b()`. The
    // gap-054 keyword block's operand predicate was widened from
    // `is_safe_unary_operand` to `is_safe_unary_kw_operand`, which also
    // accepts a leading symbol/keyword unary chain and a call/member
    // chain. A parenthesised BINARY operand (`typeof(b+c)`) is still
    // rejected. `minify_unary_typeof_void` / `minify_unary_neg_operand`
    // / `minify_unary_call_operand` enforced.
    // gap-081 RESOLVED in CLOC12.89 — a ternary CONDITION grouping
    // paren (`(a)?b:c` → `a?b:c`) now elides via the gap-077
    // left-operand pre-pass (a structural `?` joined its after-set).
    // `minify_ternary_cond_paren` enforced.
    // gap-082 RESOLVED in CLOC12.91 — a decimal float / scientific
    // literal that denotes an INTEGER value fitting in u128 (`1e3` =
    // 1000 → `1E3`, `1.5e10` → `15E9`, `1.0` → `1`, `100.00` → `100`)
    // is now routed through the shortest-form integer logic by
    // `decimal_float_as_u128`. `minify_num_exp_case` enforced.
    // Residual (still deferred): the V8 fractional shortest-form
    // (`0.5` → `.5`, `1e-5` → `1E-5`, `0.0001` → `1E-4`) and
    // out-of-u128 magnitudes (`1e100` → `1E100`) need a Grisu/Ryu
    // float formatter — tracked as gap-085. `minify_num_neg_exp_frac`
    // (`5e-3` → `.005`) is the negative-exponent fractional case of the
    // same deferred gap.
    ("num_neg_exp_frac", "gap-085: negative-exp scientific -> fractional shortest-form"),
    ("num_small_frac",   "gap-085: small decimal fraction -> exponential (0.0001 -> 1E-4)"),
    // gap-083 (CLOC14.38): PRECEDENCE-aware operand paren elision —
    // `a==(b+c)` → `a==b+c` (the inner op binds tighter than the
    // outer). Extends gap-077/078 beyond the atomic-operand guard;
    // needs an operator-precedence table.
    ("precedence_operand", "gap-083: precedence-aware operand paren elision"),
    // gap-086 RESOLVED in CLOC12.93 — redundant parens around a whole
    // CALL ARGUMENT (`f((a))` → `f(a)`, `f((a+b))` → `f(a+b)`,
    // `f((a),(b))` → `f(a,b)`) now elide via a call-open-anchored
    // arg-list walk. The one load-bearing case `f((a,b))` (a single
    // comma-operator argument) is preserved by a top-level-comma guard.
    // `minify_call_arg_paren` enforced; `minify_call_arg_comma_keep`
    // guards the comma exception.
    // gap-087 RESOLVED in CLOC12.92 — a paren wrapping the WHOLE index
    // of a computed-member subscript (`a[(b)]` → `a[b]`, `a[(b+c)]` →
    // `a[b+c]`, `a[(b,c)]` → `a[b,c]`, `a[b[(c)]]` → `a[b[c]]`) now
    // elides via a subscript-anchored pre-pass. No comma guard is
    // needed (the brackets delimit a single-expression context); array-
    // literal element parens (`[(a,b)]` must keep) are the gap-086
    // comma-guarded family. `minify_index_paren` enforced.
    // gap-088 RESOLVED in CLOC12.94 — EMPTY-STATEMENT (`;`) elimination
    // (`;;var x=1;;;` → `var x=1;`, `;;;` → ``, `function f(){;;x();}` →
    // `function f(){x()}`) now drops a `;` whose predecessor is
    // `{`/`;`/start-of-input, except inside a `for(...)` header.
    // `minify_empty_stmt` enforced.
    // gap-089 RESOLVED in CLOC12.95 — empty `new` call-paren drop for a
    // MEMBER-expression callee (`new a.b()` → `new a.b`, `new a.b.c()`
    // → `new a.b.c`, `new a[x]()` → `new a[x]`) via a forward pre-pass
    // that extends gap-050 beyond bare-identifier callees. Blocked
    // followers (`.`/`[`/`(`/backtick) are left to the new-expr
    // member-wrap pass. `minify_new_member_empty` enforced.
    // gap-084 RESOLVED in CLOC12.90 — a nested double-paren around a
    // var-init RHS (`((a))` → `a`, `(((a)))` → `a`, `((a+b))` →
    // `a+b`) now fully strips: the gap-053 var-init elision runs to a
    // FIXPOINT, peeling every redundant layer while the
    // top-level-comma guard still halts at `((a,b))` → `(a,b)`.
    // `minify_double_paren_varinit` enforced.
    // gap-077 RESOLVED in CLOC12.88 — a binary operator's
    // parenthesised ATOMIC LEFT operand (`(a)+b` → `a+b`, `(a)*b` →
    // `a*b`) now elides via a new left-operand pre-pass — the mirror
    // of gap-075/078. Guards: the `(` must START an expression (not a
    // call paren), the `)` must be followed by a binary operator, and
    // the span must pass the atomic-operand guard (so `(a+b)*c` keeps
    // its parens). `minify_left_operand_paren` enforced.
    // gap-078 RESOLVED in CLOC12.87 — the right-operand paren-elision
    // pre-pass (gap-075) now also anchors on the binary comparison /
    // logical / arithmetic / bitwise symbol operators (`a==(b)` →
    // `a==b`, `a||(b)` → `a||b`, …), keeping the conservative atomic-
    // operand guard. `minify_eq_operand_paren` enforced.
    // gap-080 RESOLVED in CLOC12.86 — an `else`-body single-statement
    // block (`if(x)a();else{b()}` → `if(x)a();else b();`) now flattens
    // via a parallel `else`-anchored pre-pass (the `else` keyword
    // followed directly by `{`, no `(…)` header). Reuses the
    // gap-074/079 single-statement / synthetic-`;` machinery.
    // `minify_else_body_flatten` enforced.
    // gap-079 RESOLVED in CLOC12.85 — an `if`-body single-statement
    // block (`if(x){y()}` → `if(x)y();`) now flattens via the gap-074
    // pre-pass (`if` added to the anchor keyword set). The
    // dangling-else hazard is covered for free by the existing
    // no-control-flow-keyword guard. `minify_if_body_flatten`
    // enforced.
    // gap-075 RESOLVED in CLOC12.84 — a prefix-unary SYMBOL operator
    // operand's grouping parens (`-(a)` → `-a`, `!(a)` → `!a`,
    // `~(a)` → `~a`, `-(-a)` → `- -a`) now elide;
    // `minify_unary_minus_paren` enforced.
    // gap-076 RESOLVED in CLOC12.83 — a `with`-body single-statement
    // block (`with(o){a()}` → `with(o)a();`) now flattens via the
    // gap-074 pre-pass (`with` added to the anchor keyword set);
    // `minify_with_body_flatten` enforced.
    // gap-074 RESOLVED in CLOC12.81 — a loop body that is a
    // single-statement block (`for(;;){continue l}` →
    // `for(;;)continue l;`) now flattens; `minify_loop_body_flatten`
    // enforced.
    // gap-067 RESOLVED in CLOC12.77 — a labeled single-statement
    // block (`label:{break label}` → `label:break label;`) now
    // flattens; `minify_label_block_flatten` enforced.
    // gap-068 RESOLVED in CLOC12.76 — redundant parens around a
    // `new` callee (`new(f)()` → `new f`, `new(a.b)` → `new a.b`)
    // are now stripped; both fixtures enforced.
    // gap-065 RESOLVED in CLOC12.74 — parens around a call /
    // tagged-template callee (`(f)(x)`→`f(x)`, `(a.b)(x)`→
    // `a.b(x)`) are now stripped; those three fixtures are
    // enforced again.
    // gap-066 RESOLVED in CLOC12.75 — redundant parens after
    // `extends` (`class A extends(B){}` → `class A extends B{}`)
    // are now stripped; `minify_class_extends_paren` enforced.
    // gap-064 RESOLVED in CLOC12.73 — `new A(")")` no longer
    // misreads the string `)` arg as the empty-paren close
    // (the gap-050 drop now gates on `is_structural_punct`).
    // `minify_new_str_paren_arg` + `minify_new_str_paren_member`
    // are enforced again.
    // gap-055/056/057 all RESOLVED (CLOC12.64/65/66) — ternary
    // arms, return/throw/=> prefixes, and member-object parens
    // (`(a).b` → `a.b`) now pass and are no longer ignored.
    // gap-053 was RESOLVED in CLOC12.62 — token-stream
    // pre-pass strips outer `(` `)` around `= ( ... ) ;`
    // when contents have no top-level `,` and don't start
    // with `function`. `minify_null_undef_compare` flipped
    // IGNORED → PASS.
    // gap-054 was RESOLVED in CLOC12.63 — token-stream
    // pre-pass strips parens around single-token operand
    // of `void`/`typeof`/`delete`. `minify_void_zero_call`
    // flipped IGNORED → PASS.
    // gap-051 was RESOLVED in CLOC12.60 — token-stream
    // pre-pass reorders `} ( ) )` to `} ) ( )`. IIFE
    // inner-call form `(fn(){...}())` normalizes to
    // outer-call form `(fn(){...})()`. `minify_fn_expr_iife`
    // flipped IGNORED → PASS.
    // gap-052 was RESOLVED in CLOC12.61 — `BlockKind::Other`
    // at EOF now wants `;`. `minify_labeled_block` and
    // `minify_double_break_continue` flipped IGNORED → PASS.
    // gap-050 was RESOLVED in CLOC12.57 — token-level
    // peephole drops the empty `()` after `new IDENT`
    // when the follower is safe. `minify_new_expr`
    // flipped IGNORED → PASS.
    // gap-048 was RESOLVED in CLOC12.55 — BigInt token
    // path now strips ES2021 `_` numeric separators.
    // `minify_bigint_separator` flipped IGNORED → PASS.
    // gap-049 was RESOLVED in CLOC12.56 — gap-032's
    // flatten now peeks the token after the closing `}`;
    // if it's another `}`, the trailing `;` is suppressed
    // from the inline emission. `minify_for_await_of`
    // flipped IGNORED → PASS.
    // gap-046 was RESOLVED in CLOC12.52 — `,` immediately
    // before `]` is now suppressed. `minify_trailing_array_comma`
    // flipped IGNORED → PASS. Object-literal trailing comma
    // (gap-046b) deferred.
    // gap-047 was RESOLVED in CLOC12.53 — `}` handler now
    // adds a 5th branch in its decision: when next non-trivia
    // is a statement-starting keyword, no synthetic `;` is
    // emitted (ASI covers the boundary). `minify_multi_line_func`
    // flipped IGNORED → PASS.
];

/// Walk `tests/diff/` and collect every directory whose name
/// starts with `minify_`. The discovery is at test time so adding
/// a new fixture only requires creating the directory — no source
/// file changes.
fn discover_fixtures() -> Vec<String> {
    let diff_root = Path::new("tests/diff");
    let mut fixtures: Vec<String> = std::fs::read_dir(diff_root)
        .expect("read tests/diff/")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|ty| ty.is_dir())
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("minify_"))
        .collect();
    fixtures.sort();
    fixtures
}

/// Run one fixture: load flags.txt, exec closurec, capture stdout.
/// Returns the closurec stdout on success, or an error string
/// describing what went wrong.
fn run_fixture(fixture: &str) -> Result<String, String> {
    let flags_path = format!("tests/diff/{fixture}/flags.txt");
    let raw = std::fs::read_to_string(&flags_path)
        .map_err(|e| format!("read {flags_path}: {e}"))?;
    let flags: Vec<String> = raw
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(|s| s.to_string())
        .collect();

    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .map_err(|e| format!("spawn closurec: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "closurec exited {:?}; stderr:\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("non-UTF-8 stdout: {e}"))
}

/// Comparison verdict for a fixture.
enum Verdict {
    Match,
    Diverge { actual: String, expected: String },
    Error(String),
    Skipped(String),
}

fn check_fixture(fixture: &str) -> Verdict {
    let bare = fixture.strip_prefix("minify_").unwrap_or(fixture);
    if let Some((_, reason)) = IGNORE_FIXTURES.iter().find(|(n, _)| *n == bare) {
        return Verdict::Skipped(reason.to_string());
    }

    let actual = match run_fixture(fixture) {
        Ok(s) => s,
        Err(e) => return Verdict::Error(e),
    };
    let expected_path = format!("tests/diff/{fixture}/expected.stdout");
    let expected = match std::fs::read_to_string(&expected_path) {
        Ok(s) => s,
        Err(e) => return Verdict::Error(format!("read {expected_path}: {e}")),
    };
    if actual == expected {
        Verdict::Match
    } else {
        Verdict::Diverge { actual, expected }
    }
}

/// Render a small diff banner showing the first divergent line.
/// Avoids dragging in the `similar` / `diff` crate just for tests
/// — line-by-line is enough for human-readable output.
fn first_diverging_line(actual: &str, expected: &str) -> String {
    let mut a = actual.lines();
    let mut e = expected.lines();
    let mut idx = 0usize;
    loop {
        idx += 1;
        match (a.next(), e.next()) {
            (Some(la), Some(le)) if la == le => continue,
            (Some(la), Some(le)) => {
                return format!(
                    "line {idx}:\n  actual:   {la:?}\n  expected: {le:?}"
                );
            }
            (Some(la), None) => {
                return format!("line {idx}: actual has extra:\n  {la:?}");
            }
            (None, Some(le)) => {
                return format!("line {idx}: expected has extra:\n  {le:?}");
            }
            (None, None) => return "(no line-level divergence — likely a trailing-byte difference)".to_string(),
        }
    }
}

/// The single test entry point. Walks all `minify_*` fixtures,
/// collects per-fixture verdicts, and asserts that every
/// non-ignored fixture matched.
///
/// **Why one test rather than one-per-fixture:** test discovery
/// happens at *runtime* not compile-time, so we can't generate a
/// `#[test]` per fixture without macros or build scripts. The
/// single-test design keeps the runner self-contained and reports
/// every failure in one shot.
#[test]
fn diff_minify_all_fixtures() {
    let fixtures = discover_fixtures();
    if fixtures.is_empty() {
        // Nothing under tests/diff/minify_*/ yet — that's not a
        // failure, but flag it so the next contributor sees the
        // empty state.
        eprintln!(
            "diff_minify: no fixtures discovered under tests/diff/minify_*/. \
             Add one via the format documented at the top of this file."
        );
        return;
    }

    let mut failures: Vec<(String, String)> = Vec::new();
    let mut matched = 0usize;
    let mut skipped: Vec<(String, String)> = Vec::new();

    for fixture in &fixtures {
        match check_fixture(fixture) {
            Verdict::Match => matched += 1,
            Verdict::Diverge { actual, expected } => {
                failures.push((
                    fixture.clone(),
                    first_diverging_line(&actual, &expected),
                ));
            }
            Verdict::Error(e) => {
                failures.push((fixture.clone(), format!("error: {e}")));
            }
            Verdict::Skipped(reason) => {
                skipped.push((fixture.clone(), reason));
            }
        }
    }

    eprintln!(
        "diff_minify: {} matched, {} failed, {} skipped (of {} total)",
        matched,
        failures.len(),
        skipped.len(),
        fixtures.len(),
    );
    for (f, r) in &skipped {
        eprintln!("  SKIP  {f}: {r}");
    }

    if !failures.is_empty() {
        let mut msg = String::from("diff_minify failures:\n");
        for (f, why) in &failures {
            msg.push_str(&format!("\n  ❌ {f}\n     {}\n", why.replace('\n', "\n     ")));
        }
        msg.push_str(&format!(
            "\n{} of {} non-ignored fixtures diverged from Google Closure golden.",
            failures.len(),
            fixtures.len() - skipped.len(),
        ));
        panic!("{msg}");
    }
}
