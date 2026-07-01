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
  function format(v) {
    if (v === null || v === undefined) { return "nil"; }
    if (v === true) { return "#t"; }
    if (v === false) { return "#f"; }
    if (typeof v === "string") { return v; }
    if (typeof v === "number") { return String(v); }
    if (v instanceof Sym) { return v.name; }
    if (v instanceof Pair) {
      return "(" + format(v.car) + " . " + format(v.cdr) + ")";
    }
    if (v instanceof Closure) { return "#<closure>"; }
    if (Array.isArray(v)) { return "[" + v.map(format).join(", ") + "]"; }
    return String(v);
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
    "+": (...a) => a.length === 0 ? 0 : numFold(a.slice(1), a[0], (x, y) => x + y),
    "-": (...a) => a.length === 1 ? -a[0] : numFold(a.slice(1), a[0], (x, y) => x - y),
    "*": (...a) => numFold(a, 1, (x, y) => x * y),
    "/": (...a) => a.length === 1 ? 1 / a[0] : numFold(a.slice(1), a[0], (x, y) => x / y),
    "=": (x, y) => x === y,
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
  function callMethod(recv, name, ...rawArgs) {
    const args = rawArgs.map(unwrapArg);
    // `length` as a nullary method mirrors the property.  Kept special-cased
    // ahead of the allowlist: it is a property read, not a method call.
    if (name === "length" && args.length === 0) { return recv.length; }
    // SECURITY gate: refuse any name outside the allowlist so reflective
    // gadgets (`constructor`, `__proto__`, `apply`, …) can never be reached.
    if (!METHOD_ALLOWLIST.has(name)) {
      throw new TypeError(
        "method `" + name + "` is not an allowed collection method");
    }
    const m = recv == null ? undefined : recv[name];
    if (typeof m !== "function") {
      throw new TypeError(
        "method `" + name + "` is not defined on " + format(recv));
    }
    return m.apply(recv, args);
  }

  return {
    Sym, Pair, Closure,
    intern, applyClosure, truthy, format, print,
    builtins, builtinClosure, callBuiltin, callMethod,
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
        ] {
            assert!(RUNTIME.contains(needed), "runtime missing `{needed}`");
        }
    }
}
