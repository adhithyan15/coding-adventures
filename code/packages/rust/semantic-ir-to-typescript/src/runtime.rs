//! Inlined TypeScript runtime helpers.
//!
//! The TypeScript backend produces **self-contained** output: every
//! generated `.ts` file embeds the runtime helpers it needs.  This
//! module supplies the runtime as a single string constant that is
//! pasted into every artifact, plus a banner comment.
//!
//! Per SIR12, the runtime is byte-identical across modules (a future
//! optimization could elide unused parts; v0 favours simplicity).
//!
//! Style notes:
//!
//! - All helpers live inside a `__Sir` TypeScript namespace so they
//!   never collide with user names.
//! - The value model is a discriminated union (`__Sir.Val`).
//! - Cons cells / closures / symbols are tiny classes — no
//!   inheritance, no decorators, easy for `tsc` to type-check.

/// The full runtime block.  Always emitted verbatim.
pub const RUNTIME: &str = r##"namespace __Sir {
  // ── value model ───────────────────────────────────────────────
  export type Val =
    | number
    | boolean
    | null
    | string
    | Sym
    | Pair
    | Closure;

  export const NIL: null = null;

  // ── symbols (interned) ────────────────────────────────────────
  export class Sym {
    readonly name: string;
    constructor(name: string) { this.name = name; }
  }
  const SYMBOL_TABLE = new Map<string, Sym>();
  export function intern(name: string): Sym {
    let s = SYMBOL_TABLE.get(name);
    if (s === undefined) { s = new Sym(name); SYMBOL_TABLE.set(name, s); }
    return s;
  }

  // ── cons cells ────────────────────────────────────────────────
  export class Pair {
    car: Val;
    cdr: Val;
    constructor(car: Val, cdr: Val) { this.car = car; this.cdr = cdr; }
  }

  // ── closures ──────────────────────────────────────────────────
  export class Closure {
    readonly fn: (...args: Val[]) => Val;
    constructor(fn: (...args: Val[]) => Val) { this.fn = fn; }
  }
  export function applyClosure(c: Val, args: Val[]): Val {
    if (!(c instanceof Closure)) {
      throw new Error("apply on non-closure value");
    }
    return c.fn(...args);
  }

  // ── module globals ────────────────────────────────────────────
  // `_init` populates this; user code reads/writes via builtins.
  const GLOBALS = new Map<string, Val>();
  export function globalSet(name: Val, value: Val): Val {
    const key = (name instanceof Sym) ? name.name : String(name);
    GLOBALS.set(key, value);
    return value;
  }
  export function globalGet(name: Val): Val {
    const key = (name instanceof Sym) ? name.name : String(name);
    const v = GLOBALS.get(key);
    if (v === undefined) {
      throw new Error("undefined global: " + key);
    }
    return v;
  }

  // ── builtins ──────────────────────────────────────────────────
  export function plus(...args: Val[]): Val {
    let total = 0;
    for (const a of args) total += a as number;
    return total;
  }
  export function minus(...args: Val[]): Val {
    if (args.length === 0) return 0;
    if (args.length === 1) return -(args[0] as number);
    let acc = args[0] as number;
    for (let i = 1; i < args.length; i++) acc -= args[i] as number;
    return acc;
  }
  export function times(...args: Val[]): Val {
    let total = 1;
    for (const a of args) total *= a as number;
    return total;
  }
  export function divide(...args: Val[]): Val {
    if (args.length === 0) return 0;
    let acc = args[0] as number;
    for (let i = 1; i < args.length; i++) acc = Math.trunc(acc / (args[i] as number));
    return acc;
  }
  export function eq(a: Val, b: Val): Val {
    if (a instanceof Sym && b instanceof Sym) return a.name === b.name;
    return a === b;
  }
  export function lt(a: Val, b: Val): Val { return (a as number) < (b as number); }
  export function gt(a: Val, b: Val): Val { return (a as number) > (b as number); }
  export function cons(a: Val, b: Val): Val { return new Pair(a, b); }
  export function car(a: Val): Val {
    if (!(a instanceof Pair)) throw new Error("car on non-pair");
    return a.car;
  }
  export function cdr(a: Val): Val {
    if (!(a instanceof Pair)) throw new Error("cdr on non-pair");
    return a.cdr;
  }
  export function isNull(a: Val): Val { return a === null; }
  export function isPair(a: Val): Val { return a instanceof Pair; }
  export function isNumber(a: Val): Val { return typeof a === "number"; }
  export function isSymbol(a: Val): Val { return a instanceof Sym; }
  export function print(a: Val): Val {
    console.log(format(a));
    return null;
  }

  // ── formatting ────────────────────────────────────────────────
  export function format(v: Val): string {
    if (v === null) return "nil";
    if (typeof v === "boolean") return v ? "#t" : "#f";
    if (typeof v === "number") return String(v);
    if (typeof v === "string") return v;
    if (v instanceof Sym) return v.name;
    if (v instanceof Pair) return formatPair(v);
    if (v instanceof Closure) return "<closure>";
    return String(v);
  }
  function formatPair(p: Pair): string {
    let out = "(" + format(p.car);
    let rest: Val = p.cdr;
    while (rest instanceof Pair) {
      out += " " + format(rest.car);
      rest = rest.cdr;
    }
    if (rest !== null) {
      out += " . " + format(rest);
    }
    return out + ")";
  }

  // ── truthiness — only #f and nil are false ────────────────────
  export function truthy(v: Val): boolean { return v !== false && v !== null; }

  // ── builtin dispatch by name (for var-ref scope=Builtin) ──────
  export const builtins: Record<string, (...args: Val[]) => Val> = {
    "+": plus, "-": minus, "*": times, "/": divide,
    "=": eq, "<": lt, ">": gt,
    "cons": cons, "car": car, "cdr": cdr,
    "null?": isNull, "pair?": isPair, "number?": isNumber, "symbol?": isSymbol,
    "print": print,
    "global_set": (a, b) => globalSet(a, b),
    "global_get": (a) => globalGet(a),
  };
}
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
    fn runtime_declares_namespace_and_value_model() {
        assert!(RUNTIME.contains("namespace __Sir"));
        assert!(RUNTIME.contains("export type Val"));
    }

    #[test]
    fn runtime_includes_all_builtins() {
        for op in &[
            "plus", "minus", "times", "divide", "eq", "lt", "gt",
            "cons", "car", "cdr", "isNull", "isPair", "isNumber", "isSymbol",
            "print", "globalSet", "globalGet", "intern", "applyClosure",
        ] {
            assert!(
                RUNTIME.contains(op),
                "runtime missing helper `{}`",
                op
            );
        }
    }

    #[test]
    fn runtime_exposes_truthy() {
        assert!(RUNTIME.contains("export function truthy"));
    }
}
