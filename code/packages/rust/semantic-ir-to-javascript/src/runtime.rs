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

/// The full inlined runtime.  Always emitted verbatim, exactly once,
/// near the top of every artifact (after the banner, before the user's
/// function declarations).
///
/// Indentation is 2 spaces, every statement is terminated with a
/// semicolon, and the blob ends with a newline so the following
/// declarations start on their own line.
pub const RUNTIME: &str = r##"const __Sir = (() => {
  "use strict";
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

  // ── truthiness ─────────────────────────────────────────────────
  // SIR truthiness, NOT JavaScript's: only `false` and `nil` (null)
  // are falsy.  `0`, `""`, and `NaN` are all truthy — matching Lisp /
  // Ruby semantics rather than JS's surprising coercions.
  function truthy(v) {
    return v !== false && v !== null && v !== undefined;
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
    if (v === true) { return "#t"; }
    if (v === false) { return "#f"; }
    if (typeof v === "string") { return v; }
    if (typeof v === "number") { return String(v); }
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
    return String(v);
  }

  // ── SIR value equality (structural, for `uniq`) ────────────────
  // Ruby's `Array#uniq` dedups by `eql?`/`hash`, i.e. VALUE equality: two
  // equal-but-distinct arrays (`[1,2]` and a separate `[1,2]`) count as one.
  // Primitives compare with `===` — the same op the `"="`/`case_eq` builtins
  // use, so `uniq` agrees with `==` everywhere else.  Arrays and Maps compare
  // STRUCTURALLY (element-/entry-wise), matching Ruby, since JS `===` on two
  // distinct-but-equal arrays is `false`.  Cycle safety: a program can build a
  // self-referential array (`a = []; a << a`), so the recursive array walk is
  // guarded by a `seen` set of the (a, b) pairs already on the compare path —
  // a re-encountered pair is treated as equal (the cycle has matched so far),
  // which terminates instead of recursing forever (CWE-674).  `Sym`/`Pair`/
  // `Closure` fall back to reference identity (`===`), matching how the rest
  // of the runtime treats them as opaque.
  function sirEqual(a, b) { return sirEqualSeen(a, b, new Set()); }
  function sirEqualSeen(a, b, seen) {
    if (a === b) { return true; }
    if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) { return false; }
      // Cycle guard: record the LEFT array reference on the active compare
      // path.  Re-encountering it means we are inside a self-referential
      // structure whose elements have matched so far — treat as equal and
      // stop recursing, so a cyclic array terminates instead of looping.
      if (seen.has(a)) { return true; }
      seen.add(a);
      let ok = true;
      for (let i = 0; i < a.length; i++) {
        if (!sirEqualSeen(a[i], b[i], seen)) { ok = false; break; }
      }
      seen.delete(a);
      return ok;
    }
    if (a instanceof Map && b instanceof Map) {
      if (a.size !== b.size) { return false; }
      // Same cycle guard as the array branch: two DISTINCT but structurally
      // equal self-referential hashes (`a={}; a[:k]=a; b={}; b[:k]=b`) would
      // otherwise recurse forever through their values (the `===` fast-path
      // only covers the reference-identical case).  Record the LEFT map on
      // the active compare path and short-circuit on re-encounter.
      if (seen.has(a)) { return true; }
      seen.add(a);
      let ok = true;
      for (const [k, v] of a) {
        if (!b.has(k) || !sirEqualSeen(v, b.get(k), seen)) { ok = false; break; }
      }
      seen.delete(a);
      return ok;
    }
    return false;
  }

  // ── deep flatten (Ruby `Array#flatten`) ────────────────────────
  // Ruby's `flatten` recursively splices EVERY nested array into a single
  // flat array, into a FRESH result (no aliasing / mutation of any input).
  // Cycle safety mirrors `putsOne`/`format`: a self-referential array must
  // not infinite-loop (CWE-674).  `seen` holds the array references on the
  // active flatten path; an array already on the path is a cycle — real Ruby
  // raises `ArgumentError: tried to flatten recursive array`, so we do too
  // (a typed, rescuable error), rather than hanging or emitting a placeholder.
  function flattenDeep(arr, out, seen) {
    if (seen.has(arr)) {
      raiseError("ArgumentError", "tried to flatten recursive array");
    }
    seen.add(arr);
    for (const el of arr) {
      if (Array.isArray(el)) { flattenDeep(el, out, seen); }
      else { out.push(el); }
    }
    seen.delete(arr);
    return out;
  }

  // ── builtins dispatch table ────────────────────────────────────
  // Reached only for builtins the emitter did not specialise inline
  // (e.g. a variadic `+`, or a builtin referenced as a value via
  // `__Sir.builtins["name"]`).  Each entry is an ordinary JS function.
  function numFold(args, init, step) {
    let acc = init;
    for (const a of args) { acc = step(acc, a); }
    return acc;
  }
  const builtins = {
    // `+`/`*` route through the polymorphic helpers (hoisted function
    // declarations below) so a builtin referenced as a VALUE, or a
    // variadic `(+ 1 2 3)`, gets the same string/array/numeric dispatch
    // as the inlined 2-arg form.
    "+": (...a) => plus(...a),
    "-": (...a) => a.length === 1 ? -a[0] : numFold(a.slice(1), a[0], (x, y) => x - y),
    "*": (...a) => times(...a),
    "/": (...a) => a.length === 1 ? 1 / a[0] : numFold(a.slice(1), a[0], (x, y) => x / y),
    "=": (x, y) => x === y,
    // Ruby case-equality (`pattern === value`) — the test a `when`/`in` arm
    // runs.  Ruby keys `===` to the pattern's type (Range → membership, Regexp
    // → match); this backend has no Range/Regexp value, so the only patterns
    // that reach here are plain values and the op is ordinary equality (the
    // same `===` the `=` builtin uses).  `when SomeClass` is lowered to
    // `.is_a?` at the frontend and never becomes a case_eq call.
    "case_eq": (pattern, value) => pattern === value,
    "<": (x, y) => x < y,
    ">": (x, y) => x > y,
    "<=": (x, y) => x <= y,
    ">=": (x, y) => x >= y,
    "not": (x) => !truthy(x),
    "neg": (x) => -x,
    "cons": (x, y) => new Pair(x, y),
    "car": (p) => p.car,
    "cdr": (p) => p.cdr,
    "pair?": (p) => p instanceof Pair,
    "null?": (x) => x === null || x === undefined,
    "number?": (x) => typeof x === "number",
    "symbol?": (x) => x instanceof Sym,
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
  };
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
      if (typeof first === "string" && typeof rhs === "number") {
        // String repeat: `"ab" * 3` → "ababab".  Empty receiver short-
        // circuits so a huge count does no work; the guard rejects an
        // oversized product before `repeat` allocates.
        if (first.length === 0) { return ""; }
        const n = repeatCount(first.length, rhs);
        return n === 0 ? "" : first.repeat(n);
      }
      if (Array.isArray(first) && typeof rhs === "number") {
        // Array repeat: `[0] * 3` → [0, 0, 0], a NEW array.  Empty
        // receiver short-circuits; the guard bounds total elements.
        if (first.length === 0) { return []; }
        const n = repeatCount(first.length, rhs);
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
    if (b === 0) {
      raiseError("ZeroDivisionError", "divided by 0");
    }
    return a / b;
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
    if (typeof recv === "boolean" && BOOL_METHODS.has(name)) { return true; }
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
    // ── Ruby Array collection methods NOT native to JS arrays ─────
    // `sum`/`min`/`max`/`uniq`/`flatten`/`compact`/`each_with_index`/`to_a`
    // are everyday Ruby `Array` methods with NO 1:1 native JS equivalent (JS
    // has `Math.min`/`.flat`/… but their semantics diverge — `flat` is
    // shallow-by-default, `Math.min([])` is `Infinity` not `nil`, there is no
    // native `uniq`/`compact`/`sum`/`each_with_index`).  So — exactly like
    // `.fetch` above — they are handled HERE, ahead of the native-method
    // allowlist, as EXPLICIT Ruby-semantic special cases.  Dispatch is a
    // fixed `name ===` test on an `Array.isArray(recv)` receiver: never
    // `recv[name]`, `eval`, or reflection on the source-derived name, so the
    // RCE-hardened allowlist gate below is completely untouched (a name like
    // `constructor` still falls through to it and is rejected).
    if (Array.isArray(recv)) {
      // `sum` — numeric sum; empty → 0 (or the seed).  A seed arg (`sum(s)`)
      // is the starting accumulator, matching Ruby.  We fold through the
      // runtime's polymorphic `plus`, so int/float promotion (and, per Ruby,
      // string/array concat when a matching seed is given) is consistent with
      // `+` everywhere else.
      if (name === "sum") {
        let acc = args.length >= 1 ? args[0] : 0;
        for (const el of recv) { acc = plus(acc, el); }
        return acc;
      }
      // `min` / `max` — element-wise extreme by the SIR `<` order (the same
      // comparison the `"<"` builtin uses: native `<` on numbers/strings).
      // An EMPTY array has no extreme → `nil` (null), matching Ruby.
      if (name === "min" || name === "max") {
        if (recv.length === 0) { return null; }
        let best = recv[0];
        for (let i = 1; i < recv.length; i++) {
          const el = recv[i];
          // `max` keeps the larger, `min` the smaller.  Strict `<` so an
          // equal element does not displace the earlier one (stable).
          if (name === "max" ? best < el : el < best) { best = el; }
        }
        return best;
      }
      // `uniq` — first-occurrence dedup by SIR VALUE equality (`sirEqual`),
      // into a FRESH array (no mutation / aliasing of the receiver).  A later
      // element equal to one already kept is dropped.
      if (name === "uniq") {
        const out = [];
        for (const el of recv) {
          if (!out.some((k) => sirEqual(k, el))) { out.push(el); }
        }
        return out;
      }
      // `flatten` — DEEP recursive flatten into a FRESH array, cycle-guarded
      // (a self-referential array raises ArgumentError rather than looping).
      if (name === "flatten") {
        return flattenDeep(recv, [], new Set());
      }
      // `compact` — a NEW array with the nils (null / undefined) removed.
      if (name === "compact") {
        return recv.filter((el) => el !== null && el !== undefined);
      }
      // `each_with_index` — apply the trailing block Closure with
      // `(element, index)` for each element, in order, and return the
      // RECEIVER (Ruby returns self).  A block-less call returns the receiver
      // too (Ruby returns an Enumerator; v0 floor).  The block arrives as the
      // last RAW arg (an un-unwrapped `Closure`), invoked via `applyClosure`.
      if (name === "each_with_index") {
        const block = rawArgs[rawArgs.length - 1];
        if (block instanceof Closure) {
          for (let i = 0; i < recv.length; i++) {
            applyClosure(block, [recv[i], i]);
          }
        }
        return recv;
      }
      // `to_a` on an array is the identity (returns self), matching Ruby.
      if (name === "to_a") { return recv; }
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
    if (typeof recv === "number") { return "an instance of Numeric"; }
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

  return {
    Sym, Pair, Closure,
    intern, applyClosure, truthy, format, print, puts,
    plus, times, divide,
    builtins, builtinClosure, callBuiltin, callMethod,
    SirError, raiseError, rescueMatches, registerAncestry,
    // OOP (O3): instantiation, method definition + dispatch, super,
    // the self stack, and instance/class-variable access.
    SirInstance, newInstance, callNew, callSuper,
    defMethod, defClassMethod, currentSelf,
    ivarGet, ivarSet, cvarGet, cvarSet,
    // Mixins (MX4): include/extend registration + class-method dispatch.
    includeModule, extendModule, callClassMethod,
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
            "intern", "applyClosure", "truthy", "format",
            "builtins", "builtinClosure", "callBuiltin", "callMethod",
            "class Sym", "class Pair", "class Closure",
            // Exception runtime (SIR17): the four helpers the emitter
            // references from its TryCatch / raise / ClassDef arms.
            "class SirError", "raiseError", "rescueMatches", "registerAncestry",
            // OOP runtime (O3): the helpers the emitter references from
            // its __new__ / __super__ / __def_method__ / @ivar arms.
            "class SirInstance", "callNew", "callSuper",
            "defMethod", "defClassMethod", "currentSelf",
            "ivarGet", "ivarSet", "cvarGet", "cvarSet",
            // Mixins (MX4): include/extend + class-method dispatch.
            "includeModule", "extendModule", "callClassMethod",
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
        assert!(RUNTIME.contains("plus, times, divide,"), "divide must be exported");

        // `.fetch` raises typed errors: IndexError for a sequence OOB,
        // KeyError for a missing hash key (no default).
        assert!(RUNTIME.contains(r#"if (name === "fetch")"#));
        assert!(RUNTIME.contains(r#"raiseError("IndexError","#));
        assert!(RUNTIME.contains(r#"raiseError("KeyError", "key not found: ""#));

        // An unknown method raises NoMethodError (not a JS-native TypeError).
        assert!(RUNTIME.contains(r#"raiseError("NoMethodError","#));
        assert!(RUNTIME.contains(r#""undefined method `" + name + "` for " + classDescription(recv)"#));
        assert!(RUNTIME.contains("function classDescription(recv)"));
        // The old JS-native TypeError floor for the allowlist miss is gone.
        assert!(!RUNTIME.contains("is not an allowed collection method"));
    }

    #[test]
    fn runtime_defines_m6_universal_metaprogramming_surface() {
        // M6: send/tap/then/yield_self/respond_to? + boolean &/|/^ are mixed
        // into EVERY receiver, ported to match the Python/TS references.
        assert!(RUNTIME.contains(r#"const SEND_METHODS = new Set(["send", "__send__", "public_send"]);"#));
        assert!(RUNTIME.contains(r#"const OBJECT_BLOCK_METHODS = new Set(["tap", "then", "yield_self"]);"#));
        assert!(RUNTIME.contains(r#"const BOOL_METHODS = new Set(["&", "|", "^"]);"#));
        assert!(RUNTIME.contains("function objectMetaMethod("));
        assert!(RUNTIME.contains("function respondsTo("));
        assert!(RUNTIME.contains("function boolMethod("));
        // `tap` returns the receiver; `then`/`yield_self` return the block result.
        assert!(RUNTIME.contains(r#"if (name === "tap") { applyClosure(last, [recv]); return recv; }"#));
        assert!(RUNTIME.contains("return applyClosure(last, [recv]); // then / yield_self"));

        // SECURITY (the C3 RCE lesson): `send` routes the DYNAMIC name back
        // through `callMethod` — the SAME allowlist / method-table gate a direct
        // call uses — NEVER `recv[name]` / `eval` / `new Function` on the name.
        assert!(RUNTIME.contains("return callMethod(recv, methodNameArg(rawArgs[0]), ...rawArgs.slice(1));"));
        assert!(!RUNTIME.contains("new Function("));
        assert!(!RUNTIME.contains("eval("));
        // `respond_to?` checks the same tables dispatch uses (method table for a
        // SirInstance, the allowlist for a primitive) — not a probe of recv[name].
        assert!(RUNTIME.contains("resolveMethod(methodTable, recv.sirClass, name, includedModules) !== undefined"));
        assert!(RUNTIME.contains("METHOD_ALLOWLIST.has(native)"));
    }

    #[test]
    fn runtime_defines_ruby_array_collection_methods() {
        // Parity fill: the Ruby Array methods with NO 1:1 native JS equivalent
        // (`sum`/`min`/`max`/`uniq`/`flatten`/`compact`/`each_with_index`/`to_a`)
        // are special-cased in `callMethod`, ahead of the native allowlist, on
        // an `Array.isArray(recv)` receiver.  Each name must be recognised.
        assert!(RUNTIME.contains(r#"if (name === "sum")"#));
        assert!(RUNTIME.contains(r#"if (name === "min" || name === "max")"#));
        assert!(RUNTIME.contains(r#"if (name === "uniq")"#));
        assert!(RUNTIME.contains(r#"if (name === "flatten")"#));
        assert!(RUNTIME.contains(r#"if (name === "compact")"#));
        assert!(RUNTIME.contains(r#"if (name === "each_with_index")"#));
        assert!(RUNTIME.contains(r#"if (name === "to_a")"#));

        // The helpers the special-cases reuse.
        assert!(RUNTIME.contains("function sirEqual("));
        assert!(RUNTIME.contains("function flattenDeep("));

        // SECURITY: dispatch is a fixed `name ===` test on `Array.isArray`,
        // NEVER a reflective `recv[name]` / eval on the source-derived name —
        // the RCE-hardened allowlist gate is untouched.
        assert!(!RUNTIME.contains("new Function("));
        assert!(!RUNTIME.contains("eval("));

        // `sum` folds through the polymorphic `plus` (int/float promotion).
        assert!(RUNTIME.contains("acc = plus(acc, el);"));
        // `min`/`max` floor an empty array to `nil` (null), matching Ruby.
        assert!(RUNTIME.contains("if (recv.length === 0) { return null; }"));
        // `uniq` dedups by SIR VALUE equality, not JS `===` on references.
        assert!(RUNTIME.contains("!out.some((k) => sirEqual(k, el))"));
        // `flatten` is cycle-guarded: a self-referential array raises a typed,
        // rescuable ArgumentError rather than looping forever (CWE-674).
        assert!(RUNTIME.contains(r#"raiseError("ArgumentError", "tried to flatten recursive array")"#));
        // `each_with_index` invokes the trailing Closure block with (el, i) and
        // returns the receiver.
        assert!(RUNTIME.contains("applyClosure(block, [recv[i], i]);"));
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
}
