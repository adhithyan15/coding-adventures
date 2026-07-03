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
    // gap-095 RESOLVED in CLOC12.131 — `new new A` → `new (new A)`.
    // A pre-pass in `whitespace_only.rs` detects two consecutive operator
    // `new` tokens and wraps the inner NewExpression (callee + optional
    // dot-chain, WITHOUT the following arg-list) in `(…)`.
    // `minify_chained_new` now ENFORCED.
    // gap-096 RESOLVED in CLOC12.99 — the es2024/es2025 REGEX token's
    // flag character class was `[dgimsvy]`, accidentally omitting the
    // ES2015 `u` (unicode) flag (a typo when `v` was added for ES2024).
    // So `/x/gimsuy` stopped at `u`, lexing as `/x/gims` + a stray `uy`
    // identifier emitted as the corrupt `/x/gims uy`. Fixed in the
    // source grammars (`[dgimsuvy]`) + regenerated lexer pattern.
    // gap-090 RESOLVED — es2025.tokens now declares `escapes: none` so the
    // grammar lexer delivers raw string interiors to whitespace_only.rs.
    // `emit_quoted_string` fully decodes every ECMAScript escape form
    // (\xNN, \uNNNN, \u{N+}, \0, standard single-char) and re-emits in
    // Closure canonical form (`\x41` -> `A`, `\u{1F600}` -> `😀`,
    // `\0` -> `\x00`). `str_codepoint_esc` / `str_unicode4_esc` /
    // `str_hex_esc` / `str_hex27_esc` / `str_null_esc` are now ENFORCED.
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
    // gap-092 RESOLVED by F10 (declarative lexer mode transitions): the
    // es2025 grammar now declares a `div` mode entered after value-producing
    // tokens, so `/` after an identifier/number/`)`/`]` lexes as SLASH, not
    // REGEX. `regex_div` is now ENFORCED (un-ignored below).
    // gap-044 RESOLVED (first slice) — template literal substitutions
    // `${expr}` are now handled by a pair of flat lexer modes: `template`
    // (active right after TEMPLATE_HEAD/TEMPLATE_MIDDLE) and `template_div`
    // (active after a value-producing NAME inside `${...}`).  Both modes own
    // TEMPLATE_TAIL/TEMPLATE_MIDDLE patterns at higher priority than the
    // inherited RBRACE, so `}` after a simple expression closes the
    // substitution correctly.  `template_subst` and `tagged_subst` are now
    // ENFORCED.  Limitation: expressions with operators (`.`, `+`, `(`, …)
    // or nested `{ }` reset the mode to default/div, losing the template
    // context — full brace-depth support is a follow-up.
    // gap-072 RESOLVED in CLOC12.106 — `await` OPERAND paren elision
    // (`await(x)` → `await x`) plus the always-separating-space rule
    // (`await(a+b)` → `await (a+b)`). `await` binds at UNARY precedence,
    // so it was added to gap-101's `is_safe_unary_kw_operand` keyword
    // block; a parenthesised binary operand keeps its parens (await
    // binds tighter) but gains the space via `await_operator_needs_space`.
    // Contextual-keyword guards keep `function await(x){}` /
    // `{await(x){}}` (name) and `o.await(x)` (property) untouched. The
    // upstream compiler rejects non-async `await` as a parse error, so
    // identifier-`await` never appears in a byte-identity input.
    // `minify_await_paren_elide` / `minify_await_binary_kept` enforced.
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
    // gap-085 RESOLVED in CLOC12.129 — both remaining fractional-shortest-form
    // sub-cases now produce byte-identical output:
    //   `5e-3`   → `.005`  (`num_neg_exp_frac`: negative-exponent scientific)
    //   `0.0001` → `1E-4`  (`num_small_frac`:   small decimal → exponential)
    // Both fixtures now ENFORCED.
    // gap-107 RESOLVED in CLOC12.110 — a FRACTIONAL float literal
    // (non-integer value) with trailing zeros in its fractional part
    // now has them stripped to the shortest exact decimal, plus a lone
    // leading `0` before the `.` elided:
    //   `1.50`     -> `1.5`     (trailing zero)
    //   `123.4500` -> `123.45`  (multiple trailing zeros)
    //   `0.50`     -> `.5`      (trailing-zero strip + leading-`0` drop)
    // Fixed by a gap-107 arm in `normalize_number_value`'s fractional
    // fallback (reached only after `decimal_float_as_u128` returns
    // None, i.e. the value is genuinely non-integer): when the literal
    // has a `.` and NO exponent, strip trailing `0`s from the
    // fractional part (and a now-bare trailing `.`) then elide a lone
    // `0` integer part. Pure decimal-string normalisation — no
    // Grisu/Ryu. As a bonus the long-standing `0.5` -> `.5` now also
    // resolves. The genuinely Grisu-needing cases stay gap-085:
    // exponent forms (`5e-3`, `0.0001`'s `1E-4`) are excluded by the
    // no-`e`/`E` guard, and f64 precision loss
    // (`12345678901234567890` -> `1.2345678901234567E19`) never reaches
    // the arm (all-digits, no `.`). `minify_num_frac_trail_zero` /
    // `..._trail_zeros` / `..._lead_zero` now ENFORCED.
    // gap-083 RESOLVED in CLOC12.130 — precedence-aware operand paren
    // elision. When the inner operator's minimum precedence is STRICTLY
    // GREATER than the outer binary operator's precedence, the grouping
    // parens are redundant: `a==(b+c)` → `a==b+c` (`+` prec 12 > `==`
    // prec 9). Implemented via `binary_op_prec` + `min_toplevel_binary_prec`
    // helpers in `whitespace_only.rs`. `minify_precedence_operand` ENFORCED.
    // gap-108 RESOLVED in CLOC12.111 — a single-statement DO-body block
    // is flattened — `do{x()}while(a)` -> `do x();while(a)` — the same
    // single-statement-block-flatten family as gap-074 (loop body),
    // gap-079 (if body), gap-080 (else body), gap-076 (with body).
    // Fixed by a gap-108 token-re-stitcher block in `whitespace_only.rs`
    // mirroring the gap-080 else-flatten: anchor on a `do` keyword
    // (reserved -> `do{…}` is unambiguously the loop body), scan the
    // body to its `}`, and if it holds exactly one statement (no nested
    // `{`, no control-flow keyword at depth 1, zero top-level `;`), drop
    // the braces + swap `}` for a synthetic `;`. The trailing
    // `while(cond)` is untouched. A MULTI-statement do-body
    // (`do{x();y()}while(a)`) is correctly left braced.
    // `minify_do_body_flatten` now ENFORCED.
    // gap-109 RESOLVED in CLOC12.112 — a STRING-literal method KEY is
    // normalised to a COMPUTED key — `{"m"(){}}` -> `{["m"](){}}` — in
    // both class and object bodies. Fixed by a gap-109 pre-pass in
    // `whitespace_only.rs` that wraps the string in a synthetic `[`…`]`
    // pair when it is a method key: a string literal at a property-start
    // position (`{`/`,`/`}`/`static`), immediately followed by `(`, whose
    // matching `)` is followed by `{` (the method body — the decisive
    // guard distinguishing a method from a string CALL). Identifier keys
    // (`{m(){}}`), already-computed keys (`{["m"](){}}`), string property
    // VALUES (`{"a":1}`), and string calls (`f("m")`, `"m"(x)`) are all
    // unaffected. `minify_class_string_method` / `minify_obj_string_method`
    // now ENFORCED. (A string-keyed ACCESSOR `get"a"(){}` -> `get "a"(){}`
    // is a SEPARATE space-insertion gap, not this computed-wrap.)
    // gap-110 (CLOC14.53): a string method KEY preceded by a method
    // MODIFIER (`*` generator or `async`) is ALSO normalised to a
    // COMPUTED key, just like the plain case (gap-109) — but gap-109's
    // pre-pass only fired when the string's predecessor was a property
    // boundary (`{`/`,`/`}`/`static`), so a `*`/`async`-prefixed key was
    // missed:
    //   {*"m"(){}}        -> {*["m"](){}}
    //   class A{async"m"(){}} -> class A{async["m"](){}}
    //   {async*"m"(){}}   -> {async*["m"](){}}
    // The fix extends the gap-109 property-start set to include `*` and
    // `async` (with the same method-body guard). `static"m"` already
    // works (gap-109 covered `static`).
    // gap-110 RESOLVED in CLOC12.113 — the three `*_string_method`
    // fixtures below are now ENFORCED (un-ignored).
    // gap-111 (CLOC14.53): a reserved KEYWORD immediately before a
    // STRING LITERAL that the keyword grammatically takes needs a
    // separating SPACE that closurec omits:
    //   switch(x){case"a":…}      -> case "a":          (case clause)
    //   {get"a"(){}} / {set"a"…}  -> get "a" / set "a"  (accessor key)
    //   new"s"                    -> new "s"            (new callee)
    // NOT all keyword+string pairs need it — `typeof"s"`, `void"s"`,
    // `throw"e"`, `a in"s"` are already byte-identical (no space). The
    // fix is a `needs_separator`-style rule keyed on the specific
    // keyword set (`case`/`get`/`set`/`new`) immediately followed by a
    // string literal. (`get`/`set` here is the string-keyed-accessor
    // case noted under gap-109.)
    // gap-111 RESOLVED in CLOC12.114 — the three fixtures
    // (minify_case_string_space / minify_accessor_string_key /
    // minify_new_string_callee) are now ENFORCED (un-ignored).
    // gap-112 RESOLVED in CLOC12.115 — a `for await(...)` header no
    // longer emits a spurious `await`-before-`(` space; the
    // `minify_for_await_bare_stmt` fixture is now ENFORCED (un-ignored).
    // gap-113 RESOLVED in CLOC12.113 — a sub-1 fractional NUMBER (decimal
    // or scientific source) is now canonicalised to the shorter of its
    // leading-zero-stripped decimal and uppercase-`E` scientific forms
    // (decimal wins a length tie at/above magnitude 1e-3):
    //   1e-5  -> 1E-5    .0001 -> 1E-4    1e-3 -> .001    5e-1 -> .5
    // via `small_fraction_shortest_form` in whitespace_only.rs.
    // `minify_num_neg_exp` and `minify_num_frac_4dp` flipped IGNORED ->
    // PASS. (Value>=1 scientific fractionals like `1.23e1`->`12.3` and
    // sub-normal-boundary f64 rounding remain the deferred true-Ryu
    // residual.)
    // gap-114 RESOLVED in CLOC12.116 — a large integer whose lowercase
    // hex form is shorter than decimal is now emitted as `0x…` (over the
    // f64-rounded value); `minify_num_bigint_hex` is now ENFORCED.
    // gap-115 (CLOC14.56) — CORRECTNESS: regex/division disambiguation. A
    // `/` that follows a VALUE-producing token (identifier, number, `)`,
    // `]`) is the DIVISION operator, not the start of a regex literal.
    // closurec's lexer greedily pairs `a/b/c` into `a` + regex `/b/` + `c`
    // and emits `a /b/ c` — which is INVALID JS (two adjacent primaries).
    // Affects `a/b/c`, `4/2/1`, `a/b+c/d`, `(a)/b/c`. A SINGLE division
    // (`a/b`) already lexes correctly. Lexer-level (sibling of gap-044).
    // HIGH PRIORITY — corrupts output to non-parseable JS.
    // gap-115 RESOLVED by F10: once in `div` mode, every subsequent `/` after a
    // value-producing token stays a division, so `a/b/c` lexes as `a / b / c`
    // (byte-identical `a/b/c`), not `a /b/ c`. `div_chain` un-ignored below.
    // gap-116 RESOLVED in CLOC12.116 — a STRING property key that is a
    // CANONICAL non-negative integer (< 2^53) is now unquoted to a numeric
    // key and printed in shortest numeric form: `{"123":1}` -> `{123:1}`,
    // `{"1000":1}` -> `{1E3:1}`. Leading-zero (`"01"`), non-integer
    // (`"1.5"`), and 2^53+ keys stay quoted, and string VALUES (the
    // ternary `a?"1":"2"` confound) are untouched. `numeric_string_key_
    // unquoted` in whitespace_only.rs; `minify_num_str_key` flipped
    // IGNORED → PASS.
    // gap-117 RESOLVED in CLOC12.117 — a `case` clause whose operand begins
    // with a UNARY operator (`-`/`+`/`!`/`~`) needs a separating space that
    // closurec used to omit: `case-1:` -> `case -1:`, `case!a:` -> `case !a:`.
    // Sibling of gap-111 (keyword + string space); here the operand is a
    // unary-prefixed expression rather than a string literal. Fixed via
    // `case_unary_needs_space` in whitespace_only.rs; `minify_case_neg_num`
    // now round-trips byte-identically.
    // gap-105 RESOLVED in CLOC12.109 — CORRECTNESS: LEGACY OCTAL
    // literals (`0` followed by octal digits, e.g. `010`, `017`,
    // `0123`) are sloppy-mode legacy octals denoting their OCTAL value
    // (`010` == 8, `0123` == 83). closurec used to treat the leading
    // zero as insignificant and emit the digits as DECIMAL
    // (`010` -> `10`), CHANGING THE VALUE. Fixed by adding a
    // legacy-octal arm to `normalize_number_value`: when the
    // separator-stripped literal has `len() > 1`, starts with `0`, and
    // every byte is an octal digit, it is decoded with
    // `u128::from_str_radix(.., 8)` (placed AFTER the `0x`/`0o`/`0b`
    // prefix arms, BEFORE the bare-decimal arm). `08`/`09` are not
    // legacy octal and upstream rejects them; `00`/`0o17` are
    // unaffected. `minify_num_legacy_octal` / `..._multi` /
    // `..._array` now ENFORCED.
    // gap-106 (CLOC14.49): a NUMERIC FLOAT property key is normalised to
    // a STRING key by upstream — `{.5:1}` -> `{"0.5":1}`. The float key
    // `.5` is canonicalised to its string form `"0.5"` (the ToString of
    // the numeric property name) and quoted. closurec keeps the raw
    // numeric token `.5`. Integer numeric keys (`{1:2}`) are already
    // byte-identical (both keep `1`); only non-integer numeric keys
    // diverge. Needs object-key-specific number→string canonicalisation.
    // gap-106 RESOLVED in CLOC12.129 — `{.5:1}` → `{"0.5":1}` now
    // byte-identical. `minify_obj_numkey_float` now ENFORCED.
    // gap-103 RESOLVED in CLOC12.107 — a CLASS-BODY computed `get`/`set`
    // accessor preceded by a previous member's `}` (consecutive members)
    // or the `static` modifier now gets the same separating space
    // gap-073 gives object-literal accessors (`get [x](){}`). gap-073's
    // `before_kw` context set gained `}` and `static`; a new method-body
    // guard (the accessor's `)` must be followed by `{`) keeps a
    // statement-block `}` + variable-index-call (`if(x){}get[k](x)`)
    // from being a false positive. `minify_class_accessor_pair` /
    // `minify_class_accessor_after_method` / `minify_class_static_accessor`
    // enforced.
    // gap-104 RESOLVED in CLOC12.108 — CORRECTNESS: a `}` (or
    // object-default `}`) inside a function's PARAMETER LIST made the
    // trailing-`;`-after-`}` rule (gap-030/041 family) mis-fire,
    // injecting a stray `;` that produced INVALID JS:
    //   function f({a=1}={}){}  ->  function f({a=1};={}){}  (was corrupt)
    //   function f({a=1}){}     ->  function f({a=1};){}     (was corrupt)
    //   function f(a={}){}      ->  function f(a={};){}      (was corrupt)
    // The `}` there closes a destructuring-object pattern or an object
    // default VALUE, not a statement block/function body, so no `;` is
    // due. Fixed by suppressing the synthetic `;` whenever the `}`'s
    // immediate follower is `=`/`,`/`)` — a param-list/expression
    // continuation, never a statement boundary. A genuine
    // function-DECLARATION body `}` (the only `}` that owes a `;` here)
    // can never be followed by those tokens, so the FINAL body `}` still
    // gets its `;` (`function f({a=1}={}){}` → `…{};`). The three
    // `param_*` fixtures below are now ENFORCED.
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
    // gap-118 RESOLVED in CLOC12.118 — an UPPERCASE hex literal that is
    // RETAINED in hex form (because hex is the shortest representation)
    // is now lowercased: `0xFFFFFFFFFFFFF` -> `0xfffffffffffff`.
    // Inverse/sibling of gap-114 (decimal -> lowercase hex when shorter):
    // `normalize_number_value` now lowercases the `cleaned` candidate
    // when it is a `0x`/`0X` literal, so it ties the lowercase-hex
    // candidate byte-identically. `minify_hex_upper_retained` flipped
    // IGNORED → PASS.
    // gap-119 (CLOC14.57): a regex literal immediately following the
    // `return` keyword gets a spurious separating space:
    // `return/a/g` -> closurec `return /a/g` but upstream `return/a/g`.
    // Regex/division family (sibling of gap-115): the separator logic
    // inserts a space between the word-like `return` and the `/`-led
    // regex token. Entangled with division disambiguation — deferred
    // until the regex/division lexing (gap-115) is settled.
    // gap-119 RESOLVED by F10: `return` keeps the lexer in `default` (regex)
    // mode, so `return/a/g` lexes the `/a/g` as a single REGEX token with no
    // intervening value token — the separator logic no longer splits it.
    // `regex_after_return` un-ignored below.
    // gap-120 RESOLVED in CLOC12.120 — a NON-INTEGER numeric property key
    // is now emitted as a QUOTED `String(Number(key))` string:
    //   {.5:1} -> {"0.5":1}   {1.5:1} -> {"1.5":1}   {1e-3:1} -> {"0.001":1}
    // Float-key counterpart of gap-116 (canonical INTEGER string key ->
    // unquoted number); INTEGER numeric keys stay bare. The canonical key
    // string is JS `String(Number(key))` (leading-`0`-KEPT decimal,
    // lowercase-`e` sci below 1e-6), computed exactly from the source
    // digits by `noninteger_numeric_key_string` in whitespace_only.rs.
    // `minify_float_key_quoted` flipped IGNORED -> PASS.
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
