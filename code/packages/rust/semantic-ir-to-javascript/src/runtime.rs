//! Inlined JavaScript runtime helpers.
//!
//! Unlike the TypeScript backend — which `import`s a published
//! `@coding-adventures/sir-runtime-core` package — the JavaScript
//! backend produces **fully self-contained** output: every emitted
//! `.js` file pastes this `__Sir` namespace verbatim at the top, so it
//! runs directly via `node file.js` with no `npm install`, no
//! `require()`, and no `import`.  (See `code/specs/SIR18-…md`, "Inlined
//! runtime".)
//!
//! ## Why an IIFE?
//!
//! `const __Sir = (() => { … })();` is the classic JavaScript *module
//! pattern*.  The arrow body is a private scope: the `Sym`/`Pair`/
//! `Closure` classes, the symbol table, and the dispatch helpers live
//! inside it and are invisible to user code.  Only the object returned
//! at the end — `{ Sym, Pair, Closure, intern, … }` — escapes, bound to
//! the single global name `__Sir`.  This mirrors the TypeScript
//! backend's `namespace __Sir { … }` exactly, minus the type
//! annotations.
//!
//! ## Value model (matches the spec's table)
//!
//! | SIR concept | JS representation                         |
//! |-------------|-------------------------------------------|
//! | `Int`/`Float` | native `number`                         |
//! | `Bool`      | native `boolean`                          |
//! | `Nil`       | `null`                                    |
//! | `Symbol`    | `__Sir.Sym` instance with interned `.name`|
//! | `Str`       | native `string`                           |
//! | `Pair`      | `__Sir.Pair` instance (`car`/`cdr`)       |
//! | `Closure`   | `__Sir.Closure` instance wrapping a `fn`  |
//!
//! ## Determinism
//!
//! The constant is fixed text — byte-identical in every artifact — so
//! two compilations of the same module produce the same output.
//!
//! ## Symbolic expressions + pattern/rewrite (SIR23)
//!
//! `__Sir.Symbolic` is a plain-JS port of the published
//! `@coding-adventures/symbolic-ir` / `@coding-adventures/
//! cas-pattern-matching` / `@coding-adventures/sir-runtime-symbolic`
//! TypeScript packages — the same "port it inline" treatment the
//! exception runtime gives `@coding-adventures/sir-runtime-exceptions`
//! (see that section's own comment). A `SymApply`/`SymPatternBlank`/
//! `SymRule`/`SymReplaceAll` node lowers to a call into
//! `__Sir.Symbolic.*`; see that section for the full algorithm.

/// The full inlined runtime.  Always emitted verbatim, exactly once,
/// near the top of every artifact (after the banner, before the user's
/// function declarations).
///
/// Indentation is 2 spaces, every statement is terminated with a
/// semicolon, and the blob ends with a newline so the following
/// declarations start on their own line.
pub const RUNTIME: &str = r##"const __Sir = (() => {
  "use strict";
  // Source-language display convention (SIR display-convention spec).  The
  // emitter substitutes `__SIR_DISPLAY_RUBY__` with `true` when the module's
  // `source_language` is Ruby, else `false` (the default Twig/Lisp form).  The
  // display path (`formatSeen`) reads this to render a boolean as Ruby
  // `true`/`false` rather than the Lisp `#t`/`#f`; existing Twig output is
  // unchanged.
  const SIR_DISPLAY_RUBY = __SIR_DISPLAY_RUBY__;
  // A second, independent display-convention flag: `true` when the
  // module's `source_language` is APL, else `false`. APL's own console
  // convention renders a negative number with the high-minus glyph `¯`
  // (U+00AF), never ASCII `-` (`apl_runtime::value::fmt_num`,
  // `negative_numbers_use_high_minus_not_ascii`). `formatSeen` (below)
  // reads this flag wherever a value could reach `print` as a BARE
  // (unboxed) number or a boxed `SirFloat` -- i.e. every case that is
  // NOT already a genuine (rank >= 1, or rank-0-and-caught-by-the-NDArray
  // branch) `NDArray`, which already renders high-minus unconditionally
  // via `ArrayRt.display`/`fmtNum` (see that branch's own comment).
  //
  // Why this can't be decided from the VALUE alone (a bug-history note):
  // a rank-0 SIR22 `NDArray` is NOT unique to APL -- `matlab-to-semantic-
  // ir`'s `^`/`.^` unconditionally lower to `ElementwiseOp::Pow` even for
  // two literals (no scalar fast path exists for power), so a plain
  // MATLAB `2 ^ 2` is ALSO a rank-0 `{shape: [], data}` object by the
  // time it reaches a consumer -- the identical runtime representation
  // APL's own scalars can arrive as. Yet the two must print with
  // DIFFERENT glyphs: APL wants `¯4` for `-2 ^ 2`-shaped values, while
  // `matlab-to-semantic-ir/tests/oracle.rs`'s own `unary_minus_on_power`
  // case asserts plain ASCII `-4` for the identical shape. A value-shape
  // test genuinely cannot distinguish the two cases; only the SOURCE
  // LANGUAGE that emitted the module can -- hence a second per-module
  // flag, mirroring `SIR_DISPLAY_RUBY` immediately above rather than
  // inventing a new mechanism.
  //
  // Given that, the actual fix keeps `neg`/`sign`/`recip`/`ceil`/`floor`
  // (below) blissfully unaware of source language: a rank-0 (or non-
  // array) operand ALWAYS unwraps to a bare number exactly as before
  // (never boxed into an NDArray just to carry a glyph decision), and
  // ONLY `formatSeen`'s bare-number/`SirFloat` branches consult this flag
  // at the one place the glyph is actually chosen. This is deliberately
  // NOT specific to `neg` — any bare scalar an APL program prints (a
  // literal `-5`, a negated float literal `-3.0`, `sign`/`recip`/`ceil`/
  // `floor`'s own scalar results) goes through the same two branches, so
  // fixing it there fixes all of them in one place.
  const SIR_DISPLAY_APL_HIGH_MINUS = __SIR_DISPLAY_APL_HIGH_MINUS__;
  // ── value model ────────────────────────────────────────────────
  // A symbol is an interned name; `===` on two interned symbols with
  // the same name is therefore identity-equal.
  class Sym {
    constructor(name) { this.name = name; }
  }
  // A cons pair: `(car . cdr)`.  Lisp's fundamental building block.
  class Pair {
    constructor(car, cdr) { this.car = car; this.cdr = cdr; }
  }
  // A first-class function value.  `fn` is the underlying JS function;
  // `applyClosure` invokes it.  Wrapping (rather than using a bare JS
  // function) keeps closures distinguishable from other callables.
  class Closure {
    constructor(fn) { this.fn = fn; }
  }

  // ── symbol interning ───────────────────────────────────────────
  // One table per program; `intern("x")` always returns the *same*
  // Sym object for a given name, so symbol equality is pointer equality.
  const symbolTable = new Map();
  function intern(name) {
    let s = symbolTable.get(name);
    if (s === undefined) { s = new Sym(name); symbolTable.set(name, s); }
    return s;
  }

  // ── closures ───────────────────────────────────────────────────
  function applyClosure(c, args) {
    if (!(c instanceof Closure)) {
      throw new TypeError("apply on non-closure");
    }
    return c.fn(...args);
  }

  // ── tagged floats (Ruby Integer vs Float) ──────────────────────
  //
  // JavaScript has ONE number type (`f64`), so Ruby's `Integer` `7` and
  // `Float` `7.0` are the same JS value — and Ruby distinguishes them
  // everywhere: `7 / 2 == 3` (Integer#/ floors) but `7.0 / 2 == 3.5`
  // (Float#/ true-divides); `puts 7.0` prints `7.0`, not `7`.  The
  // Rust/Go/C backends carry a tagged `Int`/`Float` runtime value; we do
  // the same, but only where JS can't already tell the two apart.
  //
  //   INVARIANT.  A Ruby Integer is an INTEGRAL native `number`.  A Ruby
  //   Float is EITHER a non-integral native `number` (`3.5` — already
  //   distinguishable, so left native) OR a `SirFloat` box wrapping an
  //   integral value (`7.0` — otherwise indistinguishable from `7`).
  //
  // Only integral-valued floats are boxed, so the whole non-integral
  // corpus stays native and untouched.  `mkFloat` is the SOLE factory;
  // everything numeric unwraps through `numOf` and re-tags through
  // `mkFloat`, so the box never escapes to native arithmetic by accident.
  class SirFloat {
    constructor(f) { this.f = f; Object.freeze(this); }
  }
  // Interning gives equal integral floats a single identity, so a boxed
  // `7.0` used as a `Map` key or `Set` member (`tally`, `group_by`,
  // `uniq`, a Hash literal) dedups by identity exactly like Ruby's `eql?`
  // — while native Integer `7` stays a DISTINCT key (`7.eql?(7.0)` is
  // false).  The cache is hard-capped: past the cap `mkFloat` returns
  // fresh un-interned boxes, so memory is bounded (no unbounded-growth
  // DoS) at the cost of losing dedup for programs with more than
  // `FLOAT_INTERN_CAP` distinct integral-float keys — bounded and rare.
  const FLOAT_INTERN_CAP = 4096;
  const floatIntern = new Map();
  function mkFloat(v) {
    if (!Number.isInteger(v)) { return v; } // non-integral float: stays native
    const hit = floatIntern.get(v);
    if (hit !== undefined) { return hit; }
    const box = new SirFloat(v);
    if (floatIntern.size < FLOAT_INTERN_CAP) { floatIntern.set(v, box); }
    return box;
  }
  // `numOf` unwraps to the raw f64 for arithmetic/comparison; `isNum`
  // recognises "a number" at type gates; `isFloat` recognises "a Ruby
  // Float" (boxed integral OR non-integral native).
  //
  // SECOND unwrap case, added alongside the `SirFloat` one above: a
  // rank-0 (scalar) SIR22 `NDArray` — `{ shape: [], data: <1 element> }`
  // (see the "SIR22: array/matrix domain" section far below, `ndarray`/
  // `toArrayValue`). A scalar-only MATLAB accumulator (`n = n + 1` inside
  // a `while` loop, where `n` was never provably scalar to the *frontend*
  // because it is a variable, not a literal — see `matlab-to-semantic-ir`'s
  // `expr_is_known_scalar`) takes the array-domain `ElementwiseOp` codegen
  // path even though every value it ever holds is a plain number, so `n`
  // becomes this NDArray shape after its first update. Every OTHER
  // consumer of `numOf` (arithmetic re-tagging, comparisons) then sees an
  // object where it expects a number; a bare JS `<`/`>`/native `-` on that
  // object coerces through `ToPrimitive` to `NaN`, which is silently
  // wrong (`NaN < 10` is `false`, not an error) rather than a crash. Since
  // `numOf` is the identity on any value it doesn't recognise, and only
  // MATLAB/APL-style SIR22 frontends ever construct an NDArray in the
  // first place, this second branch is a no-op for every other language
  // this backend serves (Ruby, JS, …) and a real fix for this one: it
  // makes "a comparison/negation/subtraction/mod against a 0-D NDArray"
  // behave exactly like "against the plain number it degenerately holds"
  // — fixing not just the while-loop non-termination bug this was written
  // for, but also (for free, same mechanism) unary minus on a scalar power
  // expression (`-2 ^ 2`, which lowers to `ElementwiseOp::Pow` even for two
  // literals and previously gave `NaN` via `neg`'s own `numOf` call).
  function numOf(x) {
    if (x instanceof SirFloat) { return x.f; }
    if (x !== null && typeof x === "object" && Array.isArray(x.shape) && x.shape.length === 0) {
      return x.data[0];
    }
    return x;
  }
  function isNum(x) { return typeof x === "number" || x instanceof SirFloat; }
  function isFloat(x) {
    return x instanceof SirFloat || (typeof x === "number" && !Number.isInteger(x));
  }
  // Arithmetic re-tags: the result is a Float iff an operand is a Float
  // (or the op forces float, e.g. Float#/).  `mkFloat` then leaves a
  // non-integral result native and boxes an integral one (`3.5 + 3.5`
  // → boxed `7.0`; `7.0 - 0.5` → native `6.5`).
  //
  // `x` may ALSO be a genuine SIR22 `{shape, data}` NDArray of rank >= 1
  // (a real APL array, e.g. `-1 2 ¯3`) -- historically this silently gave
  // `NaN`: the old code always fell through to `-numOf(x)`, and `numOf`
  // does not recognise a rank >= 1 NDArray, so native JS unary minus ran
  // on a plain object (`ToPrimitive` coercion) instead of negating any
  // element. Fixed below by mapping over `.data` into a NEW NDArray with
  // the SAME shape (`mapNDArrayRank1Plus`, defined just below `isFloat`'s
  // sibling helpers) -- this is unconditionally correct regardless of
  // source language: no OTHER frontend ever `print`/`disp`s a computed
  // array through `neg`'s result without first reading a scalar element
  // back via `IndexGet` (`formatSeen`'s own NDArray-branch comment below),
  // so only APL's own auto-print ever observes this branch's output today.
  //
  // A rank-0 NDArray operand (e.g. APL's `-(3+4)`, or MATLAB's `-2 ^ 2`,
  // whose `^` always lowers through the SIR22 array domain even for two
  // literals) is DELIBERATELY **not** given its own array-preserving
  // branch here: `mapNDArrayRank1Plus` only matches rank >= 1, so a rank-0
  // operand falls through to the plain `numOf`-unwrapping fallback below,
  // exactly as it always has. The high-minus-vs-ASCII glyph question for
  // that bare scalar RESULT is answered entirely by `formatSeen`'s
  // `SIR_DISPLAY_APL_HIGH_MINUS`-gated branches (see that flag's own
  // comment for why the value itself can't carry the decision) -- not by
  // this function boxing or not boxing its return value.
  function mapNDArrayRank1Plus(x, f) {
    if (
      x !== null && typeof x === "object" &&
      Array.isArray(x.shape) && x.data instanceof Float64Array &&
      x.shape.length >= 1
    ) {
      return ArrayRt.ndarray(x.shape, Float64Array.from(x.data, f));
    }
    return undefined; // not an array (or a rank-0 scalar): caller handles it
  }
  function neg(x) {
    const arr = mapNDArrayRank1Plus(x, (v) => -v);
    if (arr !== undefined) { return arr; }
    return isFloat(x) ? mkFloat(-numOf(x)) : -numOf(x);
  }
  function minus(a, b) {
    const r = numOf(a) - numOf(b);
    return (isFloat(a) || isFloat(b)) ? mkFloat(r) : r;
  }
  function mod(a, b) {
    const r = numOf(a) % numOf(b);
    return (isFloat(a) || isFloat(b)) ? mkFloat(r) : r;
  }
  // Comparisons unwrap through `numOf` before comparing.  Because `numOf`
  // is the IDENTITY on every non-`SirFloat` value, `eq`/`lt`/… are exactly
  // the old native `===`/`<`/… for strings, arrays, nil, and plain numbers
  // — and additionally correct for a boxed Float (`7.0 == 7` true via
  // value; `7.0 < 8` avoids the `NaN` a native `<` on the box would give).
  // `eq` returns Ruby `==` for numbers (by value across Integer/Float).
  function eq(a, b) { return numOf(a) === numOf(b); }
  function ne(a, b) { return numOf(a) !== numOf(b); }
  function lt(a, b) { return numOf(a) < numOf(b); }
  function gt(a, b) { return numOf(a) > numOf(b); }
  function le(a, b) { return numOf(a) <= numOf(b); }
  function ge(a, b) { return numOf(a) >= numOf(b); }
  // Render a boxed float the way Ruby's `to_s` does.  A box only ever
  // holds a FINITE INTEGRAL value (non-finite/non-integral never box), so
  // the job is to restore the trailing `.0` that `String(7)` drops:
  //   7.0        → "7.0"          (append ".0")
  //   -0.0       → "-0.0"         (String(-0) loses the sign — special-case)
  //   1e21       → "1.0e+21"      (insert ".0" BEFORE the exponent)
  // matching the Rust/Go backends ("shortest decimal, `.0` when integral").
  function floatToRubyString(f) {
    if (Object.is(f, -0)) { return "-0.0"; }
    const s = f.toString();
    const e = s.search(/[eE]/);
    if (e >= 0) { return s.slice(0, e) + ".0" + s.slice(e); }
    return s + ".0";
  }

  // ── truthiness ─────────────────────────────────────────────────
  // SIR truthiness, NOT JavaScript's: only `false` and `nil` (null)
  // are falsy.  `0`, `""`, and `NaN` are all truthy — matching Lisp /
  // Ruby semantics rather than JS's surprising coercions.  A `SirFloat`
  // box is an object, so it is truthy — matching Ruby (all numbers are).
  function truthy(v) {
    return v !== false && v !== null && v !== undefined;
  }

  // Real MATLAB/Octave has no separate boolean type: logicals are doubles,
  // and truthiness is "nonzero is true, zero is false" — the OPPOSITE
  // convention from `truthy()` above (canonical SIR truthy(0) is `true`;
  // MATLAB's `~0` is `1`, i.e. `0` is falsy there). A MATLAB/Octave-sourced
  // value reaching a boolean context can be EITHER a genuine JS boolean
  // (the output of a comparison/`~`/`&&`/`||`, which this backend already
  // renders as native `true`/`false`) OR a bare number (a variable, a
  // function-call result, an array-element read, …) that has never passed
  // through a comparison at all — `matlabTruthy` handles both correctly in
  // one place, so the frontend never has to prove, via static shape
  // analysis alone, which case it's looking at (an earlier version of this
  // fix tried exactly that — a lowering-time-only `!= 0` wrap gated on
  // recognising "already boolean" shapes — and got it wrong for the most
  // ordinary case, a variable holding a stored comparison result, silently
  // inverting `false`; see `matlab-to-semantic-ir::lower::to_matlab_condition`
  // for the corrected, always-wrap-through-here approach).
  function matlabTruthy(x) {
    return typeof x === "boolean" ? x : (numOf(x) !== 0);
  }

  // ── display / formatting ───────────────────────────────────────
  // `format` renders any SIR value to the string `print` writes.
  // Strings render WITHOUT surrounding quotes (so `print` of a string
  // shows its contents); everything else uses a Lisp-ish notation.
  // `format` renders a value the way Ruby's `to_s`/inspect would for the
  // contexts the runtime needs (array display, string interpolation, the
  // `Array#*`-with-a-string join, string `+` on a non-string operand's
  // display, …).  A JS array is a shared mutable reference, so a program can
  // build a *cyclic* array (`a = []; a << a`).  The array branch recurses
  // through elements, so — exactly like `putsOne` (see its comment) — it MUST
  // be cycle-guarded or a self-referential array throws `RangeError: Maximum
  // call stack size exceeded` (a DoS: CWE-674, uncontrolled recursion).  This
  // path is reachable from the polymorphic `+`/`*` arms (`str + cyclicArray`,
  // `cyclicArray * ", "`), not just `puts`.  `seen` holds the array references
  // on the active render path; an array already on the path is a cycle and
  // renders as `[...]` (matching Ruby's `inspect`), then we recurse no further.
  function format(v) { return formatSeen(v, new Set()); }
  function formatSeen(v, seen) {
    if (v === null || v === undefined) { return "nil"; }
    if (v === true) { return SIR_DISPLAY_RUBY ? "true" : "#t"; }
    if (v === false) { return SIR_DISPLAY_RUBY ? "false" : "#f"; }
    if (typeof v === "string") { return v; }
    // A boxed Float renders with its trailing `.0` (`7.0`, not `7`); a native
    // number (Integer, or a non-integral Float like `3.5`) renders as-is.
    // This is Ruby/Lisp's OWN convention -- an APL-sourced module renders
    // EITHER shape through `ArrayRt.fmtNum` instead (high-minus `¯`, no
    // trailing `.0` ever), matching `apl_runtime::value::fmt_num` exactly.
    // See `SIR_DISPLAY_APL_HIGH_MINUS`'s own comment (near the top of this
    // file) for why this decision has to live HERE (at display time) and
    // not inside `neg`/`sign`/`recip`/`ceil`/`floor` themselves: a rank-0
    // SIR22 NDArray -- the representation a bare/boxed scalar RESULT from
    // any of those five degenerately unwraps from -- is not unique to APL
    // (MATLAB's `2 ^ 2` reaches the identical shape), so only the source
    // language, not the value's own shape, can decide the glyph.
    if (v instanceof SirFloat) {
      return SIR_DISPLAY_APL_HIGH_MINUS ? ArrayRt.fmtNum(v.f) : floatToRubyString(v.f);
    }
    if (typeof v === "number") {
      return SIR_DISPLAY_APL_HIGH_MINUS ? ArrayRt.fmtNum(v) : String(v);
    }
    if (v instanceof Sym) { return v.name; }
    if (v instanceof Pair) {
      return "(" + formatSeen(v.car, seen) + " . " + formatSeen(v.cdr, seen) + ")";
    }
    if (v instanceof Closure) { return "#<closure>"; }
    if (Array.isArray(v)) {
      if (seen.has(v)) { return "[...]"; }
      seen.add(v);
      const body = v.map((el) => formatSeen(el, seen)).join(", ");
      seen.delete(v);
      return "[" + body + "]";
    }
    // A Ruby Hash is a JS `Map`.  It renders `{k: v, …}` (colon-space between
    // key and value, comma-space between pairs) — the SAME surface the Go/Rust
    // backends emit, so a printed hash (e.g. a `group_by` result) round-trips
    // identically across backends.  Cycle-guarded via `seen` like Arrays.
    if (v instanceof Map) {
      if (seen.has(v)) { return "{...}"; }
      seen.add(v);
      const body = [...v]
        .map(([k, val]) => formatSeen(k, seen) + ": " + formatSeen(val, seen))
        .join(", ");
      seen.delete(v);
      return "{" + body + "}";
    }
    // A SIR23 symbolic-expression term (see the "symbolic expressions"
    // section far below) — a plain frozen `{ kind: "symbol"|"integer"|…
    // }` object, never a class instance. Unlike Array/Map, a term is
    // built exclusively through `Symbolic.*`'s constructors, which only
    // ever wrap ALREADY-frozen children, so a term can never reference
    // itself; no cycle guard is needed here the way Array/Map need `seen`.
    if (v !== null && typeof v === "object" && typeof v.kind === "string") {
      const s = Symbolic.toDisplayString(v);
      if (s !== undefined) { return s; }
    }
    // SIR22/APL: an `NDArray` (the `{ shape, data }` value this file's
    // "array/matrix domain" section below constructs) has no Ruby/Scheme
    // display convention of its own. The MATLAB frontend never reaches this
    // branch -- it always reads a computed array back through a scalar
    // `IndexGet` instead of printing the whole thing (see
    // `semantic-ir-to-javascript`'s own `tests/sir22_array.rs` doc
    // comments) -- but APL auto-prints a bare top-level expression (see
    // `apl-to-semantic-ir`'s "Auto-print, not MATLAB-style suppression"),
    // and APL has no bracket-indexing syntax to read a value back with, so
    // a real APL program's `print` call can only ever be made to work by
    // rendering the NDArray itself. `ArrayRt.display` (below) is a 1:1 port
    // of `apl_runtime::value::display` -- APL's OWN console convention
    // (high-minus `¯` negatives, no name/`ans=` prefix), which is exactly
    // what an `apl-runtime` session would print for the same value.
    if (v !== null && typeof v === "object" && Array.isArray(v.shape) && v.data instanceof Float64Array) {
      return ArrayRt.display(v);
    }
    return String(v);
  }

  // ── builtins dispatch table ────────────────────────────────────
  // Reached only for builtins the emitter did not specialise inline
  // (e.g. a variadic `+`, or a builtin referenced as a value via
  // `__Sir.builtins["name"]`).  Each entry is an ordinary JS function.
  // Numeric fold shared by `plus`/`times`/`-`/`/`.  It unwraps every operand
  // through `numOf` (so `step` always sees raw f64s), tracks whether ANY
  // operand — including `init` — was a Ruby Float, and re-tags the result
  // via `mkFloat` iff so.  Thus `1 + 2` stays a native Integer, `3.5 + 3.5`
  // becomes the boxed Float `7.0`, and `1 + 2.5` stays the native `3.5`.
  function numFold(args, init, step) {
    let acc = numOf(init);
    let anyFloat = isFloat(init);
    for (const a of args) {
      if (isFloat(a)) { anyFloat = true; }
      acc = step(acc, numOf(a));
    }
    return anyFloat ? mkFloat(acc) : acc;
  }
  // ── SIR22/APL monadic scalar atoms: sign / reciprocal / ceiling / floor ──
  //
  // APL's monadic `× ÷ ⌈ ⌊` (`apl-to-semantic-ir/src/lower.rs`'s
  // `apply_monadic_scalar`) lower to `BuiltinCall("sign"/"recip"/"ceil"/
  // "floor", [x])`. These four names were documented (this crate's own
  // README/CHANGELOG) but never given a runtime implementation anywhere in
  // this file OR in `emit.rs`'s fixed-arm table, so every one of them
  // crashed with `TypeError: unknown builtin: <name>` for EVERY operand,
  // scalar or array (found by `apl-to-semantic-ir/tests/oracle.rs`, this
  // crate's own oracle harness). Ported 1:1 from `apl_runtime::eval::
  // apply_monadic_scalar`/`apl_sign` (`code/packages/rust/apl-runtime/
  // src/eval.rs`):
  //   - `aplSign`: NaN → NaN; positive → 1; negative → -1; zero (either
  //     sign) → 0. Deliberately NOT `Math.sign()`: although `Math.sign(0)
  //     === 0` and `Math.sign(-0) === -0` happen to compare `=== 0` (so a
  //     bare `Math.sign` call would likely also pass), `aplSign` is
  //     written to match the Rust reference's explicit if/else branching
  //     literally rather than lean on that coincidence.
  //   - `aplRecip`: plain `1 / v`, IEEE-754 -- `aplRecip(0)` is `Infinity`,
  //     never an error/`NaN` (unlike Ruby's `ZeroDivisionError`-raising
  //     `divide()` elsewhere in this file).
  //   - ceiling/floor: plain `Math.ceil`/`Math.floor` directly -- no APL
  //     comparison-tolerance quirk exists anywhere in this codebase for
  //     these two. (`runtime.rs` also has `"floor"`/`"ceil"` CASE LABELS
  //     inside `numericMethod`'s `switch`, but that is Ruby's UNRELATED
  //     `recv.floor`/`recv.ceil` METHOD-call dispatch, reached only via
  //     `BuiltinCall("__method__", ...)` -- never via a bare top-level
  //     `BuiltinCall("floor"/"ceil", ...)` the way APL's monadic atoms emit
  //     one. The two mechanisms coexist without collision.)
  //
  // `monadicScalarAtom` is the scalar/array dispatch every one of the four
  // needs: a genuine NDArray of rank >= 1 maps `f` elementwise (reusing
  // `mapNDArrayRank1Plus`, defined next to `neg` above -- the exact same
  // "rank >= 1 preserves the box, everything else falls through" split
  // `neg`'s own array branch uses); anything else (a bare number, a boxed
  // `SirFloat`, or a rank-0 NDArray) unwraps via `numOf` and returns a BARE
  // result -- deliberately never re-boxing through `mkFloat` the way `neg`/
  // `minus`/`mod` do for Ruby, because none of these four names is ever
  // emitted by a Ruby-sourced module (confirmed by a repo-wide grep for
  // `"sign"`/`"recip"`/`"ceil"`/`"floor"` as `BuiltinCall` names: only
  // `apl-to-semantic-ir` and the not-yet-`node`-tested `j-to-semantic-ir`
  // emit them) -- so there is no Integer-vs-Float distinction to preserve,
  // and re-boxing would actively be WRONG: `mkFloat` would box a whole-
  // valued result like `⌈3.2` (`Math.ceil(3.2) === 4`) into a `SirFloat`,
  // which `formatSeen` would then render Ruby-style with a spurious
  // trailing `.0` (`"4.0"`) instead of APL's own `"4"`. As with `neg`, the
  // glyph question for a bare/rank-0 result is answered entirely by
  // `formatSeen`'s `SIR_DISPLAY_APL_HIGH_MINUS`-gated branches, not here.
  function aplSign(v) {
    if (Number.isNaN(v)) { return NaN; }
    if (v > 0) { return 1; }
    if (v < 0) { return -1; }
    return 0;
  }
  function aplRecip(v) { return 1 / v; }
  function monadicScalarAtom(x, f) {
    const arr = mapNDArrayRank1Plus(x, f);
    if (arr !== undefined) { return arr; }
    return f(numOf(x));
  }

  // NULL-PROTOTYPE table.  `builtins[name]` is indexed by a SOURCE-DERIVED
  // name, so a plain object literal would resolve inherited `Object.prototype`
  // members — `builtins["toString"]`, `["constructor"]`, `["__defineGetter__"]`
  // all yield functions, sail past the `f === undefined` check in
  // `callBuiltin`/`builtinClosure`, and get INVOKED (a define-a-getter-on-
  // global gadget).  `Object.create(null)` removes the prototype chain, so an
  // unknown name is `undefined` and raises cleanly.  This matches how the
  // runtime's other name-indexed tables (`ancestry`, the ivar bags) are built.
  const builtins = Object.assign(Object.create(null), {
    // `+`/`*` route through the polymorphic helpers (hoisted function
    // declarations below) so a builtin referenced as a VALUE, or a
    // variadic `(+ 1 2 3)`, gets the same string/array/numeric dispatch
    // as the inlined 2-arg form.
    "+": (...a) => plus(...a),
    "-": (...a) => a.length === 1 ? neg(a[0]) : numFold(a.slice(1), a[0], (x, y) => x - y),
    "*": (...a) => times(...a),
    "/": (...a) => a.length === 1
      ? (isFloat(a[0]) ? mkFloat(1 / numOf(a[0])) : 1 / numOf(a[0]))
      : numFold(a.slice(1), a[0], (x, y) => x / y),
    "=": (x, y) => eq(x, y),
    // Ruby case-equality (`pattern === value`) — the test a `when`/`in` arm
    // runs.  Ruby keys `===` to the pattern's type (Range → membership, Regexp
    // → match); this backend has no Range/Regexp value, so the only patterns
    // that reach here are plain values and the op is ordinary equality (the
    // same `===` the `=` builtin uses).  `when SomeClass` is lowered to
    // `.is_a?` at the frontend and never becomes a case_eq call.
    "case_eq": (pattern, value) => eq(pattern, value),
    "<": (x, y) => lt(x, y),
    ">": (x, y) => gt(x, y),
    "<=": (x, y) => le(x, y),
    ">=": (x, y) => ge(x, y),
    "not": (x) => !truthy(x),
    "neg": (x) => neg(x),
    // SIR22/APL monadic scalar atoms (see the section just above this
    // table for the full root-cause writeup and ground-truth citations).
    "sign": (x) => monadicScalarAtom(x, aplSign),
    "recip": (x) => monadicScalarAtom(x, aplRecip),
    "ceil": (x) => monadicScalarAtom(x, Math.ceil),
    "floor": (x) => monadicScalarAtom(x, Math.floor),
    "cons": (x, y) => new Pair(x, y),
    "car": (p) => p.car,
    "cdr": (p) => p.cdr,
    "pair?": (p) => p instanceof Pair,
    "null?": (x) => x === null || x === undefined,
    "number?": (x) => isNum(x),
    "symbol?": (x) => x instanceof Sym,
    // Type reflection as BUILTINS: the Ruby frontend lowers `x.is_a?(Foo)`
    // and a `case/in Foo` class pattern to `BuiltinCall("is_a?", [x,
    // StrLit("Foo")])` — the class arrives as its NAME, so no constant-
    // reference support is needed here.
    "is_a?": (v, cls) => isA(v, methodNameArg(cls)),
    "kind_of?": (v, cls) => isA(v, methodNameArg(cls)),
    "instance_of?": (v, cls) => rubyClassName(v) === methodNameArg(cls),
    "class": (v) => rubyClassName(v),
    "len": (x) => x.length,
    "print": (x) => { console.log(format(x)); return null; },
    "puts": (...args) => puts(...args),
    "range": (start, stop, step) => {
      const out = [];
      const s = step === undefined || step === null ? 1 : step;
      if (s >= 0) { for (let i = start; i < stop; i += s) { out.push(i); } }
      else { for (let i = start; i > stop; i += s) { out.push(i); } }
      return out;
    },
  });
  // A builtin referenced as a value (e.g. passed to `map`) becomes a
  // Closure wrapping the table entry, so it round-trips through
  // `applyClosure` like any other first-class function.
  function builtinClosure(name) {
    const f = builtins[name];
    if (f === undefined) { throw new TypeError("unknown builtin: " + name); }
    return new Closure(f);
  }
  function callBuiltin(name, args) {
    const f = builtins[name];
    if (f === undefined) { throw new TypeError("unknown builtin: " + name); }
    return f(...args);
  }
  // `print` is promoted to a top-level member so emitted code can write
  // the readable `__Sir.print(x)` rather than `__Sir.builtins["print"](x)`.
  function print(x) { console.log(format(x)); return null; }

  // ── puts (Ruby semantics) ──────────────────────────────────────
  //
  // Ruby's `puts` is THE common output method and is deceptively subtle:
  //
  //   - `puts`            → one newline.
  //   - `puts x`          → `x.to_s` then a newline, UNLESS `x.to_s` already
  //                         ends in "\n" (then no second newline is added):
  //                         `puts "x\n"` prints `x\n`, not `x\n\n`.
  //   - `puts a, b`       → each argument on its own line, in order.
  //   - `puts nil`        → a blank line (`nil.to_s` is "", then the newline).
  //   - `puts []`         → a single newline (an argument that flattens to
  //                         nothing still prints a blank line).
  //   - `puts [1,[2,3]]`  → each ELEMENT on its own line, arrays flattened
  //                         recursively: `1\n2\n3\n`.
  //
  // We write via `process.stdout.write` rather than `console.log` because
  // `console.log` unconditionally appends a newline, defeating the
  // trailing-newline-suppression rule.  A sequence is a native JS array.
  // Cycle safety: a JS array is a shared, mutable reference, so a program can
  // build a *cyclic* array (`a = []; a << a`).  The element-per-line flatten
  // recurses through nested arrays, so it MUST be cycle-guarded or a self-
  // referential array throws `RangeError: Maximum call stack size exceeded`
  // (a DoS: CWE-674, uncontrolled recursion).  `seen` is a `Set` of the array
  // references currently on the active flatten path.  A array ALREADY on the
  // path is a cycle: rather than recurse forever we write `[...]` and a
  // newline — matching real Ruby, where `puts a` on a self-referential array
  // prints `[...]` and terminates.  (`format` is not itself cycle-guarded, so
  // we emit the placeholder directly instead of calling it on the cycle.)  An
  // array removed from `seen` on exit still flattens in full via a sibling
  // path — only a true self-cycle is short-circuited, so non-cyclic output is
  // unchanged (`puts [1,[2,3]]` still prints `1\n2\n3\n`).
  function putsOne(v, seen) {
    if (Array.isArray(v)) {
      if (seen.has(v)) { process.stdout.write("[...]\n"); return; }
      seen.add(v);
      for (const item of v) { putsOne(item, seen); }
      seen.delete(v);
      return;
    }
    if (v === null || v === undefined) { process.stdout.write("\n"); return; }
    const text = format(v);
    process.stdout.write(text.endsWith("\n") ? text : text + "\n");
  }
  function puts(...args) {
    if (args.length === 0) { process.stdout.write("\n"); return null; }
    const seen = new Set();
    for (const a of args) {
      // `puts []` (empty array arg) still writes one blank line.
      if (Array.isArray(a) && a.length === 0) { process.stdout.write("\n"); }
      else { putsOne(a, seen); }
    }
    return null;
  }

  // ── polymorphic `+` / `*` (Ruby operator overloading) ──────────
  //
  // Ruby overloads `+` and `*` by the RECEIVER's runtime type, and all of
  // these lower to the same SIR `+`/`*` builtins, so the dispatch has to
  // happen HERE at runtime, on the FIRST operand's type:
  //
  //   | expr          | Ruby result   | arm                             |
  //   |---------------|---------------|---------------------------------|
  //   | `1 + 2`       | `3`           | numeric fold (unchanged)        |
  //   | `"a" + "b"`   | `"ab"`        | string concat                   |
  //   | `[1] + [2]`   | `[1, 2]`      | array concat (NEW array)        |
  //   | `"ab" * 3`    | `"ababab"`    | string repeat                   |
  //   | `[0] * 3`     | `[0, 0, 0]`   | array repeat (NEW array)        |
  //   | `[1,2] * ", "`| `"1, 2"`      | array join (via `format`)       |
  //
  // Dispatch is `typeof x === "string"` / `Array.isArray(x)` — a runtime
  // TAG test, NEVER reflection / `eval` / property access on a
  // source-derived name (the C3 RCE lesson).  The numeric arm is byte-for-
  // byte the old behaviour; the string/array arms sit strictly *ahead* of
  // it, so every existing numeric program is unchanged.
  //
  // SECURITY — repeat-count guard (CWE-1284 / CWE-400).  The two repeat
  // arms multiply a length by a PROGRAM-CONTROLLED `count`.  Unguarded,
  // `String.prototype.repeat` throws a raw `RangeError` on a negative or
  // huge count, and an array-repeat loop can allocate until the process
  // OOMs — a denial-of-service.  `repeatCount` normalises `count` to a
  // safe non-negative integer and rejects an oversized product with a
  // Ruby-shaped `ArgumentError: argument too big` (matching Ruby, which
  // raises `ArgumentError` for `"x" * (2**62)`).  A non-finite, non-
  // integer, or `count <= 0` yields `0` → an empty result (Ruby: `"x" * 0
  // == ""`, `"x" * -1` raises, but we clamp to empty for DoS-safety since
  // the typed-error cascade owns the raise).  Callers short-circuit an
  // empty receiver so a huge count on `"" * n` / `[] * n` does no work.
  const MAX_REPEAT_ELEMS = Number.MAX_SAFE_INTEGER;
  function repeatCount(unitLen, count) {
    // Reject anything that is not a finite integer, and clamp <= 0 to 0.
    if (typeof count !== "number" || !Number.isFinite(count) ||
        !Number.isInteger(count) || count <= 0) {
      return 0;
    }
    // Guard the product against the safe-integer cap before we allocate.
    if (unitLen > 0 && count > MAX_REPEAT_ELEMS / unitLen) {
      raiseError("ArgumentError", "argument too big");
    }
    return count;
  }
  // `+` — left-associative fold so the variadic contract survives.  The
  // arm is chosen by the FIRST operand; `plus()` with no args is `0`,
  // mirroring the numeric identity.
  function plus(...args) {
    if (args.length === 0) { return 0; }
    const first = args[0];
    if (typeof first === "string") {
      // String concat: render every operand through `format` (the same
      // display used by `puts`/`print`) and join.  A string first operand
      // means Ruby's `String#+`.
      let acc = "";
      for (const a of args) { acc += format(a); }
      return acc;
    }
    if (Array.isArray(first)) {
      // Array concat into a FRESH array — no aliasing or mutation of any
      // input (do NOT reuse an operand).  Non-array operands are pushed as
      // single elements only if they are arrays; Ruby's `Array#+` requires
      // array operands, so we concat each array operand's elements.
      const out = [];
      for (const a of args) {
        if (Array.isArray(a)) { for (const e of a) { out.push(e); } }
        else { out.push(a); }
      }
      return out;
    }
    // Numeric fold (unchanged): int/float promotion via native `+`.
    return numFold(args.slice(1), first, (x, y) => x + y);
  }
  // `*` — Ruby `*` is BINARY (receiver * arg).  We handle the binary
  // string/array arms explicitly and keep the variadic numeric fold for
  // the ≥2-arg numeric case (`(* 2 3 4)`).
  function times(...args) {
    const first = args[0];
    if (args.length === 2) {
      const rhs = args[1];
      if (typeof first === "string" && isNum(rhs)) {
        // String repeat: `"ab" * 3` → "ababab".  Empty receiver short-
        // circuits so a huge count does no work; the guard rejects an
        // oversized product before `repeat` allocates.  A boxed-Float count
        // unwraps via `numOf`, then `repeatCount` rejects a non-integer.
        if (first.length === 0) { return ""; }
        const n = repeatCount(first.length, numOf(rhs));
        return n === 0 ? "" : first.repeat(n);
      }
      if (Array.isArray(first) && isNum(rhs)) {
        // Array repeat: `[0] * 3` → [0, 0, 0], a NEW array.  Empty
        // receiver short-circuits; the guard bounds total elements.
        if (first.length === 0) { return []; }
        const n = repeatCount(first.length, numOf(rhs));
        const out = [];
        for (let i = 0; i < n; i++) {
          for (const e of first) { out.push(e); }
        }
        return out;
      }
      if (Array.isArray(first) && typeof rhs === "string") {
        // Array join: `[1, 2] * ", "` → "1, 2".  Elements render through
        // `format` (the SAME display helper `puts` uses), joined by the
        // separator string.  Matches Ruby's `Array#*` with a String arg.
        return first.map(format).join(rhs);
      }
    }
    // Numeric fold (unchanged): variadic product, identity 1.
    return numFold(args, 1, (x, y) => x * y);
  }

  // ── division `/` (Ruby ZeroDivisionError) ──────────────────────
  //
  // Ruby raises `ZeroDivisionError` ("divided by 0") for `1 / 0` — for BOTH
  // integer and float receivers (`1 / 0` and `1.0 / 0` both raise; Ruby's
  // `Float#/` by an integer zero raises, unlike bare float math which gives
  // `Infinity`).  Native JavaScript `/` never throws: `1 / 0 === Infinity`
  // and `0 / 0 === NaN`.  So the backend routes the binary `/` builtin
  // through this helper, which ADDS the explicit zero-divisor check and
  // raises a typed `SirError` (`ZeroDivisionError`) that a translated
  // `rescue ZeroDivisionError` catches — matching Ruby exactly.
  //
  // We only special-case a divisor that is exactly `0` (integer or the
  // float `0`/`-0`); any other numeric divisor divides natively as before,
  // so no existing numeric program changes.  Note `1.0 / 0.0` in Ruby also
  // raises `ZeroDivisionError` (it does NOT return `Float::INFINITY`), and
  // `0 === -0` and `0 === 0.0` in JS, so the single `=== 0` test covers the
  // integer-zero, float-zero, and negative-zero divisors uniformly.
  function divide(a, b) {
    const an = numOf(a), bn = numOf(b);
    if (bn === 0) {
      raiseError("ZeroDivisionError", "divided by 0");
    }
    // Ruby's `/` is polymorphic on the RECEIVER's type: `Integer#/` FLOORS
    // toward −∞ (`-7 / 2 == -4`), while `Float#/` true-divides (`7.0 / 2 ==
    // 3.5`).  With tagged floats the two are now distinguishable: if EITHER
    // operand is a Ruby Float, true-divide and re-tag the result (`6.0 / 2`
    // → boxed `3.0`, `7.0 / 2` → native `3.5`); otherwise both are Integers,
    // so floor — matching the SIR21 §E3 oracle `DivOp::Floor` on every sign
    // combination.  (A boxed Float is unwrapped via `numOf` for the math.)
    return (isFloat(a) || isFloat(b)) ? mkFloat(an / bn) : Math.floor(an / bn);
  }

  // ── method dispatch (`__method__`) ─────────────────────────────
  // `recv.meth(args…)` reaches the backend as
  // `BuiltinCall("__method__", [recv, "meth", args…])`; the emitter routes
  // it here as `callMethod(recv, "meth", args…)`.  We dispatch to the
  // JS-native method on the receiver — arrays' `push`/`pop`/`map`/`filter`/
  // `forEach`/`includes`/`reduce`/…, strings' `toUpperCase`/… — so
  // frontend collection code runs end-to-end (C3/C4).
  //
  // A callback argument arrives as a `Closure` (the frontend lowers an
  // arrow / function argument to `MakeClosure`); `unwrapArg` turns it into
  // an ordinary JS function via `applyClosure`, so `arr.map(fn)` and
  // friends receive a callable the native method can invoke.  `length` is
  // accepted as a zero-arg method too (a property read spelled as a call),
  // though the frontend normally lowers bare `.length` to `SeqLen`.
  function unwrapArg(a) {
    if (a instanceof Closure) {
      // Native higher-order methods pass (element, index, array); the SIR
      // closure only binds the params it declared, so extra JS arguments
      // are harmlessly ignored by `applyClosure`'s spread.
      return (...xs) => applyClosure(a, xs);
    }
    return a;
  }
  // ── method-name allowlist (SECURITY, load-bearing) ─────────────
  // `name` here is ATTACKER-CONTROLLED: it originates as a source-level
  // method name in an untrusted input program and reaches us verbatim.
  // `recv[name]` is therefore an *unrestricted* dynamic property lookup, and
  // a handful of JavaScript member names are reflective gadgets that turn
  // that lookup into arbitrary-code execution.  The worst is `constructor`:
  // on any function it yields the `Function` constructor, so a translated
  // program can write
  //     id.constructor("return …payload…")
  // to synthesise and run evil code — and a native higher-order method
  // (Array.prototype.map/filter/…) will then invoke the result.  That is a
  // remote-code-execution hole.  `apply`/`call`/`bind` re-bind `this`, and
  // `__proto__`/`prototype`/`__define*etter__`/`__lookup*etter__`/`valueOf`/
  // `hasOwnProperty` are the other prototype-chain escape hatches.
  //
  // We therefore dispatch ONLY through this fixed allowlist of known-safe
  // collection / String / Number methods.  Anything not on the list — every
  // gadget above included, none of which appear here — is rejected with a
  // TypeError *before* any property is looked up or invoked.  This is the
  // primary gate: the emitted JS is what actually executes, so the allowlist
  // must live here (the frontend denylist is defense in depth in front of it).
  const METHOD_ALLOWLIST = new Set([
    // Array
    "push", "pop", "shift", "unshift", "slice", "splice", "concat",
    "map", "filter", "reduce", "reduceRight", "forEach", "includes",
    "indexOf", "lastIndexOf", "join", "sort", "reverse", "find",
    "findIndex", "some", "every", "flat", "flatMap", "fill", "at",
    "keys", "values", "entries",
    // String
    "toUpperCase", "toLowerCase", "trim", "trimStart", "trimEnd", "split",
    "charAt", "charCodeAt", "codePointAt", "substring", "repeat",
    "startsWith", "endsWith", "replace", "replaceAll", "padStart", "padEnd",
    // Number
    "toFixed", "toString",
  ]);
  // Ruby method names whose spelling differs from the JS-native equivalent.
  // Applied in `callMethod` BEFORE the security allowlist check, so a Ruby
  // spelling like `upcase` normalises to `toUpperCase` — which IS on the
  // allowlist — while the allowlist itself stays a fixed set of native names
  // (the reflective-gadget gate is unchanged).  ONLY unambiguous 1:1 renames
  // belong here; semantics-diverging pairs (e.g. Ruby `gsub` vs JS `replaceAll`,
  // whose replacement/global rules differ) are deliberately omitted.  Every
  // value below already appears in `METHOD_ALLOWLIST`.
  const RUBY_METHOD_ALIASES = {
    "upcase": "toUpperCase",
    "downcase": "toLowerCase",
    "strip": "trim",
    "lstrip": "trimStart",
    "rstrip": "trimEnd",
    "start_with?": "startsWith",
    "end_with?": "endsWith",
    "include?": "includes",
  };

  // ── M6: universal Object metaprogramming surface ───────────────
  //
  // Ruby's Kernel / Object mixes a handful of methods into EVERY object,
  // independent of the receiver's type.  M6 ports the same four groups the
  // Python / TypeScript backends carry, matching their return-value rules
  // exactly:
  //
  //   • `send` / `__send__` / `public_send` — the FIRST argument (a Symbol or
  //     string) names a method; re-enter dispatch with that name + the
  //     remaining args.  SECURITY-CRITICAL (the C3 RCE lesson): the dynamic
  //     name routes through the SAME `callMethod` — so a SirInstance goes
  //     through the explicit `(class, method)` Map, and a primitive goes
  //     through the fixed `METHOD_ALLOWLIST`.  We NEVER do `recv[name]`, `eval`,
  //     `new Function`, or any host reflection on the source-derived name; an
  //     unknown name floors to the same `NoMethodError` a direct call raises.
  //   • `tap` — yields the receiver to the block, returns the RECEIVER (the
  //     "run a side effect, keep the value" pipeline method).
  //   • `then` / `yield_self` — yields the receiver to the block, returns the
  //     BLOCK'S RESULT (functional "pipe into a block"); block-less `then`
  //     returns the receiver (matching Python's v0 floor).
  //   • `respond_to?` — true iff dispatch on the receiver would resolve the
  //     named method, checked against the SAME tables/allowlist dispatch uses
  //     (`respondsTo` below), so it stays honest.
  //   • boolean `&` / `|` / `^` on a `true`/`false` receiver — Ruby's *eager*
  //     (non-short-circuiting) logical operators, distinct from the lazy
  //     `&&`/`||` keywords; the operand is coerced by SIR `truthy`.
  //
  // The names below are recognised BEFORE the native-method allowlist gate, so
  // they resolve on primitives (which the allowlist would otherwise reject) and
  // on SirInstances (after a user override misses).  `respond_to?` reporting is
  // driven by `respondsTo`, never by probing `recv[name]`.
  const SEND_METHODS = new Set(["send", "__send__", "public_send"]);
  const OBJECT_BLOCK_METHODS = new Set(["tap", "then", "yield_self"]);
  const BOOL_METHODS = new Set(["&", "|", "^"]);

  // Coerce a `respond_to?` / `send` name argument (a `Sym`, a `":m"`-ish
  // string, or a bare name) to the plain method name used as the dispatch key.
  function methodNameArg(arg) {
    if (arg instanceof Sym) { return arg.name; }
    return String(arg);
  }

  // Whether dispatch on `recv` would resolve `name` — checked against the SAME
  // structures `callMethod` uses, so `respond_to?` never lies:
  //   • a SirInstance: the user method table walking its MRO/ancestry;
  //   • any receiver: the universal M6 surface (send family + tap/then/
  //     yield_self + respond_to?), and — on a boolean — the eager operators;
  //   • a primitive: the native-method allowlist (after Ruby→native aliasing).
  // This is pure DATA lookup (Set membership + `Map.get` on a `(class, method)`
  // key), never `recv[name]` / reflection — a name like `constructor` is inert.
  function respondsTo(recv, name) {
    if (recv instanceof SirInstance) {
      if (resolveMethod(methodTable, recv.sirClass, name, includedModules) !== undefined) {
        return true;
      }
    }
    if (name === "respond_to?" || SEND_METHODS.has(name) || OBJECT_BLOCK_METHODS.has(name)) {
      return true;
    }
    // Type reflection answers on every receiver (matching the Go backend,
    // which reports `class`/`is_a?`/`kind_of?`/`instance_of?` universally).
    if (name === "class" || REFLECT_PREDICATES.has(name)) { return true; }
    if (typeof recv === "boolean" && BOOL_METHODS.has(name)) { return true; }
    // A number resolves the hand-implemented Ruby Numeric catalog (kept in
    // lockstep with `numericMethod`'s case labels), ahead of the native gate.
    if (isNum(recv) && NUMERIC_METHODS.has(name)) { return true; }
    // A string resolves the hand-implemented Ruby String catalog (in lockstep
    // with `stringMethod`'s case labels), ahead of the native gate.
    if (typeof recv === "string" && STRING_METHODS.has(name)) { return true; }
    // A Hash (`Map`) resolves the hand-implemented Ruby Hash catalog (in
    // lockstep with `hashMethod`'s case labels), ahead of the native gate.
    if (recv instanceof Map && HASH_METHODS.has(name)) { return true; }
    // A Symbol resolves the hand-implemented Ruby Symbol catalog (in lockstep
    // with `symbolMethod`'s case labels), ahead of the native gate.
    if (recv instanceof Sym && SYMBOL_METHODS.has(name)) { return true; }
    // An Array resolves the hand-implemented Ruby Array/Enumerable catalog (in
    // lockstep with `arrayMethod`'s case labels), ahead of the native gate.
    if (Array.isArray(recv) && ARRAY_METHODS.has(name)) { return true; }
    // A primitive resolves a name iff it (or its Ruby→native alias) is on the
    // method allowlist AND the native member is actually a function on `recv`.
    if (!(recv instanceof SirInstance)) {
      const native = Object.prototype.hasOwnProperty.call(RUBY_METHOD_ALIASES, name)
        ? RUBY_METHOD_ALIASES[name]
        : name;
      if (name === "length" || name === "fetch") { return true; }
      if (METHOD_ALLOWLIST.has(native)) {
        return recv != null && typeof recv[native] === "function";
      }
    }
    return false;
  }

  // Eager boolean operators on a `true`/`false` receiver.  Returns the sentinel
  // `BOOL_MISS` when `name` is not an operator (or called with no operand) so
  // the caller can fall through — mirroring Python/TS `_MISS`.
  const BOOL_MISS = Symbol("bool-miss");
  function boolMethod(recv, name, args) {
    if (!BOOL_METHODS.has(name) || args.length === 0) { return BOOL_MISS; }
    const other = truthy(args[0]);
    if (name === "&") { return recv && other; }
    if (name === "|") { return recv || other; }
    return recv !== other; // "^"
  }

  // ── Ruby Numeric catalog (Integer / Float) ─────────────────────
  // Hand-implemented Ruby numeric methods that have no 1:1 JS-native
  // spelling (`gcd`, `digits`, `upto`/`downto`/`step`, …).  Dispatched by an
  // EXPLICIT `switch` on the source-derived `name` — never `recv[name]` — so
  // no reflective gadget is reachable; the receiver is a primitive number and
  // the native allowlist below still guards `toString`/`toFixed`.  A
  // non-numeric argument degrades to 0 via `numArg` (the lenient
  // never-raise-on-the-OO-surface floor), matching the Go/Rust/Python
  // reference runtimes.  `NUMERIC_METHODS` mirrors these case labels EXACTLY so
  // `respond_to?` stays honest as the catalog grows.
  const NUM_MISS = Symbol("num-miss");
  const NUMERIC_METHODS = new Set([
    "abs", "to_i", "to_int", "to_f", "even?", "odd?", "zero?", "positive?",
    "negative?", "succ", "next", "pred", "floor", "ceil", "round",
    "divmod", "fdiv", "clamp", "between?", "gcd",
    "pow", "**", "digits", "times", "upto", "downto", "step",
    // Tagged-float type predicates: distinguish Integer from Float now that
    // the runtime carries the tag.
    "integer?", "float?", "finite?", "nan?", "infinite?",
  ]);
  // Lenient numeric coercion: a non-number argument becomes 0 rather than
  // producing NaN (which would silently break `<=`/`>=` loop guards).
  // Unwrap a numeric method argument to a raw f64 (a boxed Float too),
  // defaulting a non-number to 0 (the lenient coercion Ruby's numeric
  // methods use for their integer arguments).
  function numArg(x) { return isNum(x) ? numOf(x) : 0; }
  // Ruby rounds half AWAY from zero (`2.5.round == 3`, `-2.5.round == -3`),
  // unlike JS `Math.round` (half toward +∞).
  function rubyRound(x) {
    return x >= 0 ? Math.floor(x + 0.5) : Math.ceil(x - 0.5);
  }
  function gcdInt(a, b) {
    a = Math.abs(Math.trunc(a));
    b = Math.abs(Math.trunc(b));
    while (b !== 0) { const t = a % b; a = b; b = t; }
    return a;
  }
  function numericMethod(recv, name, args) {
    // `recv` may be a native number (Integer, or a non-integral Float like
    // `3.5`) or a boxed integral Float (`7.0`).  `n` is the raw f64 for the
    // arithmetic; `rf` is "the receiver is a Ruby Float".  A result that Ruby
    // types as a Float is re-tagged through `mkFloat` (which boxes an integral
    // result, leaves a non-integral one native); an Integer result stays
    // native.  `floatIf(x)` centralises "Float iff the receiver is a Float".
    const n = numOf(recv);
    const rf = isFloat(recv);
    const floatIf = (x) => rf ? mkFloat(x) : x;
    switch (name) {
      // Type predicates (now that Integer and Float are distinguishable).
      case "integer?": return !rf;
      case "float?": return rf;
      case "finite?": return Number.isFinite(n);
      case "nan?": return Number.isNaN(n);
      case "infinite?": return n === Infinity ? 1 : (n === -Infinity ? -1 : null);
      case "abs": return floatIf(Math.abs(n));
      case "to_i": case "to_int": return Math.trunc(n);      // → Integer
      case "to_f": return mkFloat(n);                        // → Float (7 → 7.0)
      case "even?": return Math.trunc(n) % 2 === 0;
      case "odd?": return Math.abs(Math.trunc(n) % 2) === 1;
      case "zero?": return n === 0;
      case "positive?": return n > 0;
      case "negative?": return n < 0;
      case "succ": case "next": return floatIf(n + 1);
      case "pred": return floatIf(n - 1);
      case "floor": return Math.floor(n);                    // → Integer
      case "ceil": return Math.ceil(n);                      // → Integer
      case "round": {
        // Ruby `round` / `round(ndigits)` — half AWAY from zero (via `rubyRound`,
        // NOT `Math.round` which is half-toward-+∞).  Return TYPE: an Integer
        // receiver is always an Integer; a Float receiver is an Integer when
        // `ndigits <= 0` (`7.5.round == 8`) and a Float when `ndigits > 0`
        // (`7.0.round(2) == 7.0`).  A non-finite receiver returns unchanged
        // (tag preserved).  Hostile-magnitude `ndigits` degrades naturally —
        // `factor` saturates to `Infinity`, `n / Infinity` is `0`.
        const nd = isNum(args[0]) ? Math.trunc(numOf(args[0])) : 0;
        if (!Number.isFinite(n)) { return recv; }
        let result;
        if (Number.isInteger(n) && nd >= 0) { result = n; }
        else { const factor = Math.pow(10, nd); result = rubyRound(n * factor) / factor; }
        return (rf && nd > 0) ? mkFloat(result) : result;
      }
      case "divmod": {
        // Ruby `divmod(n)` → `[quotient, remainder]`, FLOORED quotient (always
        // Integer) and divisor-signed remainder (a Float iff either operand is
        // a Float: `7.0.divmod(2) == [3, 1.0]`).  Zero divisor raises.
        const d = numArg(args[0]);
        if (d === 0) { raiseError("ZeroDivisionError", "divided by 0"); }
        const q = Math.floor(n / d);
        const r = n - q * d;
        const remFloat = rf || isFloat(args[0]);
        return [q, remFloat ? mkFloat(r) : r];
      }
      case "fdiv": {
        // Ruby `fdiv(n)` — floating-point division that NEVER raises (a zero
        // divisor yields `Infinity`/`-Infinity`/`NaN`).  Always a Float.
        return mkFloat(n / numArg(args[0]));
      }
      case "clamp": {
        // Ruby `Comparable#clamp(min, max)`: `min` if recv < min, `max` if
        // recv > max, else recv.  Returns the ORIGINAL bound/receiver value so
        // its tag is preserved.  (The Range form is a follow-up.)
        if (n < numArg(args[0])) { return args[0]; }
        if (n > numArg(args[1])) { return args[1]; }
        return recv;
      }
      case "between?":
        return n >= numArg(args[0]) && n <= numArg(args[1]);
      case "gcd": return gcdInt(n, numArg(args[0]));         // → Integer
      case "pow": case "**": {
        // Float iff either the base or the exponent is a Float
        // (`2 ** 3 == 8` Integer; `2.0 ** 3 == 8.0` Float).
        const p = Math.pow(n, numArg(args[0]));
        return (rf || isFloat(args[0])) ? mkFloat(p) : p;
      }
      case "digits": {
        // Base-10 digits, least-significant first (`123.digits == [3, 2, 1]`).
        // A negative receiver is taken by magnitude (parity with the reference
        // runtimes).  Digits are Integers.
        let d = Math.abs(Math.trunc(n));
        const out = [];
        if (d === 0) { out.push(0); }
        while (d > 0) { out.push(d % 10); d = Math.trunc(d / 10); }
        return out;
      }
      case "times": {
        // Block arg arrives already unwrapped to a JS function; a block-less
        // call returns the receiver (v0 floor for Ruby's Enumerator).  Yields
        // Integer indices (`3.times` yields `0, 1, 2`).
        const blk = args[args.length - 1];
        if (typeof blk === "function") {
          const cnt = Math.trunc(n);
          for (let i = 0; i < cnt; i++) { blk(i); }
        }
        return recv;
      }
      case "upto": {
        const blk = args[args.length - 1];
        if (typeof blk === "function") {
          const hi = Math.trunc(numArg(args[0]));
          for (let i = Math.trunc(n); i <= hi; i++) { blk(i); }
        }
        return recv;
      }
      case "downto": {
        const blk = args[args.length - 1];
        if (typeof blk === "function") {
          const lo = Math.trunc(numArg(args[0]));
          for (let i = Math.trunc(n); i >= lo; i--) { blk(i); }
        }
        return recv;
      }
      case "step": {
        // `a.step(limit, stride=1) { |v| … }`.  A zero (or non-numeric → 0)
        // stride yields nothing rather than spinning forever — the never-hang
        // floor.  Yielded values are Floats iff the receiver or stride is a
        // Float (`1.0.step(2.0, 0.5)` yields Floats).
        const blk = args[args.length - 1];
        if (typeof blk === "function") {
          const limit = numArg(args[0]);
          const stride = args.length >= 3 ? numArg(args[1]) : 1;
          const yieldFloat = rf || (args.length >= 3 && isFloat(args[1]));
          const emit = (v) => blk(yieldFloat ? mkFloat(v) : v);
          if (stride > 0) { for (let v = n; v <= limit; v += stride) { emit(v); } }
          else if (stride < 0) { for (let v = n; v >= limit; v += stride) { emit(v); } }
        }
        return recv;
      }
    }
    return NUM_MISS;
  }

  // ── Ruby String catalog ────────────────────────────────────────
  // Hand-implemented Ruby String methods that either have no JS-native
  // equivalent (`capitalize`, `chomp`, `chars`, `bytes`, `to_sym`, …) or whose
  // Ruby semantics DIVERGE from the native (Ruby `sub`/`gsub` are LITERAL
  // first/all replacement with no regex or back-reference expansion; Ruby
  // `index` is a rune index; Ruby `reverse` is rune-aware and, unlike arrays,
  // has no JS-native String method at all).  Dispatched by an EXPLICIT `switch`
  // on the source-derived `name` — never `recv[name]` — ahead of the native
  // allowlist, so the already-aliased natives (`upcase`→`toUpperCase`, …) still
  // fall through on `STR_MISS`.  `STRING_METHODS` mirrors these labels for an
  // honest `respond_to?`.
  const STR_MISS = Symbol("str-miss");
  const STRING_METHODS = new Set([
    "capitalize", "chomp", "chars", "bytes", "sub", "gsub", "to_i", "to_f",
    "to_sym", "to_s", "empty?", "index", "reverse", "size",
    "ljust", "rjust", "center", "swapcase",
    "tr", "count", "delete", "squeeze",
  ]);
  // Ruby `String#to_i` / `#to_f`: parse a LEADING numeric prefix (optional
  // sign, digits, and — for to_f — a fractional/exponent part), yielding 0 when
  // there is no numeric prefix.  Never raises (the lenient OO floor).
  function strToI(s) {
    const m = /^[+-]?\d+/.exec(s.trimStart());
    return m ? Math.trunc(Number(m[0])) : 0;
  }
  function strToF(s) {
    const m = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?/.exec(s.trimStart());
    return m ? Number(m[0]) : 0;
  }
  function stringMethod(recv, name, args) {
    switch (name) {
      case "to_s": return recv;
      case "empty?": return recv.length === 0;
      case "size": return [...recv].length; // rune count, mirrors Ruby length
      case "reverse": return [...recv].reverse().join("");
      case "chars": return [...recv];
      case "bytes": {
        // Raw UTF-8 byte values as integers (Ruby `String#bytes`).
        const enc = new TextEncoder().encode(recv);
        return Array.from(enc, (b) => b);
      }
      case "to_i": return strToI(recv);
      case "to_f": return strToF(recv);
      case "to_sym": return new Sym(recv);
      case "capitalize": {
        // First character upcased, the rest downcased; rune-aware so a leading
        // multibyte char is not split.
        const cps = [...recv];
        if (cps.length === 0) { return ""; }
        return cps[0].toUpperCase() + cps.slice(1).join("").toLowerCase();
      }
      case "chomp": {
        // With an explicit separator, drop exactly that trailing suffix once;
        // otherwise drop one trailing "\r\n", "\n", or "\r".
        if (args.length > 0) {
          const sep = args[0];
          if (typeof sep === "string") {
            return sep !== "" && recv.endsWith(sep)
              ? recv.slice(0, recv.length - sep.length) : recv;
          }
          return recv;
        }
        if (recv.endsWith("\r\n")) { return recv.slice(0, -2); }
        if (recv.endsWith("\n") || recv.endsWith("\r")) { return recv.slice(0, -1); }
        return recv;
      }
      case "index": {
        // Rune index of the first occurrence of the substring, or nil.
        if (args.length > 0 && typeof args[0] === "string") {
          const bi = recv.indexOf(args[0]);
          if (bi < 0) { return null; }
          return [...recv.slice(0, bi)].length; // byte offset → rune count
        }
        return null;
      }
      case "sub": {
        // LITERAL replacement of the FIRST occurrence (no regex / no `$&`/`\1`).
        if (args.length >= 2 && typeof args[0] === "string" && typeof args[1] === "string") {
          const bi = recv.indexOf(args[0]);
          if (bi < 0) { return recv; }
          return recv.slice(0, bi) + args[1] + recv.slice(bi + args[0].length);
        }
        return recv;
      }
      case "gsub": {
        // LITERAL replacement of ALL occurrences.  `split(from).join(to)` is
        // verbatim (no regex), matching Ruby's string-argument `gsub`.
        if (args.length >= 2 && typeof args[0] === "string" && typeof args[1] === "string") {
          if (args[0] === "") { return recv; }
          return recv.split(args[0]).join(args[1]);
        }
        return recv;
      }
      case "ljust":
      case "rjust":
      case "center": {
        // Ruby String#ljust/#rjust/#center(width, pad = " "): pad to `width`
        // RUNES using `pad` cyclically; `width <= current rune length` returns
        // the string unchanged; `center` puts an odd extra pad rune on the
        // RIGHT.  An empty pad degrades to a single space (never-raise floor).
        const width = args.length > 0 ? Math.trunc(numArg(args[0])) : 0;
        let pad = " ";
        if (args.length > 1 && typeof args[1] === "string" && args[1] !== "") {
          pad = args[1];
        }
        const cps = [...recv];
        if (width <= cps.length) { return recv; }
        const total = width - cps.length;
        const pr = [...pad];
        const buildPad = (n) => {
          let out = "";
          for (let i = 0; i < n; i++) { out += pr[i % pr.length]; }
          return out;
        };
        if (name === "ljust") { return recv + buildPad(total); }
        if (name === "rjust") { return buildPad(total) + recv; }
        const left = Math.floor(total / 2);
        return buildPad(left) + recv + buildPad(total - left);
      }
      case "swapcase": {
        // Flip the case of each ASCII letter (leaving non-letters and non-ASCII
        // code points untouched).  Iterating the string yields whole runes.
        let out = "";
        for (const ch of recv) {
          const c = ch.codePointAt(0);
          if (c >= 65 && c <= 90) { out += String.fromCodePoint(c + 32); }
          else if (c >= 97 && c <= 122) { out += String.fromCodePoint(c - 32); }
          else { out += ch; }
        }
        return out;
      }
      case "tr": {
        // Ruby `String#tr(from, to)`: position-wise code-point translation.  A
        // shorter `to` repeats its last code point; an empty `to` deletes
        // matching code points; a repeated code point in `from` keeps the last
        // mapping.  Iterates by code point (`[...str]`/`for..of`) so a multibyte
        // receiver is never split.  Literal only — the range (`"a-z"`) and
        // negation (`"^abc"`) forms are a follow-up, matching the literal-only
        // sub/gsub precedent here.
        const from = typeof args[0] === "string" ? args[0] : null;
        const to = typeof args[1] === "string" ? args[1] : null;
        if (from === null || to === null) { return recv; }
        const toC = [...to];
        const table = new Map();
        const fromC = [...from];
        for (let i = 0; i < fromC.length; i++) {
          if (toC.length === 0) { table.set(fromC[i], null); }
          else { table.set(fromC[i], i < toC.length ? toC[i] : toC[toC.length - 1]); }
        }
        let out = "";
        for (const ch of recv) {
          if (table.has(ch)) {
            const r = table.get(ch);
            if (r !== null) { out += r; }
          } else { out += ch; }
        }
        return out;
      }
      case "count": case "delete": case "squeeze": {
        // Char-set methods.  Each `set` argument is treated LITERALLY — the code
        // points it contains (ranges/negation are a follow-up).  `count` tallies
        // code points of the receiver in the set; `delete` removes them;
        // `squeeze` collapses consecutive runs (of set code points, or of ALL
        // when no set is given).  Multiple set args intersect (Ruby's rule).
        const sets = [];
        for (const a of args) {
          if (typeof a === "string") { sets.push(new Set([...a])); }
        }
        const inAll = (ch) => sets.length > 0 && sets.every((set) => set.has(ch));
        if (name === "squeeze" && sets.length === 0) {
          let out = "";
          let last = null;
          for (const ch of recv) { if (ch !== last) { out += ch; last = ch; } }
          return out;
        }
        if (name === "count") {
          let n = 0;
          for (const ch of recv) { if (inAll(ch)) { n++; } }
          return n;
        }
        if (name === "delete") {
          let out = "";
          for (const ch of recv) { if (!inAll(ch)) { out += ch; } }
          return out;
        }
        let out = "";
        let last = null;
        for (const ch of recv) {
          if (ch === last && inAll(ch)) { continue; }
          out += ch;
          last = ch;
        }
        return out;
      }
    }
    return STR_MISS;
  }

  // ── Ruby Hash catalog (`Map` receiver) ─────────────────────────
  // A Ruby Hash is a JS `Map` (insertion-ordered).  Hand-implemented by an
  // EXPLICIT `switch` on the source-derived `name` (never `recv[name]`) ahead of
  // the native allowlist — Ruby's `keys`/`values`/`to_a` must return real
  // Arrays (native `Map.keys()` yields a lazy iterator, not a Ruby Array), and
  // `each`/`merge`/`dig`/`invert`/… have no faithful native equivalent.  Value
  // comparison (`has_value?`/`invert`) uses `===`: exact for primitives,
  // strings, and interned symbols — the v0 floor (deep-equal of nested values
  // is a follow-up).  `HASH_METHODS` mirrors these labels for `respond_to?`.
  const HASH_MISS = Symbol("hash-miss");
  const HASH_METHODS = new Set([
    "keys", "values", "size", "length", "empty?", "has_key?", "key?",
    "include?", "member?", "has_value?", "value?", "to_a", "merge", "dig",
    "invert", "delete", "store", "[]=", "fetch", "clear", "each", "each_pair",
    "map", "select", "filter", "reject", "transform_values", "transform_keys",
    "find", "detect", "any?", "all?", "none?", "count",
    "sort_by", "min_by", "max_by",
    "group_by", "partition", "flat_map", "collect_concat",
    "reduce", "inject", "sum",
    "to_h", "each_with_index", "each_with_object",
  ]);
  function hashMethod(recv, name, args) {
    switch (name) {
      case "keys": return [...recv.keys()];
      case "values": return [...recv.values()];
      case "size": case "length": return recv.size;
      case "empty?": return recv.size === 0;
      case "has_key?": case "key?": case "include?": case "member?":
        return recv.has(args[0]);
      case "has_value?": case "value?": {
        for (const v of recv.values()) { if (v === args[0]) { return true; } }
        return false;
      }
      case "to_a": {
        // Array of `[key, value]` two-element Arrays, in insertion order.
        const out = [];
        for (const [k, v] of recv) { out.push([k, v]); }
        return out;
      }
      case "merge": {
        // Non-mutating: a fresh Map with `other`'s entries overlaid (last wins).
        const out = new Map(recv);
        if (args[0] instanceof Map) { for (const [k, v] of args[0]) { out.set(k, v); } }
        return out;
      }
      case "dig": {
        // Walk one key per argument, nil the moment a level is missing; a
        // non-diggable intermediate stops the walk.
        let cur = recv;
        for (const k of args) {
          if (cur instanceof Map) { cur = cur.has(k) ? cur.get(k) : null; }
          else if (Array.isArray(cur) && isNum(k)) { cur = cur[numOf(k)] ?? null; }
          else { return null; }
          if (cur === null) { return null; }
        }
        return cur;
      }
      case "invert": {
        const out = new Map();
        for (const [k, v] of recv) { out.set(v, k); }
        return out;
      }
      case "delete": {
        // Ruby `delete` MUTATES and returns the removed value (nil if absent).
        if (recv.has(args[0])) { const v = recv.get(args[0]); recv.delete(args[0]); return v; }
        return null;
      }
      case "store": case "[]=": {
        // Ruby `store(k, v)` (alias `[]=`): mutates, returns the value.
        recv.set(args[0], args[1]);
        return args[1];
      }
      case "fetch": {
        // Ruby `Hash#fetch(k)`: returns the value for `k` if present; a MISSING
        // key with no default raises `KeyError` (unlike `hash[k]`, which returns
        // nil).  A second argument supplies a default returned instead of
        // raising.  Typed `SirError` so a translated `rescue KeyError` catches it.
        // (The block form is out of v0 scope.)
        if (recv.has(args[0])) { return recv.get(args[0]); }
        if (args.length > 1) { return args[1]; }
        raiseError("KeyError", "key not found: " + format(args[0]));
        return null; // unreachable — raiseError throws
      }
      case "clear": {
        // Ruby `Hash#clear`: MUTATES, removing every pair, and returns the
        // (now-empty) receiver.
        recv.clear();
        return recv;
      }
      case "each": case "each_pair": {
        // Yields (key, value); returns the receiver.
        const blk = args[args.length - 1];
        if (typeof blk === "function") { for (const [k, v] of recv) { blk(k, v); } }
        return recv;
      }
      case "map": {
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return []; }
        const out = [];
        for (const [k, v] of recv) { out.push(blk(k, v)); }
        return out;
      }
      case "select": case "filter": case "reject": {
        // Ruby `select`/`reject` return a new Hash of the kept pairs.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return new Map(recv); }
        const keepWhenTruthy = name !== "reject";
        const out = new Map();
        for (const [k, v] of recv) {
          const t = truthy(blk(k, v));
          if (keepWhenTruthy ? t : !t) { out.set(k, v); }
        }
        return out;
      }
      case "transform_values": {
        // Ruby `Hash#transform_values { |v| … }`: a NEW hash whose keys are
        // copied verbatim and whose values are the block results.  The block
        // yields ONE argument (the value); keys stay untouched (and unique, so
        // no collision) and insertion order is preserved.  Non-mutating.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return new Map(recv); }
        const out = new Map();
        for (const [k, v] of recv) { out.set(k, blk(v)); }
        return out;
      }
      case "transform_keys": {
        // Ruby `Hash#transform_keys { |k| … }`: a NEW hash whose values are
        // untouched and whose keys are the block results (yields ONE argument,
        // the key).  Two source keys can map to the SAME new key; Ruby keeps the
        // LAST such entry's value at the FIRST-seen position — which is exactly
        // how `Map.set` behaves on an existing key (updates value, keeps slot).
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return new Map(recv); }
        const out = new Map();
        for (const [k, v] of recv) { out.set(blk(k), v); }
        return out;
      }
      // ── Enumerable aggregates (Hash includes Enumerable) ─────────
      //
      // Ruby's Hash mixes in Enumerable, so these iterate the hash as a
      // sequence of [key, value] pairs: the block is yielded (key, value)
      // (two arguments, matching `each`), and the "element" an aggregate
      // returns is the two-element [key, value] Array.
      case "find": case "detect": {
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        for (const [k, v] of recv) { if (truthy(blk(k, v))) { return [k, v]; } }
        return null;
      }
      case "any?": {
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return recv.size > 0; }
        for (const [k, v] of recv) { if (truthy(blk(k, v))) { return true; } }
        return false;
      }
      case "all?": {
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return true; }
        for (const [k, v] of recv) { if (!truthy(blk(k, v))) { return false; } }
        return true;
      }
      case "none?": {
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return recv.size === 0; }
        for (const [k, v] of recv) { if (truthy(blk(k, v))) { return false; } }
        return true;
      }
      case "count": {
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return recv.size; }
        let n = 0;
        for (const [k, v] of recv) { if (truthy(blk(k, v))) { n++; } }
        return n;
      }
      case "sort_by": {
        // A NEW Array of [k, v] pairs sorted by the block key (`arrCmp` is the
        // never-throw numeric-aware comparator used by Array#sort_by).
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        const keyed = [];
        for (const [k, v] of recv) { keyed.push([blk(k, v), [k, v]]); }
        keyed.sort((a, b) => arrCmp(a[0], b[0]));
        return keyed.map((p) => p[1]);
      }
      case "min_by": case "max_by": {
        // The [k, v] pair with the extremal block key (first-on-tie; nil on
        // an empty hash).
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        if (recv.size === 0) { return null; }
        const wantMin = name === "min_by";
        let bestPair = null;
        let bestKey;
        for (const [k, v] of recv) {
          const key = blk(k, v);
          if (bestPair === null || (wantMin ? key < bestKey : key > bestKey)) {
            bestPair = [k, v];
            bestKey = key;
          }
        }
        return bestPair;
      }
      // ── Enumerable breadth (block-taking reshape / fold) ─────────
      //
      // Same [key, value]-pair iteration as the aggregates above: every method
      // yields (key, value) EXCEPT `reduce`/`inject`, which follow Ruby's memo
      // convention and yield (memo, pair) — the [k, v] Array as ONE argument.
      case "group_by": {
        // A Map from each block key to the Array of the [k, v] pairs that
        // produced it, in first-seen key order (mirrors Array#group_by, which
        // also returns a Map).
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        const groups = new Map();
        for (const [k, v] of recv) {
          const gk = blk(k, v);
          const bucket = groups.get(gk);
          if (bucket) { bucket.push([k, v]); } else { groups.set(gk, [[k, v]]); }
        }
        return groups;
      }
      case "partition": {
        // [[matching pairs], [rest pairs]] — each a fresh Array of [k, v] pairs.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        const yes = [];
        const no = [];
        for (const [k, v] of recv) {
          if (truthy(blk(k, v))) { yes.push([k, v]); } else { no.push([k, v]); }
        }
        return [yes, no];
      }
      case "flat_map": case "collect_concat": {
        // Map each pair then concatenate one level: an Array result splices its
        // elements, a scalar is appended as-is.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        const out = [];
        for (const [k, v] of recv) {
          const r = blk(k, v);
          if (Array.isArray(r)) { out.push(...r); } else { out.push(r); }
        }
        return out;
      }
      case "reduce": case "inject": {
        // Ruby's memo fold.  Unlike every other method here (which yields the
        // two-arg pair), `reduce` yields (memo, pair) — the [k, v] Array as ONE
        // argument.  `reduce(seed) { … }` seeds from the arg; seedless seeds
        // from the first pair; an empty seedless reduce is nil.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        const pairs = [...recv]; // Map iteration yields [k, v] Arrays
        let acc;
        let start;
        if (args.length >= 2) { acc = args[0]; start = 0; }
        else if (pairs.length > 0) { acc = pairs[0]; start = 1; }
        else { return null; }
        for (let i = start; i < pairs.length; i++) { acc = blk(acc, pairs[i]); }
        return acc;
      }
      case "sum": {
        // Numeric fold seeded at 0 (or the explicit seed arg) over the block
        // results — the same native `+` accumulation Array#sum uses, so
        // integer inputs stay integers and any float promotes.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        let acc = args.length >= 2 ? args[0] : 0;
        for (const [k, v] of recv) { acc = acc + blk(k, v); }
        return acc;
      }
      case "to_h": {
        // WITHOUT a block, a shallow copy of the hash (a fresh `Map`, so
        // mutating it never aliases the receiver).  WITH a block
        // `{ |k, v| [new_k, new_v] }`, a NEW hash from the `[k, v]` pairs the
        // block returns: the block is yielded the two args `(k, v)` (matching
        // `each`), a non-pair result is skipped (Ruby raises TypeError,
        // deferred to the typed-error cascade), and a later pair with a
        // duplicate key wins (Ruby's rule, and how `Map.set` behaves).
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return new Map(recv); }
        const out = new Map();
        for (const [k, v] of recv) {
          const pair = blk(k, v);
          if (Array.isArray(pair) && pair.length === 2) { out.set(pair[0], pair[1]); }
        }
        return out;
      }
      case "each_with_index": {
        // Yields each `[k, v]` pair with its 0-based index and returns the
        // receiver.  Unlike the two-arg `(k, v)` yield of `each`, the element
        // arrives as a single `[k, v]` Array (the second block param is the
        // index), matching Ruby's Enumerable convention.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        let i = 0;
        for (const [k, v] of recv) { blk([k, v], i); i++; }
        return recv;
      }
      case "each_with_object": {
        // `each_with_object(memo) { |(k, v), memo| … }` — yields each `[k, v]`
        // pair with the memo object and returns the (mutated) memo.  Like
        // `each_with_index`, the element is the single `[k, v]` pair (the second
        // block param is the memo).  With no memo argument the receiver is
        // returned unchanged.
        const blk = args[args.length - 1];
        if (typeof blk !== "function") { return HASH_MISS; }
        if (args.length < 2) { return recv; }
        const memo = args[0];
        for (const [k, v] of recv) { blk([k, v], memo); }
        return memo;
      }
    }
    return HASH_MISS;
  }

  // ── Ruby Symbol catalog (`Sym` receiver) ───────────────────────
  // Hand-implemented Ruby Symbol methods, dispatched by an EXPLICIT `switch` on
  // the source-derived `name` (never `recv[name]`) ahead of the native
  // allowlist.  Ruby's case methods (`upcase`/`downcase`/`capitalize`) return a
  // new SYMBOL (`:foo.upcase == :FOO`), not a string; `to_s` returns the name
  // string; `inspect` is the `:`-prefixed form.  `SYMBOL_METHODS` mirrors these
  // labels for `respond_to?`.
  const SYM_MISS = Symbol("sym-miss");
  const SYMBOL_METHODS = new Set([
    "to_s", "to_sym", "to_proc", "length", "size", "empty?", "upcase",
    "downcase", "capitalize", "inspect",
  ]);
  function symbolMethod(recv, name, args) {
    switch (name) {
      case "to_s": return recv.name;
      case "to_sym": return recv;
      case "inspect": return ":" + recv.name;
      case "length": case "size": return [...recv.name].length;
      case "empty?": return recv.name.length === 0;
      case "upcase": return new Sym(recv.name.toUpperCase());
      case "downcase": return new Sym(recv.name.toLowerCase());
      case "capitalize": {
        const cps = [...recv.name];
        if (cps.length === 0) { return new Sym(""); }
        return new Sym(cps[0].toUpperCase() + cps.slice(1).join("").toLowerCase());
      }
      case "to_proc": {
        // `:m.to_proc` → a Closure that dispatches `.m(rest…)` on its first
        // argument.  The dynamic method name (`recv.name`) routes back through
        // `callMethod` — the SAME allowlist / method-table gate a direct call
        // uses — never `recv[name]` / reflection (the C3 RCE discipline).
        const method = recv.name;
        return new Closure((...a) => {
          if (a.length === 0) { return null; }
          return callMethod(a[0], method, ...a.slice(1));
        });
      }
    }
    return SYM_MISS;
  }

  // ── Ruby Array / Enumerable catalog (Array receiver) ───────────
  // A Ruby Array is a JS Array.  Hand-implemented by an EXPLICIT `switch` on
  // the source-derived `name` (never `recv[name]`) so the Ruby-named methods
  // (`select`/`reject`/`detect`/`inject`/`any?`/…) and the semantics-diverging
  // ones (numeric `sort` — native JS `sort` is lexicographic; `min`/`max`;
  // `sort_by`/`group_by`/`partition`/…) resolve.  A block arrives already
  // unwrapped to a JS function as the trailing positional arg.  `ARR_MISS`
  // falls through so native mutators/accessors keep working.  `ARRAY_METHODS`
  // mirrors these labels for `respond_to?`.
  const ARR_MISS = Symbol("arr-miss");
  const ARRAY_METHODS = new Set([
    "each", "each_with_index", "map", "collect", "select", "filter", "reject",
    "find", "detect", "reduce", "inject", "any?", "all?", "none?", "count",
    "sort", "sort_by", "min", "max", "minmax", "min_by", "max_by", "group_by",
    "partition", "flat_map", "collect_concat", "take_while", "drop_while",
    "each_with_object", "sum", "uniq", "first", "last", "empty?", "to_a",
    "take", "drop", "values_at",
    "flatten", "compact", "rotate", "zip",
    "include?", "index",
    "each_slice", "each_cons", "chunk_while", "slice_when", "tally", "cycle",
  ]);
  // Numeric-aware comparator (`<`/`>` keeps numbers numeric, never throws) —
  // the same ordering the Ruby `sort` reference uses.
  function arrCmp(a, b) { return a < b ? -1 : a > b ? 1 : 0; }
  // Ruby value equality (`==`) for `Array#include?` / `Array#index`.  Ruby
  // compares by VALUE, not identity: `[[1,2]].include?([1, 2])` is `true` and
  // `[1,2,3].index(9)` is `nil`.  Native JS `Array#includes` / `indexOf` use
  // SameValueZero (identity for objects), so a nested Array or Symbol would
  // wrongly miss — this mirrors the Go/Python reference (`_sir_value_eq`):
  // scalars by `===`, Symbols by name, Arrays element-wise, Maps entry-wise.
  // (`NaN !== NaN`, matching Ruby's `Float::NAN == Float::NAN == false`.)
  // `seen` is a path-set of the `a`-side containers currently being compared; a
  // cyclic array (`a = []; a << a`, which this runtime explicitly supports) would
  // otherwise recurse forever.  Re-encountering `a` on the path means the two
  // structures have matched identically all the way down to the back-edge, so we
  // return `true` — exactly Ruby's recursive-`==` rule — mirroring the `seen` set
  // the display path (`formatSeen`) carries for the same cyclic values.
  function valEq(a, b, seen) {
    if (a === b) { return true; }
    // Ruby `==` on numbers is by VALUE across Integer/Float: `7.0 == 7` is
    // true, so `[7.0].include?(7)`. Compare unwrapped payloads (a boxed
    // `7.0` and native `7` are `===`-distinct objects/values but `==`
    // equal). NaN stays `== NaN` false, matching Ruby. Hash keys use `eql?`
    // (identity here), NOT this, so Integer `7` and Float `7.0` remain
    // distinct KEYS while being `==` equal — exactly Ruby's split.
    if (isNum(a) && isNum(b)) { return numOf(a) === numOf(b); }
    if (a instanceof Sym && b instanceof Sym) { return a.name === b.name; }
    if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) { return false; }
      if (seen === undefined) { seen = new Set(); }
      if (seen.has(a)) { return true; }
      seen.add(a);
      for (let i = 0; i < a.length; i++) {
        if (!valEq(a[i], b[i], seen)) { seen.delete(a); return false; }
      }
      seen.delete(a);
      return true;
    }
    if (a instanceof Map && b instanceof Map) {
      if (a.size !== b.size) { return false; }
      if (seen === undefined) { seen = new Set(); }
      if (seen.has(a)) { return true; }
      seen.add(a);
      for (const [k, v] of a) {
        if (!b.has(k) || !valEq(b.get(k), v, seen)) { seen.delete(a); return false; }
      }
      seen.delete(a);
      return true;
    }
    return false;
  }
  function arrayMethod(recv, name, args) {
    // A trailing function positional is the block (`arr.map { … }`).
    const blk = args.length > 0 && typeof args[args.length - 1] === "function"
      ? args[args.length - 1]
      : null;
    switch (name) {
      case "each":
        if (blk) { for (const x of recv) { blk(x); } }
        return recv;
      case "each_with_index":
        if (blk) { recv.forEach((x, i) => blk(x, i)); }
        return recv;
      case "map": case "collect":
        return blk ? recv.map((x) => blk(x)) : ARR_MISS;
      case "select": case "filter":
        return blk ? recv.filter((x) => truthy(blk(x))) : ARR_MISS;
      case "reject":
        return blk ? recv.filter((x) => !truthy(blk(x))) : ARR_MISS;
      case "find": case "detect": {
        if (!blk) { return ARR_MISS; }
        for (const x of recv) { if (truthy(blk(x))) { return x; } }
        return null;
      }
      case "reduce": case "inject": {
        if (!blk) { return ARR_MISS; }
        // `reduce(seed) { … }` (args = [seed, block]) or `reduce { … }`.
        let acc;
        let start;
        if (args.length >= 2) { acc = args[0]; start = 0; }
        else if (recv.length > 0) { acc = recv[0]; start = 1; }
        else { return null; }
        for (let i = start; i < recv.length; i++) { acc = blk(acc, recv[i]); }
        return acc;
      }
      case "any?": return blk ? recv.some((x) => truthy(blk(x))) : recv.some(truthy);
      case "all?": return blk ? recv.every((x) => truthy(blk(x))) : recv.every(truthy);
      case "none?": return blk ? !recv.some((x) => truthy(blk(x))) : !recv.some(truthy);
      case "count":
        if (blk) { return recv.reduce((n, x) => (truthy(blk(x)) ? n + 1 : n), 0); }
        if (args.length > 0) { return recv.filter((x) => x === args[0]).length; }
        return recv.length;
      case "sort": return [...recv].sort(arrCmp);
      case "sort_by": {
        if (!blk) { return ARR_MISS; }
        const keyed = recv.map((x) => [blk(x), x]);
        keyed.sort((a, b) => arrCmp(a[0], b[0]));
        return keyed.map((p) => p[1]);
      }
      case "min": return recv.length ? recv.reduce((a, b) => (b < a ? b : a)) : null;
      case "max": return recv.length ? recv.reduce((a, b) => (b > a ? b : a)) : null;
      case "minmax": {
        // `minmax` (no block) — the two-element array `[min, max]` in one pass,
        // via `<`/`>` (the same comparison the `min`/`max` arms use).
        // `[3,1,2].minmax` → `[1, 3]`.  An empty array yields `[null, null]`
        // (Ruby `[nil, nil]` — no smallest/largest element), matching the
        // Go/Rust/Python references' 2-element nil array.
        if (recv.length === 0) { return [null, null]; }
        let lo = recv[0];
        let hi = recv[0];
        for (let i = 1; i < recv.length; i++) {
          if (recv[i] < lo) { lo = recv[i]; }
          if (recv[i] > hi) { hi = recv[i]; }
        }
        return [lo, hi];
      }
      case "min_by": case "max_by": {
        if (!blk) { return ARR_MISS; }
        if (recv.length === 0) { return null; }
        const wantMin = name === "min_by";
        let bestItem = recv[0];
        let bestKey = blk(recv[0]);
        for (let i = 1; i < recv.length; i++) {
          const k = blk(recv[i]);
          if (wantMin ? k < bestKey : k > bestKey) { bestItem = recv[i]; bestKey = k; }
        }
        return bestItem;
      }
      case "group_by": {
        if (!blk) { return ARR_MISS; }
        const groups = new Map();
        for (const x of recv) {
          const k = blk(x);
          const bucket = groups.get(k);
          if (bucket) { bucket.push(x); } else { groups.set(k, [x]); }
        }
        return groups;
      }
      case "partition": {
        if (!blk) { return ARR_MISS; }
        const yes = [];
        const no = [];
        for (const x of recv) { if (truthy(blk(x))) { yes.push(x); } else { no.push(x); } }
        return [yes, no];
      }
      case "flat_map": case "collect_concat": {
        if (!blk) { return ARR_MISS; }
        const out = [];
        for (const x of recv) {
          const r = blk(x);
          if (Array.isArray(r)) { out.push(...r); } else { out.push(r); }
        }
        return out;
      }
      case "take_while": {
        if (!blk) { return ARR_MISS; }
        const out = [];
        for (const x of recv) { if (truthy(blk(x))) { out.push(x); } else { break; } }
        return out;
      }
      case "drop_while": {
        if (!blk) { return ARR_MISS; }
        const out = [];
        let dropping = true;
        for (const x of recv) {
          if (dropping && truthy(blk(x))) { continue; }
          dropping = false;
          out.push(x);
        }
        return out;
      }
      case "each_with_object": {
        if (!blk) { return ARR_MISS; }
        if (args.length < 2) { return recv; } // no memo supplied
        const memo = args[0];
        for (const x of recv) { blk(x, memo); }
        return memo;
      }
      case "sum": {
        let acc = args.length > 0 && typeof args[0] !== "function" ? args[0] : 0;
        for (const x of recv) { acc = acc + (blk ? blk(x) : x); }
        return acc;
      }
      case "uniq": {
        const out = [];
        const seen = new Set();
        for (const x of recv) { if (!seen.has(x)) { seen.add(x); out.push(x); } }
        return out;
      }
      case "first":
        return args.length > 0 ? recv.slice(0, args[0]) : (recv.length ? recv[0] : null);
      case "last":
        return args.length > 0
          ? recv.slice(Math.max(0, recv.length - args[0]))
          : (recv.length ? recv[recv.length - 1] : null);
      case "empty?": return recv.length === 0;
      case "take": case "drop": {
        // `take(n)` / `drop(n)` — the first / all-but-first `n` elements.  `n`
        // is clamped to `[0, len]` (`n <= 0` -> 0, `n > len` -> len).  Ruby
        // raises `ArgumentError` on a negative `n`; the never-raise floor
        // treats it as 0.  `recv.slice` never throws for in-range bounds.
        let n = typeof args[0] === "number" ? Math.trunc(args[0]) : 0;
        if (n < 0) { n = 0; }
        if (n > recv.length) { n = recv.length; }
        return name === "take" ? recv.slice(0, n) : recv.slice(n);
      }
      case "values_at": {
        // `values_at(*idxs)` — the element at each index, folding a negative
        // index from the end once; an out-of-range index yields `null`.
        const out = [];
        for (const a of args) {
          let idx = isNum(a) ? Math.trunc(numOf(a)) : 0;
          if (idx < 0) { idx += recv.length; }
          out.push(idx >= 0 && idx < recv.length ? recv[idx] : null);
        }
        return out;
      }
      case "flatten": {
        // Ruby `flatten` fully flattens nested Arrays; `flatten(n)` flattens to
        // depth `n` (a negative `n` means no limit).  Only Array elements are
        // flattened — strings and other values stay intact — matching Ruby and
        // the sibling backends.  (`Array#flat` is deliberately handled here, not
        // via the native alias, so the no-arg case is full-depth, not depth 1.)
        let depth = typeof args[0] === "number" ? Math.trunc(args[0]) : Infinity;
        if (depth < 0) { depth = Infinity; }
        return recv.flat(depth);
      }
      case "compact": {
        // Ruby `compact` returns a copy with every `nil` (`null`) removed.
        return recv.filter((x) => x !== null && x !== undefined);
      }
      case "rotate": {
        // `a.rotate(n=1)` — elements rotated left by `n` (a negative `n` rotates
        // right).  The modulo wraps so any magnitude terminates; an empty array
        // is `[]`.  No arg defaults to 1; a non-numeric arg degrades to 0.
        const length = recv.length;
        if (length === 0) { return []; }
        const n = args.length === 0
          ? 1
          : (typeof args[0] === "number" ? Math.trunc(args[0]) : 0);
        const shift = ((n % length) + length) % length;
        return recv.slice(shift).concat(recv.slice(0, shift));
      }
      case "zip": {
        // `a.zip(b, c, ...)` — an Array of tuples `[a[i], b[i], ...]` of length
        // `a.length`.  A shorter operand pads with `null`; a non-array operand
        // is treated as empty (pad-only), never raising.
        const others = args.map((o) => (Array.isArray(o) ? o : []));
        const zipped = [];
        for (let i = 0; i < recv.length; i++) {
          const row = [recv[i]];
          for (const o of others) { row.push(i < o.length ? o[i] : null); }
          zipped.push(row);
        }
        return zipped;
      }
      case "include?": {
        // Ruby `Array#include?(x)` — VALUE equality (see `valEq`), so a nested
        // Array/Symbol matches structurally.  Overrides the native `includes`
        // alias (SameValueZero), matching the Go/Python reference.
        for (const x of recv) { if (valEq(x, args[0])) { return true; } }
        return false;
      }
      case "index": {
        // Ruby `Array#index(x)` — the first index whose element `== x` (value
        // equality), or `nil` when absent.  Native JS `indexOf` returns `-1`
        // and uses identity; this returns `null` and matches Ruby / the Go
        // reference.  (The block form `index { … }` is out of v0 scope.)
        for (let i = 0; i < recv.length; i++) {
          if (valEq(recv[i], args[0])) { return i; }
        }
        return null;
      }
      case "to_a": return recv;
      case "each_slice": {
        // `each_slice(n)` — consecutive sub-arrays of at most `n` elements (the
        // last may be shorter).  `[1,2,3,4,5].each_slice(2)` → [[1,2],[3,4],[5]].
        // Ruby raises ArgumentError for n <= 0; the never-throw floor yields [].
        const n = args.length > 0 && isNum(args[0]) && Number.isInteger(numOf(args[0])) ? numOf(args[0]) : 0;
        if (n <= 0) { return []; }
        const out = [];
        for (let i = 0; i < recv.length; i += n) { out.push(recv.slice(i, i + n)); }
        return out;
      }
      case "each_cons": {
        // `each_cons(n)` — every consecutive n-element sliding window.
        // `[1,2,3,4].each_cons(2)` → [[1,2],[2,3],[3,4]].  A window larger than
        // the array (or n <= 0) yields [].
        const n = args.length > 0 && isNum(args[0]) && Number.isInteger(numOf(args[0])) ? numOf(args[0]) : 0;
        if (n <= 0) { return []; }
        const out = [];
        for (let i = 0; i + n <= recv.length; i++) { out.push(recv.slice(i, i + n)); }
        return out;
      }
      case "chunk_while": {
        // `chunk_while { |prev, cur| pred }` — runs of consecutive elements: the
        // block is called on each ADJACENT pair; while it is truthy the run
        // continues, and a falsy result starts a new run.
        // `[1,2,4,5,7].chunk_while { |a,b| b-a==1 }` → [[1,2],[4,5],[7]].
        // An empty array yields []; a single element yields [[x]].
        if (typeof blk !== "function") { return ARR_MISS; }
        if (recv.length === 0) { return []; }
        const chunks = [[recv[0]]];
        for (let i = 1; i < recv.length; i++) {
          if (truthy(blk(recv[i - 1], recv[i]))) { chunks[chunks.length - 1].push(recv[i]); }
          else { chunks.push([recv[i]]); }
        }
        return chunks;
      }
      case "slice_when": {
        // `slice_when { |prev, cur| pred }` — the INVERSE of `chunk_while`: runs
        // of consecutive elements, starting a NEW run BETWEEN an adjacent pair
        // exactly WHERE the block is truthy (chunk_while starts a new run where
        // the block is FALSY).
        // `[1,2,4,9,10,11,12].slice_when { |a,b| b-a>1 }` → [[1,2],[4],[9,10,11,12]].
        // An empty array yields []; a single element yields [[x]].
        if (typeof blk !== "function") { return ARR_MISS; }
        if (recv.length === 0) { return []; }
        const slices = [[recv[0]]];
        for (let i = 1; i < recv.length; i++) {
          if (truthy(blk(recv[i - 1], recv[i]))) { slices.push([recv[i]]); }
          else { slices[slices.length - 1].push(recv[i]); }
        }
        return slices;
      }
      case "tally": {
        // `tally` — a Hash counting how many times each element occurs, keyed
        // in first-seen order.  `["a","b","a","c","a"].tally` →
        // `{"a"=>3, "b"=>1, "c"=>1}`; an empty array yields `{}`.  Realised as a
        // `Map` (insertion-ordered), the same shape `group_by` returns and the
        // display path (`formatSeen`) prints as `{k=>v}`.  Keys compare by JS
        // SameValueZero, which agrees with Ruby `eql?`/hash on the scalar
        // elements this covers; matches the Go/Rust/Python references.
        const counts = new Map();
        for (const x of recv) { counts.set(x, (counts.get(x) || 0) + 1); }
        return counts;
      }
      case "cycle": {
        // `cycle(n) { |x| … }` — iterate the array n full passes in order,
        // yielding each element on every pass; always returns null (Ruby nil).
        // `[1,2,3].cycle(2)` yields 1,2,3,1,2,3.  n <= 0, a negative count, an
        // empty receiver, or a nil / non-integer count (Ruby's block-less
        // Enumerator and infinite no-`n` forms) yields nothing rather than
        // hanging, so emitted programs can never spin forever.
        if (typeof blk !== "function") { return ARR_MISS; }
        const n = args.length > 0 && isNum(args[0]) && Number.isInteger(numOf(args[0])) ? numOf(args[0]) : 0;
        if (n <= 0) { return null; }
        for (let p = 0; p < n; p++) { for (const x of recv) { blk(x); } }
        return null;
      }
    }
    return ARR_MISS;
  }

  // ── Ruby type reflection (`class`, `is_a?`, `instance_of?`) ────
  //
  // The Ruby CLASS NAME of a runtime value, mirroring the Go backend's
  // `_sir_ruby_class_name` so `.class` reads identically on both.  The
  // Integer-vs-Float split is answerable only because numbers now carry a
  // tag: an integral native number is an `Integer`, while a `SirFloat` box
  // (or a non-integral native number) is a `Float`.  Before tagged floats
  // this distinction was literally unrepresentable here — `7` and `7.0`
  // were the same JS value.
  function rubyClassName(v) {
    if (v === null || v === undefined) { return "NilClass"; }
    // A user instance reports its own class tag, so `obj.class` names the
    // real class (e.g. `Dog`) rather than a generic label.
    if (v instanceof SirInstance) { return v.sirClass; }
    // A raised/caught exception is an `Error`, NOT a `SirInstance`.  Route it
    // through `classOfThrown` — the SAME bucketing `rescue` matching uses — so
    // reflection and rescue never disagree: a `SirError` reports its own class
    // tag, and a native JS error (a `TypeError` from an internal operation)
    // reports `StandardError`, which is exactly the class `rescue` catches it
    // as.  Without this, `rescue => e; handle if e.is_a?(StandardError)` would
    // silently skip the handler for a value `rescue` had just caught.
    if (v instanceof Error) { return classOfThrown(v); }
    if (v === true) { return "TrueClass"; }
    if (v === false) { return "FalseClass"; }
    if (isNum(v)) { return isFloat(v) ? "Float" : "Integer"; }
    if (typeof v === "string") { return "String"; }
    if (v instanceof Sym) { return "Symbol"; }
    if (Array.isArray(v)) { return "Array"; }
    if (v instanceof Map) { return "Hash"; }
    if (v instanceof Closure) { return "Proc"; }
    return "Object";
  }
  // Built-in ancestry for `is_a?`/`kind_of?`: a value is an instance of its
  // own class AND of each ancestor.  Ruby's real MRO is deeper; this is the
  // v0 surface (`Integer`/`Float` are `Numeric` and `Comparable`, `String`
  // is `Comparable`), with `Object`/`BasicObject` matching everything.
  // A `Map` (not an object literal) so a user-defined class name can never
  // reach `Object.prototype` keys like `__proto__` on lookup.
  const BUILTIN_ANCESTORS = new Map([
    ["Integer", ["Numeric", "Comparable"]],
    ["Float", ["Numeric", "Comparable"]],
    ["String", ["Comparable"]],
  ]);
  const REFLECT_PREDICATES = new Set(["is_a?", "kind_of?", "instance_of?"]);
  function isA(v, className) {
    const actual = rubyClassName(v);
    if (actual === className) { return true; }
    if (className === "Object" || className === "BasicObject") { return true; }
    const builtin = BUILTIN_ANCESTORS.get(actual);
    if (builtin !== undefined && builtin.indexOf(className) >= 0) { return true; }
    // A user instance — or an exception, which is a `SirError` — also matches
    // its SUPERCLASS chain (the same cycle-guarded `ancestry` walk `rescue`
    // matching uses) and any module mixed in along that chain.
    if (v instanceof SirInstance || v instanceof Error) {
      if (isAncestorOrSelf(actual, className)) { return true; }
      if (includesModuleTransitively(actual, className)) { return true; }
    }
    return false;
  }
  // Does `owner` (a class) reach `target` through the modules mixed into it
  // or into any of its ancestors?  Ruby's MRO is TRANSITIVE — `class C;
  // include M; end` where `module M; include N; end` makes `c.is_a?(N)` true
  // — so the module graph must be searched, not just scanned one level.
  //
  // Deliberately ITERATIVE (an explicit worklist, not recursion): the graph's
  // depth is attacker-shaped by the source, and a recursive walk over a long
  // `include` chain would exhaust the JS call stack.  A worklist keeps the JS
  // stack at O(1) and the shared `seen` set makes each module name expand at
  // most once, so a cyclic or self-including graph terminates.
  function includesModuleTransitively(owner, target) {
    const seen = new Set();
    const work = [owner];
    while (work.length > 0) {
      // Walk this owner's SUPERCLASS chain, collecting the modules mixed in
      // at every level; `chain` guards a cyclic ancestry table.
      let cur = work.pop();
      const chain = new Set();
      while (cur !== undefined && cur !== null && !chain.has(cur)) {
        chain.add(cur);
        const mods = includedModules.get(cur);
        if (mods !== undefined) {
          for (const m of mods) {
            if (m === target) { return true; }
            if (!seen.has(m)) { seen.add(m); work.push(m); }
          }
        }
        cur = ancestry[cur];
      }
    }
    return false;
  }

  // Dispatch the universal M6 surface on ANY receiver.  Returns `M6_MISS` when
  // `name` is not an M6 method, so `callMethod` continues to its type-specific
  // paths.  `rawArgs` is the UN-unwrapped argument list — a trailing block is
  // still a `__Sir.Closure`, invoked via `applyClosure`; `send` forwards the raw
  // args so a forwarded block survives as a Closure.
  const M6_MISS = Symbol("m6-miss");
  function objectMetaMethod(recv, name, rawArgs) {
    // `send`/`__send__`/`public_send`: the first arg names a method; re-enter
    // dispatch with the remaining args.  An empty arg list has no method to
    // name — fall through to the NoMethodError floor.  Routing recurses through
    // `callMethod`, so the security gate (allowlist / method table) is reused.
    if (SEND_METHODS.has(name) && rawArgs.length > 0) {
      return callMethod(recv, methodNameArg(rawArgs[0]), ...rawArgs.slice(1));
    }
    if (name === "respond_to?") {
      return respondsTo(recv, methodNameArg(rawArgs[0]));
    }
    // Type reflection on ANY receiver: `7.class` → "Integer", `7.0.class` →
    // "Float" (the tagged-float split), `obj.class` → its own class tag.
    if (name === "class") { return rubyClassName(recv); }
    // `is_a?`/`kind_of?` honour ancestry; `instance_of?` is an EXACT class
    // match.  The class argument arrives as a NAME (the frontend lowers a
    // constant reference to its name string, and a Symbol is accepted too).
    if (REFLECT_PREDICATES.has(name) && rawArgs.length > 0) {
      const cls = methodNameArg(rawArgs[0]);
      return name === "instance_of?" ? rubyClassName(recv) === cls : isA(recv, cls);
    }
    // `tap`/`then`/`yield_self` with an actual trailing Closure block.  `tap`
    // returns the receiver; `then`/`yield_self` return the block's result.
    const last = rawArgs[rawArgs.length - 1];
    if (OBJECT_BLOCK_METHODS.has(name) && rawArgs.length > 0 && last instanceof Closure) {
      if (name === "tap") { applyClosure(last, [recv]); return recv; }
      return applyClosure(last, [recv]); // then / yield_self
    }
    // Block-less `tap`/`then`/`yield_self` returns the receiver (Ruby returns an
    // Enumerator; v0 floor, matching Python/TS).
    if (OBJECT_BLOCK_METHODS.has(name)) { return recv; }
    return M6_MISS;
  }

  function callMethod(recv, name, ...rawArgs) {
    const args = rawArgs.map(unwrapArg);
    // ── user-defined-class dispatch (O3) ─────────────────────────
    // A `SirInstance` receiver dispatches to the USER method table
    // (walking ancestry), with `self` bound for the call.  This branch
    // is taken FIRST and only for `SirInstance`s, so the built-in /
    // collection path below (arrays, strings, the RCE-hardened
    // allowlist) is completely unchanged for every other receiver.
    // Resolution is `resolveMethod` → explicit `Map.get` on the
    // `(class, method)` key; a name like `constructor` simply misses.
    if (recv instanceof SirInstance) {
      const fn = resolveMethod(methodTable, recv.sirClass, name, includedModules);
      if (fn !== undefined) {
        // A user-defined method (incl. a user override of `send`/`tap`/…) wins.
        return applyWithSelf(fn, recv, args);
      }
      // No user method — fall through to the universal M6 surface (send/tap/
      // then/yield_self/respond_to?), then raise NoMethodError if M6 misses too.
      const meta = objectMetaMethod(recv, name, rawArgs);
      if (meta !== M6_MISS) { return meta; }
      raiseError("NoMethodError",
        "undefined method `" + name + "` for an instance of `" +
        recv.sirClass + "`");
    }
    // ── M6: universal metaprogramming on a NON-instance receiver ──
    // `send`/`tap`/`then`/`yield_self`/`respond_to?` resolve on EVERY receiver
    // (primitives included), and must be recognised BEFORE the native-method
    // allowlist below — otherwise a name like `tap` or `send`, which is not a
    // native JS method, would be wrongly rejected as a NoMethodError.  Dispatch
    // routes through `objectMetaMethod` (send recurses via `callMethod`, so the
    // security gate is reused — no `recv[name]` / reflection on the name).
    {
      const meta = objectMetaMethod(recv, name, rawArgs);
      if (meta !== M6_MISS) { return meta; }
    }
    // Eager boolean operators (`&`/`|`/`^`) on a `true`/`false` receiver —
    // Ruby's non-short-circuiting logical ops, distinct from `&&`/`||`.
    if (typeof recv === "boolean") {
      const b = boolMethod(recv, name, args);
      if (b !== BOOL_MISS) { return b; }
    }
    // Ruby Numeric catalog on a number receiver (native Integer/Float OR a
    // boxed Float) — explicit-switch dispatch (no `recv[name]`) ahead of the
    // native allowlist, so `gcd`/`digits`/`upto`/… resolve while
    // `toString`/`toFixed` still fall through below.
    if (isNum(recv)) {
      const nm = numericMethod(recv, name, args);
      if (nm !== NUM_MISS) { return nm; }
    }
    // Ruby String catalog on a `string` receiver — explicit-switch dispatch (no
    // `recv[name]`) ahead of the native allowlist, so `capitalize`/`gsub`/… and
    // the semantics-diverging cases resolve while `toUpperCase`/`split`/… still
    // fall through below via the alias table.
    if (typeof recv === "string") {
      const sm = stringMethod(recv, name, args);
      if (sm !== STR_MISS) { return sm; }
    }
    // Ruby Hash catalog on a `Map` receiver — explicit-switch dispatch (no
    // `recv[name]`) ahead of the native allowlist, so `keys`/`values`/`to_a`
    // return real Arrays and `each`/`merge`/`dig`/… resolve faithfully.
    if (recv instanceof Map) {
      const hm = hashMethod(recv, name, args);
      if (hm !== HASH_MISS) { return hm; }
    }
    // Ruby Symbol catalog on a `Sym` receiver — explicit-switch dispatch (no
    // `recv[name]`) ahead of the native allowlist.
    if (recv instanceof Sym) {
      const ym = symbolMethod(recv, name, args);
      if (ym !== SYM_MISS) { return ym; }
    }
    // Ruby Array (Enumerable) catalog on an Array receiver — explicit-switch
    // dispatch (no `recv[name]`) ahead of the native allowlist.  A miss falls
    // through so the native mutators/accessors (`push`/`pop`/`slice`/…) still
    // resolve; the catalog owns the Ruby-named and semantics-diverging methods
    // (`select`/`reject`/`inject`/`any?`/numeric `sort`/`sort_by`/…) that the
    // raw JS array does not provide or provides differently.
    if (Array.isArray(recv)) {
      const am = arrayMethod(recv, name, args);
      if (am !== ARR_MISS) { return am; }
    }
    // `length` as a nullary method mirrors the property.  Kept special-cased
    // ahead of the allowlist: it is a property read, not a method call.
    if (name === "length" && args.length === 0) { return recv.length; }
    // ── `.fetch` (Ruby typed lookup) ──────────────────────────────
    // Ruby's `.fetch` is the *raising* sibling of the `[]` index op (which
    // stays nil-returning): a sequence `arr.fetch(i)` past the end raises
    // `IndexError`, and a hash `h.fetch(k)` with a MISSING key and NO
    // default (no second arg / block) raises `KeyError`.  Both are typed
    // `SirError`s here so a translated `rescue IndexError` / `rescue
    // KeyError` catches them.  A supplied default arg (`fetch(k, d)`) is
    // returned instead of raising, matching Ruby.  Handled AHEAD of the
    // allowlist because native arrays have no `fetch` and native `Map`'s
    // `fetch` does not exist / does not match Ruby's semantics.
    if (name === "fetch") {
      if (Array.isArray(recv)) {
        // Ruby allows a negative index (counts from the end); an index
        // resolving outside `0 .. length-1` is out of bounds.
        const raw = args[0];
        // SECURITY: `raw` is a source-controlled value and MUST be a real
        // integer before it can index `recv`.  A non-numeric string
        // (`"constructor"`, `"__proto__"`, `"push"`, …) would sail past the
        // `NaN`-poisoned bounds checks below (every comparison with NaN is
        // false) and reach `recv[raw]` — a reflective property read that
        // leaks prototype/host gadgets and bypasses the method allowlist.
        // Ruby itself raises here (`TypeError: no implicit conversion of
        // String into Integer`), so reject a non-integer index with the same
        // typed error rather than ever indexing by a source-derived name.
        if (typeof raw !== "number" || !Number.isInteger(raw)) {
          raiseError("TypeError",
            "no implicit conversion of " + classDescription(raw) + " into Integer");
        }
        const idx = raw < 0 ? recv.length + raw : raw;
        if (idx < 0 || idx >= recv.length) {
          if (args.length >= 2) { return args[1]; }
          raiseError("IndexError",
            "index " + format(raw) + " outside of array bounds: " +
            (recv.length === 0 ? "0...0" :
              "-" + recv.length + "..." + recv.length));
        }
        return recv[idx];
      }
      if (recv instanceof Map) {
        if (recv.has(args[0])) { return recv.get(args[0]); }
        if (args.length >= 2) { return args[1]; }
        raiseError("KeyError", "key not found: " + format(args[0]));
      }
      // A `.fetch` on any other receiver has no Ruby-collection meaning
      // here; fall through to the unknown-method NoMethodError below.
    }
    // Normalise a differently-spelled Ruby method name to its JS-native
    // equivalent (`upcase` → `toUpperCase`).  Unknown names pass through
    // unchanged and simply miss the allowlist below.  This is a fixed table
    // lookup, never a reflective transform of a source-derived name.
    const native = Object.prototype.hasOwnProperty.call(RUBY_METHOD_ALIASES, name)
      ? RUBY_METHOD_ALIASES[name]
      : name;
    // SECURITY gate: refuse any name outside the allowlist so reflective
    // gadgets (`constructor`, `__proto__`, `apply`, …) can never be reached.
    // An allowlist miss is a *genuinely unknown* method, so — matching Ruby
    // — we raise a typed `NoMethodError` (rescuable via `rescue
    // NoMethodError`) rather than a JS-native `TypeError` (which a `rescue`
    // would either miss or, worse, catch as an over-broad StandardError).
    // The reflective gadgets never appear in the allowlist, so they still
    // land here and are rejected before any property is looked up.  The error
    // message reports the ORIGINAL Ruby name the source wrote.
    if (!METHOD_ALLOWLIST.has(native)) {
      raiseError("NoMethodError",
        "undefined method `" + name + "` for " + classDescription(recv));
    }
    const m = recv == null ? undefined : recv[native];
    if (typeof m !== "function") {
      raiseError("NoMethodError",
        "undefined method `" + name + "` for " + classDescription(recv));
    }
    return m.apply(recv, args);
  }

  // A Ruby-ish description of a receiver for a `NoMethodError` message —
  // e.g. `nil`, `an instance of Array`, `an instance of String`.  Pure
  // TAG tests on the runtime representation, never reflection on a
  // source-derived name, so no gadget is reachable from here.
  function classDescription(recv) {
    if (recv === null || recv === undefined) { return "nil"; }
    if (Array.isArray(recv)) { return "an instance of Array"; }
    if (recv instanceof Map) { return "an instance of Hash"; }
    if (typeof recv === "string") { return "an instance of String"; }
    if (isNum(recv)) { return "an instance of Numeric"; }
    if (typeof recv === "boolean") { return recv ? "true" : "false"; }
    if (recv instanceof Sym) { return "an instance of Symbol"; }
    return "an instance of Object";
  }

  // ── exceptions (SIR17 `Feature::Exceptions`) ───────────────────
  // Most SIR lowers to native JavaScript, and exception handling is
  // *mostly* native too: `Stmt::TryCatch` becomes a real
  // `try { … } catch (__exc) { … } finally { … }`.  Two pieces have no
  // faithful native equivalent and live here — mirroring the published
  // TypeScript `@coding-adventures/sir-runtime-exceptions` package
  // (ported to plain JS so the JavaScript backend stays self-contained,
  // no `import`/`require`):
  //
  //   1. A *class-tagged* thrown object.  Ruby's `raise ArgumentError,
  //      "boom"` names a **class** and carries a message; JavaScript's
  //      `throw` takes any value and its `Error` has no Ruby class tag.
  //      `SirError` is a real `Error` (so stack traces work) that also
  //      records the SIR class name in `sirClass`.
  //   2. Rescue-clause *type matching*.  A native `catch` binds one
  //      variable and catches everything; Ruby's ordered typed `rescue`
  //      clauses match a *set* of classes (and their subclasses) and
  //      fall through otherwise.  `rescueMatches` answers "does this
  //      caught value match this clause's class list?" so the emitted
  //      `catch` body dispatches to the right clause (or re-`throw`s).

  // Built-in Ruby exception ancestry: subclass name → immediate
  // superclass name.  Walked by `isAncestorOrSelf` so a
  // `rescue StandardError` also catches the everyday subclasses a program
  // raises.  A curated slice of Ruby's tree (the classes a frontend is
  // likely to name), each chaining up to `StandardError → Exception`.
  //
  //   Exception
  //   └─ StandardError
  //      ├─ RuntimeError ├─ ArgumentError ├─ TypeError
  //      ├─ NameError ─ NoMethodError      ├─ RangeError
  //      ├─ IndexError ─ KeyError          ├─ ZeroDivisionError
  //      ├─ IOError    ├─ StopIteration    └─ NotImplementedError
  //
  // `ancestry` starts as a copy of the built-in table; user-defined
  // classes are merged in at program init via `registerAncestry` (below).
  const BUILTIN_ANCESTRY = {
    RuntimeError: "StandardError",
    ArgumentError: "StandardError",
    TypeError: "StandardError",
    NameError: "StandardError",
    NoMethodError: "NameError",
    IndexError: "StandardError",
    KeyError: "IndexError",
    RangeError: "StandardError",
    ZeroDivisionError: "StandardError",
    IOError: "StandardError",
    StopIteration: "StandardError",
    NotImplementedError: "StandardError",
    StandardError: "Exception",
  };
  // A *mutable* lookup seeded from the built-ins.  `Object.create(null)`
  // gives a prototype-less map so a user class literally named
  // `"constructor"`/`"__proto__"` cannot poison the lookup — dispatch is
  // pure DATA, never reflection.
  const ancestry = Object.assign(Object.create(null), BUILTIN_ANCESTRY);

  // Merge a user `{ childClass: superclassName }` map into `ancestry`
  // (E2, the JS half of user-defined class ancestry).  The emitter
  // collects the module's `class Child < Super` pairs and emits ONE
  // `__Sir.registerAncestry(...)` call at program init, so
  // `class MyErr < StandardError; raise MyErr; rescue StandardError`
  // matches through the merged chain.  Own-keys only (no inherited
  // prototype keys) keeps the merge to explicit source-declared pairs.
  function registerAncestry(map) {
    if (map == null) { return; }
    for (const child of Object.keys(map)) {
      ancestry[child] = map[child];
    }
  }

  // A SIR exception: a native `Error` tagged with its Ruby class name.
  // `sirClass` is what `rescueMatches` dispatches on; `message` is the
  // human string `raise Klass, "msg"` carries (defaulting to the class
  // name, matching Ruby's `exception.message`).
  class SirError extends Error {
    constructor(sirClass, message) {
      const text =
        message === undefined || message === null ? sirClass : String(message);
      super(text);
      this.sirClass = sirClass;
      this.name = sirClass;
      // Restore the prototype chain so `err instanceof SirError` holds.
      Object.setPrototypeOf(this, new.target.prototype);
    }
  }

  // Raise a SIR exception of class `className` with an optional message.
  // Emitted for `raise Foo, "msg"` → `raiseError("Foo", "msg")`,
  // `raise Foo` → `raiseError("Foo")`, and bare `raise` → `raiseError()`
  // → a generic `RuntimeError` (SIR v0 does not thread the in-flight
  // exception into a bare re-raise; documented limitation).
  function raiseError(className, message) {
    throw new SirError(className === undefined ? "RuntimeError" : className, message);
  }

  // The SIR class name of a caught value.  A `SirError` reports its tag;
  // any other thrown value (a native `Error`, a bare string, …) is
  // bucketed as `StandardError` — the everyday rescuable root — so a
  // `rescue StandardError` / bare `rescue` also catches JS runtime errors.
  function classOfThrown(err) {
    if (err instanceof SirError) { return err.sirClass; }
    return "StandardError";
  }

  // `true` if `actual` is `target` or any of its registered ancestors.
  // The `seen` guard makes a malformed (cyclic) user ancestry map
  // terminate rather than loop forever.  Lookup is by EXPLICIT table
  // (`ancestry[cur]`), never `eval`/reflection — class names are data.
  function isAncestorOrSelf(actual, target) {
    let cur = actual;
    const seen = new Set();
    while (cur !== undefined && cur !== null && !seen.has(cur)) {
      if (cur === target) { return true; }
      seen.add(cur);
      cur = ancestry[cur];
    }
    return false;
  }

  // Does a caught value match a rescue clause naming `classNames`?
  //   - empty `classNames` → a bare `rescue` (catch-all) → always true.
  //   - `Exception` → Ruby's universal root → matches anything.
  //   - otherwise: matches if the value's class equals or descends from
  //     any named class (per `ancestry`; user classes by exact name or
  //     registered chain).
  // The emitted `catch` calls this once per clause in source order,
  // running the first match's body and re-`throw`ing if none match.
  function rescueMatches(err, classNames) {
    if (classNames.length === 0) { return true; }
    const actual = classOfThrown(err);
    return classNames.some(
      (name) => name === "Exception" || isAncestorOrSelf(actual, name),
    );
  }

  // ── user-defined-class OOP (SIR18 `Classes` dispatch, O3) ──────
  // The exceptions work above already gave us the *ancestry* map — a
  // pure-DATA `class → superclass` table walked with a `seen` cycle
  // guard, never reflection.  OOP method dispatch reuses that exact
  // walk and adds three data structures, all built to the same
  // security bar (spelled out under SECURITY below):
  //
  //   • `SirInstance`   — a user object: a class-name tag + its own
  //                       instance-variable bag (`@x`).
  //   • `methodTable`   — instance methods, keyed by a FLAT string
  //                       `"Class\x00method"`.
  //   • `classMethodTable` — class ("static") methods, same keying.
  //   • `selfStack`     — the dynamic `self` binding a running method
  //                       reads via `currentSelf()` / `ivarGet`/`ivarSet`.
  //
  // SECURITY (the C3 RCE lesson bit THIS crate — see the method
  // allowlist above).  Every lookup here is an explicit `Map.get` on a
  // `(class, method)` string key.  We NEVER do `recv[name]`, `eval`,
  // `new Function`, or any reflection on a source-derived name.  So a
  // user class or method literally named `constructor` / `__proto__` /
  // `prototype` is only ever a Map *key* — a miss floors to "method not
  // found", it can never reach a host callable.  Two further hardening
  // points: the tables are real `Map`s (not `{}`), so a `"__proto__"`
  // key cannot poison the prototype chain; and the `\x00` (NUL)
  // separator cannot appear in a Ruby/JS identifier, so distinct
  // `(class, method)` pairs can never collide into one key.

  // A user object.  `sirClass` is the class-name tag dispatch keys on;
  // `ivars` is a prototype-less bag so an instance variable literally
  // named `"__proto__"` is plain data, never a prototype write.
  class SirInstance {
    constructor(sirClass) {
      this.sirClass = sirClass;
      this.ivars = Object.create(null);
    }
  }
  // Bare allocation — no `initialize` yet (that is `callNew`'s job).
  function newInstance(cls) { return new SirInstance(cls); }

  // Flat method-table key.  `\x00` (NUL) never occurs in a source
  // identifier, so `"A\x00m"` and `"A" + "\x00m"` are the SAME key while
  // two genuinely different pairs never collide.  A plain string key in
  // a `Map` (not an object property) means `"constructor"`/`"__proto__"`
  // are inert data, closing the prototype-pollution / gadget door.
  function methodKey(cls, name) { return cls + "\x00" + name; }
  const methodTable = new Map();
  const classMethodTable = new Map();
  // `def m … end` inside `class C` → `defMethod("C", "m", <closure>)`.
  function defMethod(cls, name, fn) { methodTable.set(methodKey(cls, name), fn); }
  // `def self.m …` → a class ("static") method.
  function defClassMethod(cls, name, fn) {
    classMethodTable.set(methodKey(cls, name), fn);
  }

  // ── mixins (MX4): `include` / `extend` ─────────────────────────────
  //
  // A *module* registers its `def`s exactly like a class (via `defMethod`
  // keyed by the module name — an "owner" is now a class OR a module).
  // Two per-owner association lists connect an owner to the modules mixed
  // into it, in Ruby's include order:
  //
  //   • `includedModules[owner] = [M1, M2, …]` — `include M` appends `M`.
  //     Ruby searches the MOST-RECENTLY-included module first, so the MRO
  //     walk iterates this list in REVERSE.  The module's *instance*
  //     methods become instance methods of the owner.
  //   • `extendedModules[owner] = [M1, M2, …]` — `extend M` appends `M`.
  //     The module's *instance* methods become CLASS ("singleton")
  //     methods of the owner (callable as `Owner.method`).
  //
  // SECURITY (the same bar as the method tables): both are real `Map`s
  // keyed on the owner *name string*, holding arrays of module *name
  // strings*.  Nothing here is `Object`-property access, so a module or
  // owner literally named `__proto__` / `constructor` is inert data — a
  // Map key / array element, never a prototype write or a host callable.
  const includedModules = new Map();
  const extendedModules = new Map();
  // Append `mod` to `owner`'s list in `map`, preserving include order and
  // idempotently allowing a re-include (Ruby keeps the first position; a
  // repeat is harmless — the MRO walk de-dupes with its `seen` set).
  function appendModule(map, owner, mod) {
    let list = map.get(owner);
    if (list === undefined) { list = []; map.set(owner, list); }
    list.push(mod);
  }
  // `include M` inside `class C` (or module) → `includeModule("C", "M")`.
  function includeModule(owner, mod) { appendModule(includedModules, owner, mod); }
  // `extend M` → `extendModule("C", "M")` (M's methods become CLASS methods).
  function extendModule(owner, mod) { appendModule(extendedModules, owner, mod); }

  // Resolve `name` on `cls` following Ruby's Method Resolution Order (MRO),
  // walking the SAME `ancestry` table the exception runtime uses AND the
  // per-owner module lists.  For each class in the superclass chain the
  // walk checks, in order:
  //
  //   1. the class's OWN entry in `ownerTable` (a class-defined method
  //      SHADOWS a mixed module method — class-first MRO);
  //   2. its mixed-in modules, MOST-RECENT-FIRST (reverse of mix order),
  //      each recursively expanded so a module's OWN `include`d modules are
  //      searched too (depth-first);
  //   3. then it ascends to the superclass and repeats.
  //
  // Two tables split the class's own methods from a MODULE's methods:
  //   • instance dispatch: `ownerTable = moduleTable = methodTable`, and
  //     `topModules = includedModules` — a class and its included modules
  //     both live in `methodTable`.
  //   • class-method dispatch (`extend`): `ownerTable = classMethodTable`
  //     (the class's own `def self.m`), `moduleTable = methodTable` (a
  //     module's plain `def foo` — `extend` promotes those to class
  //     methods), and `topModules = extendedModules`.
  // Below the top owner we always follow a module's `includedModules` and
  // read from `moduleTable`, because a module mixes in *instance* methods
  // regardless of whether the top owner included or extended it.
  //
  // A single shared `seen` set spans the WHOLE walk, so a diamond include
  // (a module reached via two paths) is checked ONCE at its earliest
  // position, and a cyclic hierarchy / self-including module terminates
  // instead of looping.  Every lookup is `table.get(methodKey(owner,
  // name))` — explicit data, never `[name]` / reflection.
  function resolveMethod(ownerTable, cls, name, topModules) {
    const seen = new Set();
    // Search a MODULE `mod` (and, depth-first, its own included modules).
    // A module's methods always live in `methodTable` (instance methods).
    function searchModule(mod) {
      if (mod === undefined || mod === null || seen.has(mod)) {
        return undefined;
      }
      seen.add(mod);
      const own = methodTable.get(methodKey(mod, name));
      if (own !== undefined) { return own; }
      const mods = includedModules.get(mod);
      if (mods !== undefined) {
        for (let i = mods.length - 1; i >= 0; i--) {
          const fn = searchModule(mods[i]);
          if (fn !== undefined) { return fn; }
        }
      }
      return undefined;
    }
    // Search a CLASS `owner`: its own table first, then its top-level
    // mixed-in modules (most-recently-mixed wins → reverse iteration).
    function searchOwner(owner) {
      if (owner === undefined || owner === null || seen.has(owner)) {
        return undefined;
      }
      seen.add(owner);
      const own = ownerTable.get(methodKey(owner, name));
      if (own !== undefined) { return own; }
      const mods = topModules === undefined ? undefined : topModules.get(owner);
      if (mods !== undefined) {
        for (let i = mods.length - 1; i >= 0; i--) {
          const fn = searchModule(mods[i]);
          if (fn !== undefined) { return fn; }
        }
      }
      return undefined;
    }
    // Walk the superclass chain; the `ancestry` edge itself is also
    // `seen`-guarded above, so a cyclic `ancestry` map still terminates.
    let cur = cls;
    while (cur !== undefined && cur !== null && !seen.has(cur)) {
      const fn = searchOwner(cur);
      if (fn !== undefined) { return fn; }
      cur = ancestry[cur];
    }
    return undefined;
  }

  // ── the dynamic `self` stack ───────────────────────────────────
  // A running method needs to know its receiver for `@ivar` reads and
  // for `self`.  We push the receiver before applying a method and pop
  // it in a `finally`, so an exception thrown mid-method still unwinds
  // the stack (no stale `self` leaks to the next dispatch).
  const selfStack = [];
  function pushSelf(v) { selfStack.push(v); }
  function popSelf() { selfStack.pop(); }
  // Top of the stack, or `null` outside any method (`__self__`).
  function currentSelf() {
    return selfStack.length === 0 ? null : selfStack[selfStack.length - 1];
  }

  // Apply a resolved method closure with `recv` bound as `self`.  The
  // closure is the `__Sir.Closure` a `MakeClosure` produced for the
  // method body; `applyClosure` invokes it.  try/finally keeps the
  // self-stack balanced even when the body throws.
  function applyWithSelf(fn, recv, args) {
    pushSelf(recv);
    try {
      return applyClosure(fn, args);
    } finally {
      popSelf();
    }
  }

  // `Klass.new(args…)` → `callNew("Klass", args…)`.  Allocate, then run
  // the inherited `initialize` (if any) with `self` bound to the fresh
  // instance, then return the instance (NOT `initialize`'s result — Ruby
  // discards it).  A class with no `initialize` anywhere in its chain is
  // valid: `new` just yields a bare instance.
  function callNew(cls, ...args) {
    const obj = newInstance(cls);
    const init = resolveMethod(methodTable, cls, "initialize", includedModules);
    if (init !== undefined) { applyWithSelf(init, obj, args); }
    return obj;
  }

  // `Klass.m(args…)` on a CONSTANT receiver → `callClassMethod("Klass",
  // "m", args…)`.  Resolves through the CLASS-method MRO: the class's own
  // `def self.m` table first, then any `extend`ed modules (most-recently-
  // extended first), ascending the superclass chain — the class-method
  // analogue of instance dispatch, so `extend M` makes `M`'s instance
  // methods callable as `Klass.m`.  `self` is bound to the class-name
  // string for the duration (a module's method body may read `self`),
  // mirroring Ruby where `self` inside a class method is the class.  A
  // miss is a NoMethodError, matching Ruby's `undefined method` for a
  // class receiver.
  function callClassMethod(cls, name, ...args) {
    const fn = resolveMethod(classMethodTable, cls, name, extendedModules);
    if (fn === undefined) {
      raiseError("NoMethodError",
        "undefined method `" + name + "` for class `" + cls + "`");
    }
    return applyWithSelf(fn, cls, args);
  }

  // `super(args…)` inside method `method` of class `cls` →
  // `callSuper("method", "cls", args…)`.  Resolve `method` starting from
  // the SUPERCLASS of `cls` (skipping the current definition) and apply
  // it with the CURRENT `self` still bound — `super` reuses the live
  // receiver.  A missing super method is a NoMethodError, matching Ruby.
  function callSuper(method, cls, ...args) {
    const fn = resolveMethod(methodTable, ancestry[cls], method, includedModules);
    if (fn === undefined) {
      raiseError("NoMethodError",
        "super: no superclass method `" + method + "` for `" + cls + "`");
    }
    // Reuse the live self (do NOT push a new one): `super` runs in the
    // same object context as the caller.
    return applyClosure(fn, args);
  }

  // ── instance / class variables on the current self ─────────────
  // `@x` read / write route here.  They act on `currentSelf()` — a
  // method body's receiver.  Reading an unset `@x` yields `null` (Ruby's
  // nil), matching the `Scope::Instance` "no prior declaration" rule.
  // `Object.create(null)` for the bag means a `"__proto__"` ivar is data.
  function ivarGet(name) {
    const self = currentSelf();
    if (self instanceof SirInstance) {
      const v = self.ivars[name];
      return v === undefined ? null : v;
    }
    return null;
  }
  function ivarSet(name, val) {
    const self = currentSelf();
    if (self instanceof SirInstance) { self.ivars[name] = val; }
    return val;
  }
  // Class variables (`@@x`) are shared per class name.  A prototype-less
  // per-class bag, keyed off the current self's class, keeps them out of
  // any object's prototype chain.
  const classVarBags = new Map();
  function classVarBag(cls) {
    let bag = classVarBags.get(cls);
    if (bag === undefined) { bag = Object.create(null); classVarBags.set(cls, bag); }
    return bag;
  }
  function cvarGet(name) {
    const self = currentSelf();
    const cls = self instanceof SirInstance ? self.sirClass : null;
    if (cls === null) { return null; }
    const v = classVarBag(cls)[name];
    return v === undefined ? null : v;
  }
  function cvarSet(name, val) {
    const self = currentSelf();
    const cls = self instanceof SirInstance ? self.sirClass : null;
    if (cls !== null) { classVarBag(cls)[name] = val; }
    return val;
  }

  // ── symbolic expressions + pattern/rewrite (SIR23) ─────────────
  //
  // A `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
  // `SymPatternNamed`/`SymRule`/`SymReplaceAll` node lowers to a call
  // into `Symbolic.*` below — the same treatment the exceptions
  // section above gave `@coding-adventures/sir-runtime-exceptions`: a
  // plain-JS port of the published TypeScript packages
  // (`@coding-adventures/symbolic-ir`, `@coding-adventures/
  // cas-pattern-matching`, `@coding-adventures/sir-runtime-symbolic`)
  // this backend's TypeScript sibling *imports*, so the JavaScript
  // artifact stays self-contained (no `require`/`import`).
  //
  // ## Value model
  //
  // A term is a plain, `Object.freeze`d object — never a class
  // instance, so it never collides with `Sym`/`Pair`/`Closure`/
  // `SirInstance` above:
  //
  //   { kind: "symbol",   name }
  //   { kind: "integer",  value }          -- a JS `number`, NOT bigint.
  //   { kind: "rational", numer, denom }   -- reduced; denom > 0.
  //   { kind: "float",    value }
  //   { kind: "string",   value }
  //   { kind: "apply",    head, args }
  //
  // The TypeScript sibling package uses `bigint` for `integer`/
  // `rational` (arbitrary precision); this port deliberately uses
  // `number` instead, matching how every OTHER numeric value in this
  // backend already works (`IntLit` emits a bare JS number literal —
  // see `emit.rs`'s `Expr::IntLit` arm — there is no bigint anywhere
  // else in this runtime). A `Symbolic.int`/`Symbolic.rational` value
  // therefore shares the same `Number.isSafeInteger` ceiling ordinary
  // SIR integers already have on this backend; this is a pre-existing
  // backend-wide limitation, not a new one this port introduces.
  const Symbolic = (() => {
    function frozen(obj) { return Object.freeze(obj); }

    function symTerm(name) { return frozen({ kind: "symbol", name }); }
    function intTerm(value) {
      value = numOf(value); // boundary unwrap: a tagged Float box → raw f64
      if (!Number.isSafeInteger(value)) {
        throw new RangeError("Symbolic.int: value must be a safe integer");
      }
      return frozen({ kind: "integer", value });
    }
    function gcdAbs(a, b) {
      a = Math.abs(a);
      b = Math.abs(b);
      while (b !== 0) {
        const t = b;
        b = a % b;
        a = t;
      }
      return a === 0 ? 1 : a;
    }
    function rationalTerm(numer, denom) {
      if (denom === 0) {
        throw new RangeError("Symbolic.rational: denominator cannot be zero");
      }
      if (denom < 0) {
        numer = -numer;
        denom = -denom;
      }
      const g = gcdAbs(numer, denom);
      return frozen({ kind: "rational", numer: numer / g, denom: denom / g });
    }
    function floatTerm(value) {
      value = numOf(value); // boundary unwrap: a tagged Float box → raw f64
      if (!Number.isFinite(value)) {
        throw new RangeError("Symbolic.numberNode: value must be finite");
      }
      return frozen({ kind: "float", value });
    }
    function stringTerm(value) { return frozen({ kind: "string", value }); }
    function applyTerm(head, args) {
      return frozen({ kind: "apply", head, args: Object.freeze([...args]) });
    }

    // Structural equality — used by the matcher (a repeated pattern
    // variable must bind to the SAME term every occurrence) and by
    // `replaceRepeated`'s "did this firing actually change anything"
    // fixed-point check.
    function termEquals(a, b) {
      if (a.kind !== b.kind) { return false; }
      switch (a.kind) {
        case "symbol": return a.name === b.name;
        case "integer": return a.value === b.value;
        case "rational": return a.numer === b.numer && a.denom === b.denom;
        case "float": return Object.is(a.value, b.value);
        case "string": return a.value === b.value;
        case "apply":
          return termEquals(a.head, b.head)
            && a.args.length === b.args.length
            && a.args.every((arg, i) => termEquals(arg, b.args[i]));
        default: return false;
      }
    }

    function headName(node) { return node.kind === "symbol" ? node.name : ""; }

    // A display helper `print`/`puts`/`formatSeen` (above) reach for,
    // mirroring `symbolic-ir`'s own `toDisplayString`. Not part of the
    // SIR23 spec's own contract.
    //
    // SECURITY (CWE-674): a term built via `Symbolic.apply`/`applyTerm`
    // is NOT depth-capped at construction time (only `replaceAll`/
    // `replaceRepeated`'s tree WALK enforces `MAX_TERM_DEPTH` above) — a
    // compiled program can build one arbitrarily deep directly, e.g. a
    // loop lowered to repeated `SymApply` nesting with an
    // attacker-influenced iteration count. Without its own cap, this
    // recursive walk would `RangeError: Maximum call stack size
    // exceeded` well before 512 levels (this walk's per-frame cost is
    // heavier than `walkOnce`'s). `depth` mirrors the SAME
    // `MAX_TERM_DEPTH` cap and truncates rather than crashing, matching
    // how `formatSeen` already renders `"[...]"`/`"{...}"` for an
    // Array/Map cycle instead of recursing forever.
    function toDisplayString(node, depth) {
      if (depth === undefined) { depth = 0; }
      if (depth > MAX_TERM_DEPTH) { return "..."; }
      switch (node.kind) {
        case "symbol": return node.name;
        case "integer": return String(node.value);
        case "rational": return node.numer + "/" + node.denom;
        case "float": return String(node.value);
        case "string": return JSON.stringify(node.value);
        case "apply":
          return toDisplayString(node.head, depth + 1) + "("
            + node.args.map((a) => toDisplayString(a, depth + 1)).join(", ") + ")";
        default: return undefined;
      }
    }

    // ── pattern/rule vocabulary (cas-pattern-matching) ─────────────
    const BLANK = "Blank";
    const PATTERN = "Pattern";
    const RULE = "Rule";
    const RULE_DELAYED = "RuleDelayed";

    function isHead(node, name) {
      return node.kind === "apply" && node.head.kind === "symbol" && node.head.name === name;
    }
    function isBlank(node) { return isHead(node, BLANK); }
    function isPattern(node) { return isHead(node, PATTERN); }
    function isRule(node) {
      return node.kind === "apply" && node.head.kind === "symbol"
        && (node.head.name === RULE || node.head.name === RULE_DELAYED)
        && node.args.length === 2;
    }

    function blankTerm() { return applyTerm(symTerm(BLANK), []); }
    function blankTypedTerm(head) { return applyTerm(symTerm(BLANK), [symTerm(head)]); }
    function namedTerm(name, inner) { return applyTerm(symTerm(PATTERN), [symTerm(name), inner]); }
    function ruleTerm(lhs, rhs) { return applyTerm(symTerm(RULE), [lhs, rhs]); }
    function ruleDelayedTerm(lhs, rhs) { return applyTerm(symTerm(RULE_DELAYED), [lhs, rhs]); }

    // Bindings: a name -> term map. Persistent / copy-on-write (mirrors
    // `cas-pattern-matching`'s `Bindings` class) so a failed match
    // attempt never mutates a binding set an earlier attempt still
    // holds a reference to.
    function bindingsEmpty() { return new Map(); }
    function bindingsBind(bindings, name, value) {
      const existing = bindings.get(name);
      if (existing !== undefined && termEquals(existing, value)) { return bindings; }
      const next = new Map(bindings);
      next.set(name, value);
      return next;
    }

    function blankHeadConstraint(node) {
      if (node.args.length === 0) { return null; }
      const first = node.args[0];
      return first.kind === "symbol" ? first.name : null;
    }
    function patternName(node) {
      const first = node.args[0];
      if (first === undefined || first.kind !== "symbol") {
        throw new TypeError("Symbolic: Pattern name must be a Symbol");
      }
      return first.name;
    }
    function patternInner(node) {
      if (node.args.length < 2) {
        throw new TypeError("Symbolic: Pattern requires an inner expression");
      }
      return node.args[1];
    }
    function effectiveHeadName(node) {
      if (node.kind === "apply") { return headName(node.head) || "Apply"; }
      if (node.kind === "integer") { return "Integer"; }
      if (node.kind === "rational") { return "Rational"; }
      if (node.kind === "float") { return "Float"; }
      if (node.kind === "string") { return "String"; }
      return "Symbol";
    }

    // Five-case structural matcher: `Blank()`, `Blank(T)`,
    // `Pattern(name, inner)`, compound-vs-compound (recurse head +
    // every arg, same arity required), and plain structural equality —
    // a direct port of `cas-pattern-matching::matchPattern`.
    function matchPattern(pattern, target, bindings) {
      if (bindings === undefined) { bindings = bindingsEmpty(); }
      if (isBlank(pattern)) {
        const constraint = blankHeadConstraint(pattern);
        if (constraint === null) { return bindings; }
        return effectiveHeadName(target) === constraint ? bindings : null;
      }
      if (isPattern(pattern)) {
        const name = patternName(pattern);
        const inner = patternInner(pattern);
        const matched = matchPattern(inner, target, bindings);
        if (matched === null) { return null; }
        const existing = matched.get(name);
        if (existing !== undefined) { return termEquals(existing, target) ? matched : null; }
        return bindingsBind(matched, name, target);
      }
      if (pattern.kind === "apply") {
        if (target.kind !== "apply") { return null; }
        let current = matchPattern(pattern.head, target.head, bindings);
        if (current === null) { return null; }
        if (pattern.args.length !== target.args.length) { return null; }
        for (let i = 0; i < pattern.args.length; i++) {
          current = matchPattern(pattern.args[i], target.args[i], current);
          if (current === null) { return null; }
        }
        return current;
      }
      return termEquals(pattern, target) ? bindings : null;
    }

    function substituteTerm(template, bindings) {
      if (isPattern(template)) {
        const captured = bindings.get(patternName(template));
        return captured !== undefined ? captured : template;
      }
      if (template.kind === "apply") {
        return applyTerm(
          substituteTerm(template.head, bindings),
          template.args.map((a) => substituteTerm(a, bindings)),
        );
      }
      return template;
    }

    function applyRuleTerm(rewriteRule, expr) {
      if (!isRule(rewriteRule)) {
        throw new TypeError("Symbolic.applyRule: expected Rule/RuleDelayed");
      }
      const lhs = rewriteRule.args[0];
      const rhs = rewriteRule.args[1];
      const bindings = matchPattern(lhs, expr, bindingsEmpty());
      return bindings === null ? null : substituteTerm(rhs, bindings);
    }

    // ── replaceAll / replaceRepeated (`/.` / `//.`) + depth guard ──
    //
    // `matchPattern`/`substituteTerm`/`applyRuleTerm` recurse, but only
    // as deep as a single RULE's own (author-written, not runtime-
    // controlled) pattern/RHS shape — always shallow regardless of how
    // deep the *target* expression is. `replaceAllTerm`/
    // `replaceRepeatedTerm`, by contrast, walk the ENTIRE target
    // expression tree, which ordinary compiled-program data can build
    // up to unbounded depth — so these two need an explicit cap
    // (CWE-674 stack-overflow DoS guard), mirroring
    // `semantic_ir::limits::MAX_IR_DEPTH`'s rationale.
    const MAX_TERM_DEPTH = 512;

    function isDepthLimitError(v) {
      return v !== null && typeof v === "object" && v.kind === "depth-limit";
    }
    function isRewriteCycleErrorTerm(v) {
      return v !== null && typeof v === "object" && v.kind === "rewrite-cycle";
    }

    // `expr /. rules` — one pass, bottom-up: a node's head/args are
    // walked (and possibly replaced) before the node itself is tried
    // against `rules`; the first matching rule wins and the freshly
    // substituted replacement is NOT re-walked or retried at that same
    // position (Wolfram's single-pass `/.` contract, distinct from
    // {@link replaceRepeatedTerm}'s fixed point below).
    function walkOnce(node, rules, depth) {
      if (depth > MAX_TERM_DEPTH) {
        return { kind: "depth-limit", maxDepth: MAX_TERM_DEPTH };
      }
      let current = node;
      if (node.kind === "apply") {
        const newHead = walkOnce(node.head, rules, depth + 1);
        if (isDepthLimitError(newHead)) { return newHead; }
        const newArgs = [];
        for (const arg of node.args) {
          const nextArg = walkOnce(arg, rules, depth + 1);
          if (isDepthLimitError(nextArg)) { return nextArg; }
          newArgs.push(nextArg);
        }
        current = applyTerm(newHead, newArgs);
      }
      for (const candidateRule of rules) {
        const replacement = applyRuleTerm(candidateRule, current);
        if (replacement !== null) { return replacement; }
      }
      return current;
    }

    function replaceAllTerm(expr, rules) {
      return walkOnce(expr, rules, 0);
    }

    // `expr //. rules` — a fixed point: at each subtree, keep retrying
    // `rules` until none fire (re-walking any fresh replacement so its
    // own sub-parts also converge) before moving up to the parent.
    // `maxIterations` (default 100) is a GLOBAL cap shared across the
    // whole walk, guarding against a non-terminating rule set (SIR23
    // spec "Matcher semantics" point 6). A firing loops LOCALLY at the
    // current call frame (never a recursive call on the replacement),
    // so however many times a rule fires at one tree position costs
    // O(1) native stack frames, not O(firings) — `depth` only
    // increases on a genuine descent into `head`/`args`, so
    // `maxIterations` bounds iteration COUNT (CPU time) only, never
    // native recursion depth.
    function replaceRepeatedTerm(expr, rules, maxIterations) {
      if (maxIterations === undefined) { maxIterations = 100; }
      let counter = 0;
      function walk(node, depth) {
        if (depth > MAX_TERM_DEPTH) {
          return { kind: "depth-limit", maxDepth: MAX_TERM_DEPTH };
        }
        let current = node;
        while (true) {
          if (current.kind === "apply") {
            const newHead = walk(current.head, depth + 1);
            if (isDepthLimitError(newHead) || isRewriteCycleErrorTerm(newHead)) { return newHead; }
            const newArgs = [];
            for (const arg of current.args) {
              const nextArg = walk(arg, depth + 1);
              if (isDepthLimitError(nextArg) || isRewriteCycleErrorTerm(nextArg)) { return nextArg; }
              newArgs.push(nextArg);
            }
            current = applyTerm(newHead, newArgs);
          }
          let fired = false;
          for (const candidateRule of rules) {
            const replacement = applyRuleTerm(candidateRule, current);
            if (replacement !== null && !termEquals(replacement, current)) {
              counter += 1;
              if (counter > maxIterations) {
                return { kind: "rewrite-cycle", maxIterations };
              }
              current = replacement;
              fired = true;
              break;
            }
          }
          if (!fired) { return current; }
        }
      }
      return walk(expr, 0);
    }

    // Unwrap a `replaceAll`/`replaceRepeated` result, throwing a plain
    // `Error` if the walk hit its depth cap or (for `replaceRepeated`)
    // its iteration cap instead of returning a real term. Every
    // compiled `SymReplaceAll` call site routes through this — a
    // `SymReplaceAll` is an ordinary expression that must evaluate to a
    // term or fail loudly, never silently hand a sentinel to code
    // expecting a term.
    function unwrapTerm(result) {
      if (isDepthLimitError(result) || isRewriteCycleErrorTerm(result)) {
        throw new Error("sir-runtime-symbolic: " + result.kind);
      }
      return result;
    }

    return {
      sym: symTerm, int: intTerm, rational: rationalTerm,
      numberNode: floatTerm, stringNode: stringTerm, apply: applyTerm,
      blank: blankTerm, blankTyped: blankTypedTerm, named: namedTerm,
      rule: ruleTerm, ruleDelayed: ruleDelayedTerm,
      matchPattern, applyRule: applyRuleTerm, substitute: substituteTerm,
      replaceAll: replaceAllTerm, replaceRepeated: replaceRepeatedTerm,
      unwrap: unwrapTerm, toDisplayString, equals: termEquals,
    };
  })();

  // ── array/matrix domain (SIR22) ────────────────────────────────
  //
  // `ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`
  // (and `IndexSet`, a `Stmt`) lower to calls into `Array.*` below — a
  // plain-JS port of the published `@coding-adventures/sir-runtime-array`
  // TypeScript package this backend's TypeScript sibling *imports*, so
  // the JavaScript artifact stays self-contained — the same treatment
  // `Symbolic` above already got for SIR23.
  //
  // ## Value model
  //
  // `{ shape: number[], data: Float64Array }` — dense, rectangular,
  // COLUMN-MAJOR storage (Fortran/MATLAB order), mirroring
  // `array_runtime::value::Array` (`code/packages/rust/array-runtime/src/value.rs`)
  // field-for-field. `shape == []` is a scalar, `[n]` a vector (an `n×1`
  // column for row/column purposes), `[r, c]` a matrix — this port's
  // whole scope, like the Rust reference and the TypeScript sibling, is
  // rank ≤ 2.
  //
  // ## The SIR22 "APL addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/
  // `Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`)
  //
  // These nine were deferred when the base cut above first landed (no
  // frontend crate emitted them yet) but `apl-to-semantic-ir` now does —
  // APL's `/` (reduce), `\` (scan), `∘.` (outer product), `⍴`, `⍳`, and `,`
  // are first-class glyphs, not library calls, so real APL source reaches
  // every one of these nodes. They are ported here (below, after the base
  // cut's own helpers) from TWO Rust references, exactly as the SIR22
  // spec's addendum section describes: `array_runtime::ops::{reduce,scan,
  // outer}` (the three that take an `ElementwiseOpKind` and so reuse
  // `applyOp` above, unchanged) and `apl_runtime::builtins::{shape,reshape,
  // index_generator,index_of,ravel,catenate}` (the "bespoke, not
  // BinOp-shaped" ones — see that Rust file's own module doc comment for
  // why). The TypeScript sibling (`sir-runtime-array`) still does not
  // implement these — that package gaining them is separate, unstarted
  // follow-on work (SIR22 spec, "Backend impact"), not a precondition for
  // this inlined port.
  const ArrayRt = (() => {
    // SECURITY: every factory below validates a shape/output size
    // *before* allocating a `Float64Array` from it — a compiled
    // program's array sizes come from potentially attacker-influenced
    // runtime values (loop counts, parsed input, ...), not fixed
    // compile-time constants, so an unbounded or malformed shape must
    // fail cleanly with a catchable `Error` rather than let
    // `new Float64Array(n)` itself throw an uncaught `RangeError` or
    // stall attempting a huge allocation. Mirrors `matlab-runtime`'s own
    // `MAX_RANGE` bound and the TypeScript sibling's `MAX_ELEMENTS`
    // exactly, so behaviour is identical across both backends.
    const MAX_ELEMENTS = 1 << 26; // 67,108,864

    function checkedShapeSize(shape) {
      if (!shape.every((d) => Number.isInteger(d) && d >= 0)) {
        throw new Error(`checkedShapeSize: shape ${JSON.stringify(shape)} has a negative or non-integer dimension`);
      }
      const n = shape.reduce((acc, d) => acc * d, 1);
      if (!Number.isFinite(n) || n > MAX_ELEMENTS) {
        throw new Error(`checkedShapeSize: shape ${JSON.stringify(shape)} (${n} elements) exceeds the ${MAX_ELEMENTS}-element cap`);
      }
      return n;
    }

    function ndarray(shape, data) {
      if (!(data instanceof Float64Array)) {
        throw new Error("ndarray: data must be a Float64Array");
      }
      const n = checkedShapeSize(shape);
      if (n !== data.length) {
        throw new Error(`ndarray: shape ${JSON.stringify(shape)} implies ${n} elements, got ${data.length}`);
      }
      return { shape, data };
    }

    function fromRows(rows) {
      const nrowsIn = rows.length;
      if (nrowsIn === 0) {
        return ndarray([0, 0], new Float64Array(0));
      }
      const ncolsIn = rows[0].length;
      if (rows.some((r) => r.length !== ncolsIn)) {
        throw new Error("fromRows: ragged rows");
      }
      const n = checkedShapeSize([nrowsIn, ncolsIn]);
      const data = new Float64Array(n);
      for (let r = 0; r < nrowsIn; r++) {
        for (let c = 0; c < ncolsIn; c++) {
          data[c * nrowsIn + r] = rows[r][c]; // column-major store
        }
      }
      return ndarray([nrowsIn, ncolsIn], data);
    }

    /**
     * Coerce a bare JS `number` into a rank-0 (scalar) `NDArray`; an
     * already-`NDArray` value passes through unchanged. Needed because
     * `matlab-to-semantic-ir`'s lowerer emits a mixed operand pair for
     * `.* ./ .\` and for `* /` when exactly one side is scalar (e.g.
     * `A .* 2`) — the *bare* scalar sub-expression is passed through
     * `ElementwiseOp` unwrapped (a plain `IntLit`/`FloatLit`/arithmetic
     * result, which emits as an ordinary JS `number`), not wrapped in an
     * `ArrayLit`/scalar-array constructor first. Every function below
     * that accepts an "array" operand normalizes through this first, so
     * a raw number never reaches `.data`/`.shape` and throws a
     * `TypeError` instead of behaving correctly.
     */
    function toArrayValue(v) {
      // Boundary unwrap: a boxed Float scalar entering the tensor domain
      // becomes a native f64 in the `Float64Array` (tensor internals are
      // untagged native numbers — the tagged-float box lives outside SIR22).
      return isNum(v) ? { shape: [], data: Float64Array.of(numOf(v)) } : v;
    }

    function isScalar(a) { return a.data.length === 1; }

    /** Rows, treating a scalar as `1×1` and a vector `[n]` as `n×1`. */
    function nrows(a) {
      switch (a.shape.length) {
        case 0: return 1;
        default: return a.shape[0];
      }
    }

    /** Columns, treating a scalar as `1×1` and a vector `[n]` as `n×1`. */
    function ncols(a) {
      switch (a.shape.length) {
        case 0:
        case 1: return 1;
        default: return a.shape[1];
      }
    }

    /** Element `(r, c)` (column-major), or `undefined` if out of bounds. */
    function get(a, r, c) {
      if (r >= 0 && c >= 0 && r < nrows(a) && c < ncols(a)) {
        return a.data[c * nrows(a) + r];
      }
      return undefined;
    }

    /**
     * Set element `(r, c)` in place (column-major) — mutates `a.data`
     * directly, matching MATLAB assignment semantics (`A(i,j) = v`
     * rebinds one element of the existing array, it does not produce a
     * new one). This is why `Stmt::IndexSet` is a statement, not a pure
     * expression, in the SIR22 spec.
     */
    function set(a, r, c, value) {
      // SECURITY: written as the negation of `get`'s AND-form
      // (`!(r >= 0 && ...)`), not as an OR-form (`r < 0 || ...`) --
      // under IEEE-754 those are NOT equivalent for NaN: every
      // relational comparison with NaN is false, so an OR-form check
      // would have every branch evaluate false for r=NaN, silently
      // skipping the throw. `a.data[c * nrows(a) + NaN] = value` would
      // then set a stray, non-index property on the Float64Array rather
      // than writing the buffer -- the exact same silent-write-drop bug
      // this file's `resolvePositions`/`assertValidPosition` fix closed
      // for `indexSet`'s call path into this function. `set` itself is
      // not reachable with an unvalidated NaN today (every caller
      // resolves positions through `assertValidPosition` first), but it
      // is part of this module's exported public surface, so it stays
      // NaN-safe on its own rather than relying on every future caller
      // to re-derive that invariant.
      if (!(r >= 0 && c >= 0 && r < nrows(a) && c < ncols(a))) {
        throw new Error(`set: index (${r}, ${c}) out of bounds for shape ${JSON.stringify(a.shape)}`);
      }
      a.data[c * nrows(a) + r] = value;
    }

    // ── elementwise binary ops ────────────────────────────────────
    // Comparisons follow the same APL-style boolean convention
    // `array_runtime::BinOp` uses: `1` for true, `0` for false (never a
    // native `boolean`), since the result must stay a plain array
    // element like every other value here.
    function applyOp(op, a, b) {
      const b2f = (cond) => (cond ? 1 : 0);
      switch (op) {
        case "Add": return a + b;
        case "Sub": return a - b;
        case "Mul": return a * b;
        case "Div": return a / b;
        case "Pow": return Math.pow(a, b);
        case "Max": return Math.max(a, b);
        case "Min": return Math.min(a, b);
        case "Eq": return b2f(a === b);
        case "Ne": return b2f(a !== b);
        case "Lt": return b2f(a < b);
        case "Le": return b2f(a <= b);
        case "Ge": return b2f(a >= b);
        case "Gt": return b2f(a > b);
        default:
          // Same "crosses a JS runtime boundary the emitter can't
          // enforce" reasoning `resolvePositions` below documents: an
          // unrecognised `op` must fail loudly here, not fall through
          // to `undefined`, which would otherwise silently corrupt data
          // as `NaN` instead of erroring.
          throw new Error(`applyOp: unrecognised ElementwiseOpKind ${JSON.stringify(op)}`);
      }
    }

    function sameShape(a, b) {
      return a.length === b.length && a.every((d, i) => d === b[i]);
    }

    /**
     * Elementwise binary op with scalar broadcasting. Either operand may
     * be a scalar; otherwise the shapes must match exactly (full
     * NumPy/MATLAB broadcasting is out of scope, same as the Rust
     * reference). Result takes the non-scalar operand's shape (or the
     * scalar's, if both are).
     */
    function elementwise(op, a, b) {
      a = toArrayValue(a);
      b = toArrayValue(b);
      const ad = a.data;
      const bd = b.data;
      let data;
      if (isScalar(a)) {
        data = Float64Array.from(bd, (y) => applyOp(op, ad[0], y));
      } else if (isScalar(b)) {
        data = Float64Array.from(ad, (x) => applyOp(op, x, bd[0]));
      } else {
        if (!sameShape(a.shape, b.shape)) {
          throw new Error(`elementwise: non-conformable arrays: ${JSON.stringify(a.shape)} vs ${JSON.stringify(b.shape)}`);
        }
        data = new Float64Array(ad.length);
        for (let i = 0; i < data.length; i++) {
          data[i] = applyOp(op, ad[i], bd[i]);
        }
      }
      const shape = isScalar(a) ? b.shape : a.shape;
      return ndarray(shape, data);
    }

    /**
     * Matrix product `[m, k] · [k, n] → [m, n]` (column-major
     * throughout). `m` and `n` come from two *independent* operands
     * (each individually under `MAX_ELEMENTS`, but their product isn't
     * bounded by that alone — an outer-product-shaped call could still
     * ask for a huge output), so `checkedShapeSize` validates `[m, n]`
     * *before* allocating `out`, not after.
     */
    function matmul(a, b) {
      const m = nrows(a);
      const ka = ncols(a);
      const kb = nrows(b);
      const n = ncols(b);
      if (ka !== kb) {
        throw new Error(`matmul: inner dimensions disagree (${m}x${ka} . ${kb}x${n})`);
      }
      const outLen = checkedShapeSize([m, n]);
      const ad = a.data;
      const bd = b.data;
      const out = new Float64Array(outLen);
      for (let j = 0; j < n; j++) {
        for (let i = 0; i < m; i++) {
          let acc = 0;
          for (let p = 0; p < ka; p++) {
            acc += ad[p * m + i] * bd[j * kb + p]; // column-major indexing
          }
          out[j * m + i] = acc;
        }
      }
      return ndarray([m, n], out);
    }

    /**
     * Matrix transpose. `conjugate` distinguishes MATLAB `'` (`true`)
     * from `.'` (`false`) — this runtime has no `Complex` value type yet
     * (matching `array-runtime`'s own real-only scope today), so a
     * conjugate transpose of real data is identical to a plain
     * transpose; `conjugate` is accepted for call-shape parity with the
     * SIR spec only.
     */
    function transpose(a, conjugate) {
      void conjugate;
      const m = nrows(a);
      const n = ncols(a);
      const ad = a.data;
      const out = new Float64Array(ad.length);
      for (let j = 0; j < n; j++) {
        for (let i = 0; i < m; i++) {
          out[i * n + j] = ad[j * m + i];
        }
      }
      return ndarray([n, m], out);
    }

    // ── range ───────────────────────────────────────────────────────
    // Tolerance for the inclusive-stop boundary check, matching
    // `matlab-runtime`'s own `eval_colon` exactly — a floating step
    // (e.g. `1:0.1:2`) can drift a few ULPs short of `stop` by the final
    // iteration, and MATLAB's `a:step:b` is inclusive of `b`.
    const RANGE_EPSILON = 1e-9;

    /**
     * Materialize a MATLAB-style range `start:step:stop` (default
     * `step = 1`) as a `1×n` row vector — MATLAB's `:` always produces
     * a row, never a column. Bounded by `MAX_ELEMENTS` so a compiled
     * program's `1:1e18`-style range can't exhaust memory before this
     * function ever gets to materialize anything.
     */
    function range(start, stop, step = 1) {
      if (step === 0) {
        throw new Error("range: step cannot be zero");
      }
      // SECURITY: the loop condition below is false on its very first
      // check whenever start/stop/step is NaN (every relational
      // comparison with NaN is false), so an unguarded NaN bound would
      // silently produce an empty range instead of erroring -- the same
      // "NaN defeats a comparison-based check" class the linear
      // indexGet/indexSet fix below closes. Reject non-finite bounds
      // up front instead of letting them fall through to a
      // quietly-wrong empty result.
      if (!Number.isFinite(start) || !Number.isFinite(stop) || !Number.isFinite(step)) {
        throw new Error(`range: start/stop/step must be finite numbers, got (${start}, ${stop}, ${step})`);
      }
      const values = [];
      let x = start;
      while ((step > 0 && x <= stop + RANGE_EPSILON) || (step < 0 && x >= stop - RANGE_EPSILON)) {
        if (values.length >= MAX_ELEMENTS) {
          throw new Error(`range: produces more than ${MAX_ELEMENTS} elements`);
        }
        values.push(x);
        x += step;
      }
      return ndarray(
        values.length === 0 ? [1, 0] : [1, values.length],
        Float64Array.from(values),
      );
    }

    // ── indexing ────────────────────────────────────────────────────
    // One MATLAB-style index-position argument, mirroring the SIR22
    // spec's `IndexArg` exactly: `{kind:"scalar",value}` /
    // `{kind:"whole"}` / `{kind:"range",indices: <NDArray>}`. `end`-
    // relative indices are never seen here — per SIR10 discipline, the
    // frontend resolves `end` to a concrete 0-based `scalar` index
    // before emitting `IndexGet`/`IndexSet`.

    /**
     * Validate one resolved position is a real, finite integer.
     *
     * SECURITY: `indexGet`/`indexSet`'s own linear (1-argument) bounds
     * checks are written as `i < 0 || i >= length` — the negation of
     * `get`'s `r >= 0 && r < nrows(a)` AND-form. Under IEEE-754, `NaN`
     * fails *every* relational comparison, so for `i = NaN` **both**
     * halves of that OR are `false`, and the "out of bounds" check is
     * silently skipped entirely — `a.data[NaN]` then reads/writes a
     * stray, non-index `"NaN"` property on the `Float64Array` object
     * rather than the buffer, so a NaN index makes `indexGet` silently
     * return `undefined` (not throw) and makes `indexSet` silently drop
     * the write (not throw, not mutate). This is the exact "malformed
     * input crosses a JS boundary and must fail loudly, not fall
     * through to `undefined`/corrupt data" hazard this file's other
     * `default:` guards (`applyOp`, this function's own `default` arm)
     * already guard against — validating here, once, at the single
     * choke point both `indexGet` and `indexSet` resolve every position
     * through, closes it for both without duplicating a NaN-safe bounds
     * check at every call site.
     */
    function assertValidPosition(i) {
      if (!Number.isInteger(i)) {
        throw new Error(`resolvePositions: index ${i} is not a finite integer`);
      }
      return i;
    }

    /** Resolve one `IndexArg` against a dimension of size `dimSize` into a flat list of 0-based positions along that dimension. */
    function resolvePositions(arg, dimSize) {
      switch (arg.kind) {
        case "scalar": return [assertValidPosition(arg.value)];
        case "whole": return Array.from({ length: dimSize }, (_, i) => i);
        case "range": return Array.from(arg.indices.data, (x) => assertValidPosition(Math.trunc(x)));
        default:
          // Emitted code crosses a JS runtime boundary the emitter can't
          // enforce at the actual call site — a malformed `kind` must
          // fail cleanly here, not fall through to `undefined` and
          // surface as a confusing `TypeError` several calls further down.
          throw new Error(`resolvePositions: unrecognised IndexArg ${JSON.stringify(arg)}`);
      }
    }

    /**
     * `A(i)` / `A(i, j)` — read one element or a sub-array. Scoped to 1
     * or 2 index arguments (rank ≤ 2): a single argument indexes `a`'s
     * underlying column-major data linearly (MATLAB's own single-
     * subscript convention, which is column-major too); two arguments
     * index `(row, col)`. Returns a bare `number` when every argument is
     * `scalar` (a single element), otherwise an `NDArray`.
     */
    function indexGet(a, indices) {
      if (indices.length === 1) {
        const [arg] = indices;
        const positions = resolvePositions(arg, a.data.length);
        const read = (i) => {
          if (i < 0 || i >= a.data.length) {
            throw new Error(`indexGet: linear index ${i} out of bounds`);
          }
          return a.data[i];
        };
        if (arg.kind === "scalar") {
          return read(positions[0]);
        }
        return ndarray([1, positions.length], Float64Array.from(positions, read));
      }
      if (indices.length === 2) {
        const [rowArg, colArg] = indices;
        const rows = resolvePositions(rowArg, nrows(a));
        const cols = resolvePositions(colArg, ncols(a));
        const read = (r, c) => {
          const v = get(a, r, c);
          if (v === undefined) {
            throw new Error(`indexGet: (${r}, ${c}) out of bounds for shape ${JSON.stringify(a.shape)}`);
          }
          return v;
        };
        if (rowArg.kind === "scalar" && colArg.kind === "scalar") {
          return read(rows[0], cols[0]);
        }
        // `rows.length`/`cols.length` are each individually bounded by
        // `a`'s own dimensions (`whole`) or by a `range` NDArray's own
        // `MAX_ELEMENTS` cap — but nothing bounds their *product* on its
        // own, so this is the exact outer-product-shaped allocation
        // `matmul` guards against, one level up. Validate before
        // allocating, not after.
        const outLen = checkedShapeSize([rows.length, cols.length]);
        const data = new Float64Array(outLen);
        for (let c = 0; c < cols.length; c++) {
          for (let r = 0; r < rows.length; r++) {
            data[c * rows.length + r] = read(rows[r], cols[c]);
          }
        }
        return ndarray([rows.length, cols.length], data);
      }
      throw new Error(`indexGet: only 1 or 2 index arguments are supported (rank <= 2 scope), got ${indices.length}`);
    }

    /** Broadcast a scalar-or-`NDArray` right-hand side to exactly `count` values (mirrors `elementwise`'s scalar-broadcast rule). */
    function broadcastValues(value, count) {
      if (isNum(value)) {
        return new Float64Array(count).fill(numOf(value));
      }
      if (value.data.length === 1) {
        return new Float64Array(count).fill(value.data[0]);
      }
      if (value.data.length !== count) {
        throw new Error(`indexSet: value has ${value.data.length} elements, expected ${count}`);
      }
      return value.data;
    }

    /**
     * `A(i) = v` / `A(i, j) = v` — write one element or a sub-array, IN
     * PLACE (see `set`'s doc comment above for why this mutates rather
     * than returns a new array). `value` may be a scalar (broadcast to
     * every selected position) or an `NDArray` with exactly as many
     * elements as positions are selected.
     */
    function indexSet(a, indices, value) {
      if (indices.length === 1) {
        const [arg] = indices;
        const positions = resolvePositions(arg, a.data.length);
        const values = broadcastValues(value, positions.length);
        positions.forEach((i, k) => {
          if (i < 0 || i >= a.data.length) {
            throw new Error(`indexSet: linear index ${i} out of bounds`);
          }
          a.data[i] = values[k];
        });
        return;
      }
      if (indices.length === 2) {
        const [rowArg, colArg] = indices;
        const rows = resolvePositions(rowArg, nrows(a));
        const cols = resolvePositions(colArg, ncols(a));
        // Same product-of-two-independent-selections gap `indexGet`
        // closes above — validate before `broadcastValues` allocates.
        const count = checkedShapeSize([rows.length, cols.length]);
        const values = broadcastValues(value, count);
        let k = 0;
        for (let c = 0; c < cols.length; c++) {
          for (let r = 0; r < rows.length; r++) {
            set(a, rows[r], cols[c], values[k]);
            k++;
          }
        }
        return;
      }
      throw new Error(`indexSet: only 1 or 2 index arguments are supported (rank <= 2 scope), got ${indices.length}`);
    }

    // ── SIR22 addendum: APL primitive operators ────────────────────
    // `Reduce`/`Scan`/`OuterProduct` reuse `applyOp` above (the same
    // dispatch table `elementwise` uses) — see this section's own module
    // doc comment for the two Rust references every function below ports
    // 1:1. `MAX_ELEMENTS` (defined above) is reused as-is for every new
    // bounded-allocation check here — this file has exactly one array-size
    // cap, not one per domain, so `⍳`/dyadic `⍴`/`⍳` (index-of)/`,`
    // (catenate) share it with `matmul`/`range`/`indexGet` rather than
    // reintroducing `apl_runtime::builtins::MAX_ARRAY_LENGTH`'s smaller
    // 1,000,000 figure as a second, competing constant.

    /**
     * `+/A` (APL reduce, dyadic-op monadic-adverb) — fold `target` with
     * `op` along its one axis. Ported 1:1 from `array_runtime::ops::
     * reduce`:
     * - rank 0 (scalar): nothing to fold, returns `target` itself.
     * - rank 1 (vector `[n]`): left-fold across all `n` elements
     *   (`op(op(op(v0, v1), v2), …)`); an EMPTY vector is a clean error —
     *   unlike `sum`/`mean` (which have a built-in identity, 0), `reduce`
     *   is generic over any `op`, and guessing an identity (is it `0` for
     *   `Add`, `1` for `Mul`, `-Infinity` for `Max`?) for an arbitrary,
     *   possibly-future op would be silently wrong for most of them.
     * - rank 2 (matrix `[r, c]`): folds EACH ROW independently across its
     *   `c` columns, producing a `[r]` vector (one folded value per row).
     *   Column-major storage means element `(row, col)` lives at
     *   `col * r + row` — the row loop reads `d[row]` as the seed (column
     *   0) then walks `d[col * r + row]` for `col = 1..c`; getting `row`
     *   and `col` swapped here silently transposes the result instead of
     *   throwing, so this indexing is the single easiest place to
     *   introduce a wrong-answer bug when reading this function.
     */
    function reduce(op, a) {
      a = toArrayValue(a);
      const shape = a.shape;
      if (shape.length === 0) {
        return a;
      }
      if (shape.length === 1) {
        const n = shape[0];
        if (n === 0) {
          throw new Error("reduce: cannot fold an empty vector (no identity element for an arbitrary op)");
        }
        const d = a.data;
        let acc = d[0];
        for (let i = 1; i < n; i++) {
          acc = applyOp(op, acc, d[i]);
        }
        return ndarray([], Float64Array.of(acc));
      }
      if (shape.length === 2) {
        const [r, c] = shape;
        if (c === 0) {
          throw new Error("reduce: cannot fold an empty row (no identity element for an arbitrary op)");
        }
        const d = a.data;
        const out = new Float64Array(r);
        for (let row = 0; row < r; row++) {
          let acc = d[row]; // column-major: (row, 0) lives at plain `row`
          for (let col = 1; col < c; col++) {
            acc = applyOp(op, acc, d[col * r + row]);
          }
          out[row] = acc;
        }
        return ndarray([r], out);
      }
      throw new Error(`reduce: rank > 2 not yet supported (shape ${JSON.stringify(shape)})`);
    }

    /**
     * `+\A` (APL scan) — the same fold as `reduce`, but keeping EVERY
     * intermediate result instead of only the last; output has the same
     * shape as `target`. Ported 1:1 from `array_runtime::ops::scan`. An
     * empty axis is NOT an error here (unlike `reduce`): there is simply
     * nothing to scan, and the (empty) output shape already says so.
     */
    function scan(op, a) {
      a = toArrayValue(a);
      const shape = a.shape;
      if (shape.length === 0) {
        return a;
      }
      if (shape.length === 1) {
        const n = shape[0];
        const d = a.data;
        const out = new Float64Array(n);
        let acc;
        let started = false;
        for (let i = 0; i < n; i++) {
          acc = started ? applyOp(op, acc, d[i]) : d[i];
          started = true;
          out[i] = acc;
        }
        return ndarray([n], out);
      }
      if (shape.length === 2) {
        const [r, c] = shape;
        const d = a.data;
        const out = new Float64Array(d.length);
        for (let row = 0; row < r; row++) {
          let acc;
          let started = false;
          for (let col = 0; col < c; col++) {
            const x = d[col * r + row]; // column-major
            acc = started ? applyOp(op, acc, x) : x;
            started = true;
            out[col * r + row] = acc;
          }
        }
        return ndarray([r, c], out);
      }
      throw new Error(`scan: rank > 2 not yet supported (shape ${JSON.stringify(shape)})`);
    }

    /**
     * `A∘.×B` (APL outer product) — apply `op` to every pair `(aᵢ, bⱼ)`,
     * producing a result of rank `rank(a) + rank(b)`. Ported 1:1 from
     * `array_runtime::ops::outer`, scoped identically to `rank(a) <= 1`
     * and `rank(b) <= 1` (the vector⊗vector case below already reaches
     * this domain's rank-2 ceiling). `checkedShapeSize` validates the
     * `[m, n]` output shape *before* allocating — `m`/`n` are two
     * INDEPENDENT operand lengths, each individually under
     * `MAX_ELEMENTS`, but nothing bounds their product alone (the same
     * outer-product-shaped allocation `matmul`/`indexGet` above guard).
     */
    function outer(op, a, b) {
      a = toArrayValue(a);
      b = toArrayValue(b);
      const as = a.shape;
      const bs = b.shape;
      if (as.length === 0 && bs.length === 0) {
        return ndarray([], Float64Array.of(applyOp(op, a.data[0], b.data[0])));
      }
      if (as.length === 0 && bs.length === 1) {
        const x = a.data[0];
        return ndarray([bs[0]], Float64Array.from(b.data, (y) => applyOp(op, x, y)));
      }
      if (as.length === 1 && bs.length === 0) {
        const y = b.data[0];
        return ndarray([as[0]], Float64Array.from(a.data, (x) => applyOp(op, x, y)));
      }
      if (as.length === 1 && bs.length === 1) {
        const m = as[0];
        const n = bs[0];
        const outLen = checkedShapeSize([m, n]);
        const ad = a.data;
        const bd = b.data;
        const out = new Float64Array(outLen);
        for (let j = 0; j < n; j++) {
          for (let i = 0; i < m; i++) {
            out[j * m + i] = applyOp(op, ad[i], bd[j]); // column-major
          }
        }
        return ndarray([m, n], out);
      }
      throw new Error(`outer: operands of rank > 1 not yet supported (shapes ${JSON.stringify(as)}, ${JSON.stringify(bs)})`);
    }

    /**
     * Flatten (rank <= 2, this domain's ceiling) `a` to ROW-major order —
     * last axis varies fastest. `a` itself stores COLUMN-major (`get`'s
     * own doc comment), so a matrix must be walked "row, then column" via
     * `get` to produce true row-major order; returning the raw
     * column-major buffer would silently ravel in the WRONG order. Always
     * returns a fresh `Float64Array` (never `a.data` itself, even in the
     * rank <= 1 no-op case) — mirrors `apl_runtime::builtins::flatten`
     * returning an owned `Vec`, not a borrow, so the result never
     * accidentally aliases `a`'s own buffer.
     */
    function flattenRowMajor(a) {
      const shape = a.shape;
      if (shape.length <= 1) {
        return Float64Array.from(a.data);
      }
      if (shape.length === 2) {
        const [r, c] = shape;
        const out = new Float64Array(r * c);
        let k = 0;
        for (let row = 0; row < r; row++) {
          for (let col = 0; col < c; col++) {
            out[k++] = get(a, row, col);
          }
        }
        return out;
      }
      // Unreachable in practice (this domain's rank <= 2 ceiling) -- total
      // rather than throwing, mirroring the Rust reference's own fallback.
      return Float64Array.from(a.data);
    }

    /**
     * Monadic `⍴` (shape-of) — `target`'s dimensions as a vector. Ported
     * 1:1 from `apl_runtime::builtins::shape`: a SCALAR has zero
     * dimensions, so its shape is the EMPTY vector (not a scalar!) — `⍴5`
     * is `⍳0`-shaped, a length-0 vector, mirroring `shape.length === 0`
     * exactly. A vector `[n]` has shape `[n]` (one element); a matrix
     * `[r, c]` has shape `[r, c]` (two elements).
     */
    function shape(a) {
      a = toArrayValue(a);
      const dims = Float64Array.from(a.shape);
      return ndarray([dims.length], dims);
    }

    /**
     * Dyadic `⍴` (reshape) — reinterpret `target`'s data under the new
     * dimensions `shapeArg`. Ported 1:1 from `apl_runtime::builtins::
     * reshape`. `shapeArg` must itself be a scalar or vector (rank <= 1)
     * of non-negative integers, and is itself capped at rank <= 2 (this
     * domain's ceiling — a longer target shape is a clean error, not a
     * silent truncation). `target`'s elements are ravelled
     * (`flattenRowMajor`) then cyclically repeated or truncated to fill
     * the target shape's element count.
     *
     * CRITICAL: the cyclic fill happens in ROW-major order (APL's reshape
     * fills the LAST axis fastest, same convention as ravel), but this
     * domain's storage is COLUMN-major — so for a rank-2 target the
     * row-major `filled` sequence must be TRANSPOSED into column-major
     * storage (`data[col * r + row] = filled[row * c + col]`) before
     * calling `ndarray`. Handing `filled` straight to `ndarray` would
     * silently reshape column-major instead of APL's row-major
     * convention — a wrong answer that still LOOKS plausible (right
     * multiset of values, wrong positions).
     */
    function reshape(shapeArg, target) {
      shapeArg = toArrayValue(shapeArg);
      target = toArrayValue(target);
      if (shapeArg.shape.length > 1) {
        throw new Error(`reshape: shape argument must be a scalar or vector (got rank ${shapeArg.shape.length})`);
      }
      const dims = Array.from(shapeArg.data, (x) => {
        if (!(Number.isInteger(x) && x >= 0)) {
          throw new Error(`reshape: shape elements must be non-negative integers, got ${x}`);
        }
        return x;
      });
      if (dims.length > 2) {
        throw new Error(`reshape: reshape to rank > 2 is not yet supported (target shape ${JSON.stringify(dims)})`);
      }
      const total = checkedShapeSize(dims);
      const source = flattenRowMajor(target);
      if (total > 0 && source.length === 0) {
        throw new Error("reshape: cannot reshape an empty source into a non-empty shape");
      }
      const filled = new Float64Array(total);
      for (let k = 0; k < total; k++) {
        filled[k] = source[k % source.length];
      }
      if (dims.length <= 1) {
        return ndarray(dims, filled);
      }
      const [r, c] = dims;
      const data = new Float64Array(total);
      for (let row = 0; row < r; row++) {
        for (let col = 0; col < c; col++) {
          data[col * r + row] = filled[row * c + col];
        }
      }
      return ndarray(dims, data);
    }

    /**
     * Monadic `⍳` (index generator / iota) — `⍳n` is the 1-BASED vector
     * `[1, 2, …, n]`. Ported 1:1 from `apl_runtime::builtins::
     * index_generator` — note this is 1-based, unlike every 0-based index
     * elsewhere in this domain (`indexGet`/`indexSet`), because that is
     * genuinely what APL's `⍳` means at the SURFACE-SYNTAX level (the
     * `Expr::IndexGenerator` doc comment in `semantic-ir`'s `nodes.rs`
     * makes the same point). `checkedShapeSize([n])` both validates `n`
     * is a non-negative integer AND caps it at `MAX_ELEMENTS` before
     * allocating — `n` is a runtime value a compiled program computes,
     * not a fixed constant, so `⍳` of an absurd size must fail cleanly.
     */
    function indexGenerator(a) {
      a = toArrayValue(a);
      if (!isScalar(a)) {
        throw new Error("indexGenerator: monadic argument must be a scalar");
      }
      const x = a.data[0];
      if (!(Number.isInteger(x) && x >= 0)) {
        throw new Error(`indexGenerator: monadic argument must be a non-negative integer, got ${x}`);
      }
      const n = checkedShapeSize([x]);
      const out = new Float64Array(n);
      for (let i = 0; i < n; i++) {
        out[i] = i + 1;
      }
      return ndarray([n], out);
    }

    /**
     * Dyadic `⍳` (index-of / search) — for every element of `needle`, the
     * 1-based index of its first occurrence in the vector `haystack` (or
     * `haystack.length + 1` if not found — "not found" is a valid,
     * always-in-range position, not `-1`/`undefined`). Ported 1:1 from
     * `apl_runtime::builtins::index_of`: plain EXACT equality (no
     * floating-point tolerance — `Float64Array.prototype.indexOf` already
     * uses strict `===`, so `NaN` correctly never matches, same as Rust's
     * `==`). The work done is O(len(haystack) * len(needle)) (a full
     * linear scan per needle element) — `checkedShapeSize` is reused here
     * purely for its "product <= MAX_ELEMENTS" check (both lengths are
     * already valid non-negative integers, so its dimension-validity half
     * is a no-op) to cap the PRODUCT before scanning, since each operand
     * individually staying under `MAX_ELEMENTS` does not bound their
     * product (up to ~4.5 * 10^15 comparisons otherwise).
     */
    function indexOf(a, b) {
      a = toArrayValue(a);
      b = toArrayValue(b);
      if (a.shape.length > 1) {
        throw new Error(`indexOf: left argument must be a scalar or vector (got rank ${a.shape.length})`);
      }
      checkedShapeSize([a.data.length, b.data.length]);
      const haystack = a.data;
      const out = Float64Array.from(b.data, (needle) => {
        const idx = haystack.indexOf(needle);
        return idx === -1 ? haystack.length + 1 : idx + 1;
      });
      return ndarray(b.shape, out);
    }

    /**
     * Monadic `,` (ravel) — flatten `target` to a rank-1 vector, in
     * row-major order (see `flattenRowMajor`'s own doc comment for the
     * column-major-storage-vs-row-major-order subtlety). Ported 1:1 from
     * `apl_runtime::builtins::ravel`.
     */
    function ravel(a) {
      a = toArrayValue(a);
      const flat = flattenRowMajor(a);
      return ndarray([flat.length], flat);
    }

    /**
     * Dyadic `,` (catenate) — supports scalar-scalar, scalar-vector,
     * vector-scalar, vector-vector (all producing a vector), and
     * matrix-matrix-with-equal-row-counts (column/last-axis catenate,
     * producing `[r, ca + cb]`). Any other rank combination is a clean
     * "not yet supported" error. Ported 1:1 from `apl_runtime::builtins::
     * catenate`. The combined-length cap check happens ONCE, up front,
     * regardless of which rank combination follows (mirroring the Rust
     * reference's own structure) — neither operand alone need be
     * oversized for the RESULT to be, since a script that repeatedly
     * catenates a value with itself (`A←A,A`) doubles the size every line
     * with no other ceiling.
     */
    function catenate(a, b) {
      a = toArrayValue(a);
      b = toArrayValue(b);
      checkedShapeSize([a.data.length + b.data.length]);
      const ra = a.shape.length;
      const rb = b.shape.length;
      if (ra === 0 && rb === 0) {
        return ndarray([2], Float64Array.of(a.data[0], b.data[0]));
      }
      if (ra === 0 && rb === 1) {
        const out = new Float64Array(1 + b.data.length);
        out[0] = a.data[0];
        out.set(b.data, 1);
        return ndarray([out.length], out);
      }
      if (ra === 1 && rb === 0) {
        const out = new Float64Array(a.data.length + 1);
        out.set(a.data, 0);
        out[a.data.length] = b.data[0];
        return ndarray([out.length], out);
      }
      if (ra === 1 && rb === 1) {
        const out = new Float64Array(a.data.length + b.data.length);
        out.set(a.data, 0);
        out.set(b.data, a.data.length);
        return ndarray([out.length], out);
      }
      if (ra === 2 && rb === 2) {
        const r = nrows(a);
        if (r !== nrows(b)) {
          throw new Error(`catenate: matrix catenate needs equal row counts (${r} vs ${nrows(b)})`);
        }
        const ca = ncols(a);
        const cb = ncols(b);
        const outLen = checkedShapeSize([r, ca + cb]);
        const data = new Float64Array(outLen);
        for (let row = 0; row < r; row++) {
          for (let col = 0; col < ca; col++) {
            data[col * r + row] = get(a, row, col);
          }
          for (let col = 0; col < cb; col++) {
            data[(ca + col) * r + row] = get(b, row, col);
          }
        }
        return ndarray([r, ca + cb], data);
      }
      throw new Error(`catenate: catenate of rank ${ra} and rank ${rb} is not yet supported`);
    }

    /**
     * Format one number the way `apl_runtime::value::fmt_num` does (ported
     * 1:1): the high-minus glyph `¯` (never ASCII `-`) prefixes a negative
     * number; a whole-valued float prints without a trailing `.0`. Unlike
     * the Rust source, no separate integer-vs-float branch is needed for
     * the whole-value case — `String(5)` and `String(5.0)` are both `"5"`
     * in JS, where Rust needs `format!("{}", mag as i64)` specifically to
     * avoid `5.0`'s `Display` impl printing a trailing `.0`. `x < 0` (a
     * numeric comparison) already excludes `-0` from the high-minus branch
     * on its own — `-0 < 0` is `false` in JS — so, unlike Rust's
     * `is_sign_negative()` (a bit-level check that says `true` for `-0`),
     * no separate `-0`-is-plain-`0` guard is needed here either.
     */
    function fmtNum(x) {
      if (Number.isNaN(x)) {
        return "NaN";
      }
      if (!Number.isFinite(x)) {
        return x < 0 ? "¯∞" : "∞";
      }
      const body = String(Math.abs(x));
      return x < 0 ? "¯" + body : body;
    }

    /**
     * Render `a` the way an APL session echoes a bare (auto-printed)
     * result — ported 1:1 from `apl_runtime::value::display`. This is
     * APL's OWN display convention (high-minus negatives, no name/`ans=`
     * prefix), distinct from MATLAB's own `Array` `Display` impl (never
     * reached from this backend — MATLAB always reads a computed array
     * back through a scalar `IndexGet` instead, see `formatSeen`'s call
     * site above).
     *
     * - rank 0 (scalar): the one number.
     * - rank 1 (vector): elements, space-separated, on one line (the
     *   empty vector prints as the empty string — an APL session shows a
     *   blank line for `⍳0`, `⍴5`, etc.).
     * - rank 2 (matrix): one row per line, elements space-separated and
     *   right-aligned to the widest cell's width IN THIS DISPLAY.
     */
    function display(a) {
      const shape = a.shape;
      if (shape.length === 0) {
        return fmtNum(a.data[0]);
      }
      if (shape.length === 1) {
        const n = shape[0];
        if (n === 0) {
          return "";
        }
        return Array.from(a.data, fmtNum).join(" ");
      }
      if (shape.length === 2) {
        const [r, c] = shape;
        // Formatted once, up front (in the array's own column-major
        // storage order), so the alignment width is independent of
        // row/column traversal order -- only the WIDEST cell matters, and
        // order doesn't affect a max().
        const width = Array.from(a.data, fmtNum).reduce((w, s) => Math.max(w, s.length), 1);
        const lines = [];
        for (let row = 0; row < r; row++) {
          const rowCells = [];
          for (let col = 0; col < c; col++) {
            rowCells.push(fmtNum(get(a, row, col)).padStart(width, " "));
          }
          lines.push(rowCells.join(" "));
        }
        return lines.join("\n");
      }
      // Unreachable in practice (this domain's rank <= 2 ceiling) --
      // render something total rather than throwing, mirroring the Rust
      // reference's own `_ => format!("{a}")` fallback.
      return String(Array.from(a.data));
    }

    return {
      ndarray, fromRows, isScalar, nrows, ncols, get, set,
      elementwise, matmul, transpose, range, indexGet, indexSet,
      // SIR22 addendum (APL primitives).
      reduce, scan, outer, shape, reshape, indexGenerator, indexOf, ravel,
      catenate, display,
      // Exported so `formatSeen` (defined outside this IIFE, near the top
      // of the file) can render a bare/boxed scalar through the SAME
      // high-minus-aware number formatter a raw NDArray already uses via
      // `display` -- see `SIR_DISPLAY_APL_HIGH_MINUS`'s own comment.
      fmtNum,
    };
  })();

  return {
    Sym, Pair, Closure,
    intern, applyClosure, truthy, matlabTruthy, format, print, puts,
    plus, times, divide,
    // Tagged floats (Ruby Integer vs Float). Exported so the emitter can
    // mint a boxed float at a `FloatLit` and route `-`/`%`/`neg` through
    // the re-tagging helpers. `mkFloat` is the sole factory.
    SirFloat, mkFloat, numOf, isNum, isFloat, neg, minus, mod, floatToRubyString,
    eq, ne, lt, gt, le, ge,
    builtins, builtinClosure, callBuiltin, callMethod,
    SirError, raiseError, rescueMatches, registerAncestry,
    // OOP (O3): instantiation, method definition + dispatch, super,
    // the self stack, and instance/class-variable access.
    SirInstance, newInstance, callNew, callSuper,
    defMethod, defClassMethod, currentSelf,
    ivarGet, ivarSet, cvarGet, cvarSet,
    // Mixins (MX4): include/extend registration + class-method dispatch.
    includeModule, extendModule, callClassMethod,
    // SIR23: symbolic expression + pattern/rewrite domain, ported from
    // the published sir-runtime-symbolic/symbolic-ir/cas-pattern-matching
    // TypeScript packages so this backend stays self-contained.
    Symbolic,
    // SIR22: array/matrix domain, ported from the published
    // sir-runtime-array TypeScript package so this backend stays
    // self-contained. Exposed as `Array` (a property key, not a `const`
    // binding — this never shadows the global `Array` constructor
    // anywhere in this file).
    Array: ArrayRt,
  };
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_is_non_empty_and_terminates_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_defines_the_namespace_inline() {
        // Self-contained: the `__Sir` namespace is the IIFE itself, not
        // an import of an external package.
        assert!(RUNTIME.contains("const __Sir = (() => {"));
        assert!(!RUNTIME.contains("import "));
        assert!(!RUNTIME.contains("require("));
    }

    #[test]
    fn runtime_exports_the_helpers_the_emitter_calls() {
        for needed in [
            "intern",
            "applyClosure",
            "truthy",
            "format",
            "builtins",
            "builtinClosure",
            "callBuiltin",
            "callMethod",
            "class Sym",
            "class Pair",
            "class Closure",
            // Exception runtime (SIR17): the four helpers the emitter
            // references from its TryCatch / raise / ClassDef arms.
            "class SirError",
            "raiseError",
            "rescueMatches",
            "registerAncestry",
            // OOP runtime (O3): the helpers the emitter references from
            // its __new__ / __super__ / __def_method__ / @ivar arms.
            "class SirInstance",
            "callNew",
            "callSuper",
            "defMethod",
            "defClassMethod",
            "currentSelf",
            "ivarGet",
            "ivarSet",
            "cvarGet",
            "cvarSet",
            // Mixins (MX4): include/extend + class-method dispatch.
            "includeModule",
            "extendModule",
            "callClassMethod",
            // Tagged floats (Ruby Integer vs Float): the boxed-float
            // factory + tag helpers the emitter mints/routes through.
            "class SirFloat",
            "mkFloat",
            "numOf",
            "isNum",
            "isFloat",
            "floatToRubyString",
            // Re-tagging arithmetic + `numOf`-unwrapping comparisons the
            // emitter routes `-`/`%`/`neg` and `=`/`!=`/`<`/`>`/`<=`/`>=` through.
            "minus",
            "mod",
            "neg",
            "eq",
            "ne",
            "lt",
            "gt",
            "le",
            "ge",
        ] {
            assert!(RUNTIME.contains(needed), "runtime missing `{needed}`");
        }
    }

    #[test]
    fn runtime_aliases_ruby_string_method_names_to_native() {
        // Ruby spellings that differ from JS natives must normalise to a native
        // name that is ITSELF on the allowlist, so the security gate is unchanged
        // while `"x".upcase` etc. dispatch.  (Regression guard for the gap where
        // `upcase`/`downcase`/`strip` raised NoMethodError on the JS backend.)
        assert!(RUNTIME.contains("const RUBY_METHOD_ALIASES"));
        for (ruby, native) in [
            ("upcase", "toUpperCase"),
            ("downcase", "toLowerCase"),
            ("strip", "trim"),
            ("start_with?", "startsWith"),
            ("include?", "includes"),
        ] {
            assert!(
                RUNTIME.contains(&format!("\"{ruby}\": \"{native}\"")),
                "runtime missing Ruby→native alias `{ruby}`→`{native}`"
            );
            // The native target must be on the allowlist — the alias only
            // normalises the spelling; it never widens the gate.
            assert!(
                RUNTIME.contains(&format!("\"{native}\"")),
                "alias target `{native}` must be an allowlisted native method"
            );
        }
        // The alias is resolved BEFORE the allowlist check (not bypassing it).
        assert!(RUNTIME.contains("METHOD_ALLOWLIST.has(native)"));
    }

    #[test]
    fn oop_dispatch_is_map_keyed_not_reflection() {
        // SECURITY (O3): the method tables are real `Map`s keyed on a
        // NUL-joined `(class, method)` string — never `recv[name]`,
        // `eval`, or `new Function` on a source-derived name.  A class /
        // method named `constructor` or `__proto__` is therefore inert
        // data (a Map miss), not a reflective gadget.
        assert!(RUNTIME.contains("const methodTable = new Map();"));
        assert!(RUNTIME.contains(r#"return cls + "\x00" + name;"#));
        assert!(RUNTIME.contains("ownerTable.get(methodKey(owner, name))"));
        assert!(RUNTIME.contains("methodTable.get(methodKey(mod, name))"));
        // No dynamic-code gadget is ever *invoked*: `new Function(` and
        // `eval(` as calls appear nowhere in the runtime (the phrase
        // "new Function" occurs only in the SECURITY comment prose, so we
        // match the call form `new Function(` to avoid a false positive).
        assert!(!RUNTIME.contains("new Function("));
        assert!(!RUNTIME.contains("eval("));
        // The ivar / instance bags are prototype-less, so a `"__proto__"`
        // name is data and cannot poison a prototype chain.
        assert!(RUNTIME.contains("this.ivars = Object.create(null);"));
        // MX4: the per-owner mixin lists are real `Map`s (not `{}`), keyed
        // by the owner NAME string and holding module NAME strings — so a
        // module/owner named `__proto__` is inert data, never a prototype
        // write, matching the method-table bar.
        assert!(RUNTIME.contains("const includedModules = new Map();"));
        assert!(RUNTIME.contains("const extendedModules = new Map();"));
    }

    #[test]
    fn oop_self_stack_unwinds_in_finally() {
        // The self-stack is balanced with try/finally so an exception
        // thrown mid-method still pops `self` (no stale binding leaks).
        assert!(RUNTIME.contains("pushSelf(recv);"));
        assert!(RUNTIME.contains("popSelf();"));
        assert!(RUNTIME.contains("} finally {"));
    }

    #[test]
    fn runtime_bakes_in_builtin_exception_ancestry() {
        // `rescue StandardError` must catch the everyday subclasses, so the
        // built-in ancestry table has to chain them up to StandardError.
        assert!(RUNTIME.contains("ArgumentError: \"StandardError\""));
        assert!(RUNTIME.contains("StandardError: \"Exception\""));
    }

    #[test]
    fn runtime_defines_polymorphic_plus_times_helpers() {
        // PO3: `+`/`*` are type-polymorphic (numeric / String / Array), so
        // the runtime must define and export the dispatch helpers the
        // emitter now calls for the 2-arg form.
        assert!(RUNTIME.contains("function plus("));
        assert!(RUNTIME.contains("function times("));
        assert!(RUNTIME.contains("plus, times,"), "helpers must be exported");
        // Dispatch is a runtime TAG test, never reflection.
        assert!(RUNTIME.contains(r#"typeof first === "string""#));
        assert!(RUNTIME.contains("Array.isArray(first)"));
        // SECURITY: the repeat arms are bounded — an oversized product
        // raises a Ruby-shaped ArgumentError rather than OOMing / throwing
        // a raw RangeError.
        assert!(RUNTIME.contains(r#"raiseError("ArgumentError", "argument too big")"#));
        assert!(RUNTIME.contains("Number.MAX_SAFE_INTEGER"));
    }

    #[test]
    fn runtime_typed_errors_divide_fetch_unknown_method() {
        // T3 (sir-typed-runtime-errors): the faulting runtime ops raise the
        // CORRECT typed SirError, matching Ruby.

        // Division by zero → ZeroDivisionError ("divided by 0").  The helper
        // adds the check native JS `/` lacks (it yields Infinity).
        assert!(RUNTIME.contains("function divide(a, b)"));
        assert!(RUNTIME.contains(r#"raiseError("ZeroDivisionError", "divided by 0")"#));
        assert!(
            RUNTIME.contains("plus, times, divide,"),
            "divide must be exported"
        );

        // `.fetch` raises typed errors: IndexError for a sequence OOB,
        // KeyError for a missing hash key (no default).
        assert!(RUNTIME.contains(r#"if (name === "fetch")"#));
        assert!(RUNTIME.contains(r#"raiseError("IndexError","#));
        assert!(RUNTIME.contains(r#"raiseError("KeyError", "key not found: ""#));

        // An unknown method raises NoMethodError (not a JS-native TypeError).
        assert!(RUNTIME.contains(r#"raiseError("NoMethodError","#));
        assert!(
            RUNTIME.contains(r#""undefined method `" + name + "` for " + classDescription(recv)"#)
        );
        assert!(RUNTIME.contains("function classDescription(recv)"));
        // The old JS-native TypeError floor for the allowlist miss is gone.
        assert!(!RUNTIME.contains("is not an allowed collection method"));
    }

    #[test]
    fn runtime_defines_m6_universal_metaprogramming_surface() {
        // M6: send/tap/then/yield_self/respond_to? + boolean &/|/^ are mixed
        // into EVERY receiver, ported to match the Python/TS references.
        assert!(RUNTIME
            .contains(r#"const SEND_METHODS = new Set(["send", "__send__", "public_send"]);"#));
        assert!(RUNTIME
            .contains(r#"const OBJECT_BLOCK_METHODS = new Set(["tap", "then", "yield_self"]);"#));
        assert!(RUNTIME.contains(r#"const BOOL_METHODS = new Set(["&", "|", "^"]);"#));
        assert!(RUNTIME.contains("function objectMetaMethod("));
        assert!(RUNTIME.contains("function respondsTo("));
        assert!(RUNTIME.contains("function boolMethod("));
        // `tap` returns the receiver; `then`/`yield_self` return the block result.
        assert!(
            RUNTIME.contains(r#"if (name === "tap") { applyClosure(last, [recv]); return recv; }"#)
        );
        assert!(RUNTIME.contains("return applyClosure(last, [recv]); // then / yield_self"));

        // SECURITY (the C3 RCE lesson): `send` routes the DYNAMIC name back
        // through `callMethod` — the SAME allowlist / method-table gate a direct
        // call uses — NEVER `recv[name]` / `eval` / `new Function` on the name.
        assert!(RUNTIME
            .contains("return callMethod(recv, methodNameArg(rawArgs[0]), ...rawArgs.slice(1));"));
        assert!(!RUNTIME.contains("new Function("));
        assert!(!RUNTIME.contains("eval("));
        // `respond_to?` checks the same tables dispatch uses (method table for a
        // SirInstance, the allowlist for a primitive) — not a probe of recv[name].
        assert!(RUNTIME.contains(
            "resolveMethod(methodTable, recv.sirClass, name, includedModules) !== undefined"
        ));
        assert!(RUNTIME.contains("METHOD_ALLOWLIST.has(native)"));
    }

    #[test]
    fn runtime_dispatches_ancestry_by_table_not_reflection() {
        // SECURITY: ancestry lookup is an explicit map read, never `eval`
        // or dynamic code synthesis.  A prototype-less map keeps a user
        // class named `constructor`/`__proto__` from poisoning the lookup.
        assert!(RUNTIME.contains("ancestry[cur]"));
        assert!(RUNTIME.contains("Object.create(null)"));
        assert!(!RUNTIME.contains("eval("));
    }

    #[test]
    fn runtime_defines_symbolic_expression_domain() {
        // SIR23: the emitter's Sym* arms all call into `Symbolic.*` — this
        // must stay inlined (no import/require), matching every other
        // domain in this file.
        assert!(RUNTIME.contains("const Symbolic = (() => {"));
        assert!(RUNTIME.contains("Symbolic,"), "Symbolic must be exported");
        for needed in [
            "function symTerm(",
            "function intTerm(",
            "function rationalTerm(",
            "function floatTerm(",
            "function stringTerm(",
            "function applyTerm(",
            "function blankTerm(",
            "function blankTypedTerm(",
            "function namedTerm(",
            "function ruleTerm(",
            "function ruleDelayedTerm(",
            "function matchPattern(",
            "function applyRuleTerm(",
            "function substituteTerm(",
            "function replaceAllTerm(",
            "function replaceRepeatedTerm(",
            "function unwrapTerm(",
            "function toDisplayString(",
        ] {
            assert!(
                RUNTIME.contains(needed),
                "Symbolic runtime missing `{needed}`"
            );
        }
        assert!(!RUNTIME.contains("import "));
        assert!(!RUNTIME.contains("require("));
    }

    #[test]
    fn symbolic_uses_plain_numbers_not_bigint() {
        // Deliberate divergence from the TypeScript sibling package (which
        // uses `bigint` for arbitrary precision): this backend's numeric
        // model is `number` everywhere (see `Expr::IntLit`'s emit arm), so
        // the symbolic-term port follows suit rather than introducing the
        // only `bigint` anywhere in this runtime.
        assert!(!RUNTIME.contains("BigInt"));
        assert!(!RUNTIME.contains("0n"));
    }

    #[test]
    fn symbolic_replace_repeated_loops_locally_not_recursively() {
        // SECURITY: a rule firing must loop at the SAME call frame (the
        // `while (true)` body), never recurse on the fresh replacement —
        // otherwise a caller-supplied `maxIterations` bounds only CPU time
        // in theory while still exhausting the native stack in practice
        // (the exact gap `sir-runtime-symbolic`'s own /security-review
        // found and fixed in the TypeScript sibling; this port carries the
        // fix forward rather than reintroducing the gap).
        assert!(RUNTIME.contains("while (true) {"));
        assert!(RUNTIME.contains("const MAX_TERM_DEPTH = 512;"));
    }

    #[test]
    fn symbolic_terms_render_through_format() {
        // `print`/`puts` on a Symbolic term must not fall through to the
        // useless `[object Object]` default — formatSeen dispatches to
        // `Symbolic.toDisplayString` for any plain object carrying a
        // `.kind` tag.
        assert!(RUNTIME.contains("typeof v.kind === \"string\""));
        assert!(RUNTIME.contains("Symbolic.toDisplayString(v)"));
    }

    #[test]
    fn runtime_defines_array_matrix_domain() {
        // SIR22: the emitter's ArrayLit/Range/MatMul/ElementwiseOp/
        // Transpose/IndexGet/IndexSet arms all call into `Array.*` — this
        // must stay inlined (no import/require), matching every other
        // domain in this file.
        assert!(RUNTIME.contains("const ArrayRt = (() => {"));
        assert!(RUNTIME.contains("Array: ArrayRt,"), "Array must be exported");
        for needed in [
            "function checkedShapeSize(",
            "function ndarray(",
            "function fromRows(",
            "function isScalar(",
            "function nrows(",
            "function ncols(",
            "function get(",
            "function set(",
            "function applyOp(",
            "function elementwise(",
            "function matmul(",
            "function transpose(",
            "function range(",
            "function resolvePositions(",
            "function indexGet(",
            "function indexSet(",
            "function toArrayValue(",
        ] {
            assert!(RUNTIME.contains(needed), "Array runtime missing `{needed}`");
        }
        assert!(!RUNTIME.contains("import "));
        assert!(!RUNTIME.contains("require("));
    }

    #[test]
    fn array_runtime_validates_shape_before_allocating() {
        // SECURITY: every factory that computes an output size from
        // caller-supplied numbers must validate via `checkedShapeSize`
        // *before* `new Float64Array(...)` runs, not after — an
        // unbounded or malformed shape must fail with a catchable
        // `Error`, not an uncaught `RangeError` or a stalled huge
        // allocation. Spot-check the two shapes most likely to regress:
        // an outer-product-shaped `matmul`/`indexGet` (two independently-
        // bounded dimensions whose product isn't bounded by either
        // alone) and `range`'s own element cap.
        assert!(RUNTIME.contains("const MAX_ELEMENTS = 1 << 26;"));
        assert!(RUNTIME.contains("checkedShapeSize([m, n])"));
        assert!(RUNTIME.contains("checkedShapeSize([rows.length, cols.length])"));
        assert!(RUNTIME.contains(
            "if (values.length >= MAX_ELEMENTS) {\n          throw new Error(`range: produces more than ${MAX_ELEMENTS} elements`);"
        ));
    }

    #[test]
    fn array_elementwise_coerces_bare_scalar_operands() {
        // `matlab-to-semantic-ir`'s lowerer emits a mixed number/NDArray
        // operand pair for `.* ./ .\` and for `* /` when exactly one side
        // is scalar (e.g. `A .* 2`) — the bare scalar sub-expression is
        // passed through `ElementwiseOp` unwrapped, so `elementwise` must
        // coerce a plain JS `number` into a scalar NDArray itself rather
        // than assume both operands already carry `.data`/`.shape`.
        assert!(RUNTIME.contains("a = toArrayValue(a);"));
        assert!(RUNTIME.contains("b = toArrayValue(b);"));
    }

    #[test]
    fn array_elementwise_comparisons_return_apl_style_numbers_not_booleans() {
        // Comparisons (`Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt`) must return `1`/`0`,
        // never a native `boolean` — the result has to stay a plain
        // Float64Array element like every other value here.
        assert!(RUNTIME.contains("const b2f = (cond) => (cond ? 1 : 0);"));
    }

    #[test]
    fn array_set_bounds_check_is_a_nan_safe_negated_and_not_an_or() {
        // Security-review follow-up: `set`'s bounds check must be the
        // negation of `get`'s AND-form (`!(r >= 0 && ...)`), not an
        // OR-form (`r < 0 || ...`) -- those are NOT equivalent for NaN
        // under IEEE-754 (every relational comparison with NaN is
        // false), so an OR-form would silently skip the throw and let
        // `a.data[c * nrows(a) + NaN] = value` silently drop the write.
        // `set` is not reachable with an unvalidated NaN through any
        // current codegen path (every caller resolves positions through
        // `assertValidPosition` first), but it is part of this module's
        // exported public surface, so it must stay NaN-safe on its own.
        assert!(RUNTIME.contains("if (!(r >= 0 && c >= 0 && r < nrows(a) && c < ncols(a))) {"));
    }

    // ── SIR22 addendum: APL primitive operators ────────────────────────

    #[test]
    fn runtime_defines_array_addendum_functions() {
        // `apl-to-semantic-ir` emits `Reduce`/`Scan`/`OuterProduct`/`Shape`/
        // `Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`; the
        // emitter's arms for all nine call into these, plus `display` (the
        // APL auto-print formatter `formatSeen` dispatches to).
        for needed in [
            "function reduce(",
            "function scan(",
            "function outer(",
            "function shape(",
            "function reshape(",
            "function indexGenerator(",
            "function indexOf(",
            "function ravel(",
            "function catenate(",
            "function flattenRowMajor(",
            "function fmtNum(",
            "function display(",
        ] {
            assert!(RUNTIME.contains(needed), "Array runtime missing `{needed}`");
        }
        assert!(RUNTIME.contains("reduce, scan, outer, shape, reshape, indexGenerator, indexOf, ravel,"));
    }

    #[test]
    fn array_addendum_reuses_the_one_bounded_allocation_cap() {
        // SECURITY: `⍳`'s length, dyadic `⍴`'s target element count,
        // `⍳`(index-of)'s O(len*len) product, and `,`(catenate)'s combined
        // length are all runtime-computed from potentially attacker-
        // influenced program values -- every one of them must route
        // through `checkedShapeSize` (this file's ONE existing
        // `MAX_ELEMENTS`-capped guard), not a freshly-invented cap value
        // (`apl_runtime::builtins::MAX_ARRAY_LENGTH` is a *different*,
        // smaller Rust-side constant that this port deliberately does not
        // reintroduce -- see the addendum's own module doc comment).
        assert!(RUNTIME.contains("const n = checkedShapeSize([x]);"));
        assert!(RUNTIME.contains("const total = checkedShapeSize(dims);"));
        assert!(RUNTIME.contains("checkedShapeSize([a.data.length, b.data.length]);"));
        assert!(RUNTIME.contains("checkedShapeSize([a.data.length + b.data.length]);"));
        // (A JS-side doc comment nearby mentions the Rust constant's NAME in
        // prose, explaining why it is deliberately NOT reintroduced here —
        // so this asserts there is no `const MAX_ARRAY_LENGTH` DECLARATION,
        // not that the identifier never appears as text anywhere at all.)
        assert!(!RUNTIME.contains("const MAX_ARRAY_LENGTH"));
    }

    #[test]
    fn reduce_on_an_empty_vector_is_a_clean_error_not_a_guessed_identity() {
        // `reduce` has no built-in identity for an arbitrary op (unlike
        // `sum`, which hardcodes 0) -- an empty axis must throw, not
        // silently return e.g. 0.
        assert!(RUNTIME.contains(
            "throw new Error(\"reduce: cannot fold an empty vector (no identity element for an arbitrary op)\");"
        ));
    }

    #[test]
    fn index_generator_is_one_based_unlike_the_rest_of_this_domain() {
        // `⍳n` is `[1, 2, ..., n]` -- 1-based, unlike `indexGet`/`indexSet`
        // elsewhere in this same Array namespace, which are 0-based. This
        // is a real APL-surface-syntax fact (see `semantic-ir`'s
        // `Expr::IndexGenerator` doc comment), not an inconsistency.
        assert!(RUNTIME.contains("out[i] = i + 1;"));
    }

    #[test]
    fn reshape_transposes_row_major_fill_into_column_major_storage() {
        // The single easiest place to introduce a silent wrong-answer bug
        // in this whole port: reshape's cyclic fill is computed in
        // ROW-major order (APL convention) but must be written back into
        // COLUMN-major storage (this domain's convention) for a rank-2
        // target -- `filled[row * c + col]` read, `data[col * r + row]`
        // written, never the other way around.
        assert!(RUNTIME.contains("data[col * r + row] = filled[row * c + col];"));
    }

    #[test]
    fn array_display_uses_apl_high_minus_and_no_trailing_dot_zero() {
        // `apl-to-semantic-ir` auto-prints a bare top-level expression
        // through this backend's shared `print` builtin, and APL has no
        // bracket-indexing syntax to read a scalar back with (unlike
        // MATLAB) -- so `formatSeen` must render a raw NDArray using APL's
        // OWN console convention (high-minus `¯`, matching
        // `apl_runtime::value::fmt_num` 1:1), not `[object Object]`.
        assert!(RUNTIME.contains("return x < 0 ? \"¯\" + body : body;"));
        assert!(RUNTIME.contains("ArrayRt.display(v)"));
    }

    #[test]
    fn neg_negates_a_rank1_plus_ndarray_elementwise_instead_of_relying_on_numof() {
        // Regression guard for bug #2 (`apl-to-semantic-ir/tests/oracle.rs`):
        // a genuine rank >= 1 NDArray operand must be mapped elementwise into
        // a NEW NDArray (never coerced to `NaN` via native unary minus on a
        // plain object).
        assert!(RUNTIME.contains("function mapNDArrayRank1Plus(x, f) {"));
        assert!(RUNTIME.contains("x.shape.length >= 1"));
        assert!(RUNTIME.contains("ArrayRt.ndarray(x.shape, Float64Array.from(x.data, f));"));
    }

    #[test]
    fn runtime_registers_apl_monadic_scalar_atom_builtins() {
        // Regression guard for bug #3: `sign`/`recip`/`ceil`/`floor` were
        // documented but never registered in the `builtins` dispatch table,
        // so `__Sir.callBuiltin` crashed with `TypeError: unknown builtin:
        // <name>` for every one of them. `monadicScalarAtom` is the shared
        // scalar/array dispatch all four route through.
        assert!(RUNTIME.contains("function aplSign(v) {"));
        assert!(RUNTIME.contains("function aplRecip(v) { return 1 / v; }"));
        assert!(RUNTIME.contains("function monadicScalarAtom(x, f) {"));
        for (name, helper) in [
            ("\"sign\"", "aplSign"),
            ("\"recip\"", "aplRecip"),
            ("\"ceil\"", "Math.ceil"),
            ("\"floor\"", "Math.floor"),
        ] {
            assert!(
                RUNTIME.contains(&format!("{name}: (x) => monadicScalarAtom(x, {helper}),")),
                "builtins table missing {name} -> monadicScalarAtom(x, {helper})"
            );
        }
        // `aplSign` is explicit if/else branching (matching `apl_runtime::
        // eval::apl_sign` 1:1) -- confirmed by the `function aplSign(v) {`
        // assertion above; see that function's own doc comment for why a
        // bare `Math.sign()` call is deliberately not used instead.
    }

    #[test]
    fn formatseen_gates_bare_number_and_boxed_float_glyph_on_apl_high_minus_flag() {
        // Regression guard for bug #1: a rank-0 SIR22 NDArray is not unique
        // to APL (MATLAB's `2 ^ 2` reaches the same shape), so the glyph
        // decision for a BARE/boxed scalar has to live in `formatSeen`,
        // gated by a per-module flag -- never inferred from the value's own
        // shape the way `neg`'s array branch is.
        assert!(RUNTIME.contains("const SIR_DISPLAY_APL_HIGH_MINUS = __SIR_DISPLAY_APL_HIGH_MINUS__;"));
        assert!(RUNTIME.contains(
            "return SIR_DISPLAY_APL_HIGH_MINUS ? ArrayRt.fmtNum(v.f) : floatToRubyString(v.f);"
        ));
        assert!(RUNTIME.contains(
            "return SIR_DISPLAY_APL_HIGH_MINUS ? ArrayRt.fmtNum(v) : String(v);"
        ));
        // `fmtNum` must be reachable from OUTSIDE the `ArrayRt` IIFE (where
        // `formatSeen` lives) for the branches above to compile at all.
        assert!(RUNTIME.contains("fmtNum,"));
    }
}
