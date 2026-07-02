/** Tests for @coding-adventures/sir-runtime-core. */

import { describe, it, expect, vi } from "vitest";
import * as sir from "../src/index.js";
import { SirError } from "@coding-adventures/sir-runtime-exceptions";

describe("truthiness (false/nil-only)", () => {
  it("only false and nil are falsy", () => {
    expect(sir.truthy(false)).toBe(false);
    expect(sir.truthy(null)).toBe(false);
  });

  it("everything else is truthy", () => {
    for (const v of [0, "", 1, "x", sir.intern("s")] as sir.Val[]) {
      expect(sir.truthy(v)).toBe(true);
    }
  });
});

describe("symbols", () => {
  it("interned symbols share identity", () => {
    expect(sir.intern("a")).toBe(sir.intern("a"));
    expect(sir.intern("a")).not.toBe(sir.intern("b"));
  });

  it("eq is symbol-aware", () => {
    expect(sir.eq(sir.intern("a"), sir.intern("a"))).toBe(true);
    expect(sir.eq(sir.intern("a"), sir.intern("b"))).toBe(false);
    expect(sir.eq(1, 1)).toBe(true);
  });
});

describe("pairs", () => {
  it("cons/car/cdr", () => {
    const p = sir.cons(1, 2);
    expect(sir.car(p)).toBe(1);
    expect(sir.cdr(p)).toBe(2);
    expect(sir.isPair(p)).toBe(true);
    expect(sir.isPair(1)).toBe(false);
  });

  it("car/cdr reject non-pairs", () => {
    expect(() => sir.car(1)).toThrow(TypeError);
    expect(() => sir.cdr(1)).toThrow(TypeError);
  });

  it("list display", () => {
    const proper = sir.cons(1, sir.cons(2, sir.cons(3, null)));
    expect(sir.toDisplay(proper)).toBe("(1 2 3)");
    expect(sir.toDisplay(sir.cons(1, 2))).toBe("(1 . 2)");
  });
});

describe("predicates and display", () => {
  it("predicates", () => {
    expect(sir.isNull(null)).toBe(true);
    expect(sir.isNull(0)).toBe(false);
    expect(sir.isNumber(3)).toBe(true);
    expect(sir.isSymbol(sir.intern("x"))).toBe(true);
    expect(sir.isSymbol("x")).toBe(false);
  });

  it("display forms", () => {
    expect(sir.toDisplay(null)).toBe("nil");
    expect(sir.toDisplay(true)).toBe("#t");
    expect(sir.toDisplay(false)).toBe("#f");
    expect(sir.toDisplay(sir.intern("sym"))).toBe("sym");
    expect(sir.toDisplay(42)).toBe("42");
    expect(sir.toDisplay("hi")).toBe("hi");
  });

  it("print returns null and writes display form", () => {
    const lines: string[] = [];
    const spy = vi.spyOn(console, "log").mockImplementation((s: string) => {
      lines.push(s);
    });
    expect(sir.print(null)).toBe(null);
    spy.mockRestore();
    expect(lines).toEqual(["nil"]);
  });
});

describe("puts (Ruby semantics)", () => {
  // `puts` writes via process.stdout.write (not console.log) so the
  // trailing-newline-suppression rule can be honoured; capture that stream.
  function capture(fn: () => void): string {
    let out = "";
    const spy = vi
      .spyOn(process.stdout, "write")
      .mockImplementation((chunk: string | Uint8Array): boolean => {
        out += String(chunk);
        return true;
      });
    try {
      fn();
    } finally {
      spy.mockRestore();
    }
    return out;
  }

  it("no args prints a single newline", () => {
    expect(capture(() => expect(sir.puts()).toBe(null))).toBe("\n");
  });

  it("a string is followed by a newline", () => {
    expect(capture(() => sir.puts("hello"))).toBe("hello\n");
  });

  it("does not double a trailing newline", () => {
    expect(capture(() => sir.puts("x\n"))).toBe("x\n");
  });

  it("multiple args go one per line", () => {
    expect(capture(() => sir.puts("a", "b"))).toBe("a\nb\n");
  });

  it("an array is flattened, one element per line", () => {
    expect(capture(() => sir.puts([1, 2, 3]))).toBe("1\n2\n3\n");
    expect(capture(() => sir.puts([1, [2, 3]]))).toBe("1\n2\n3\n");
  });

  it("an empty array prints a single newline", () => {
    expect(capture(() => sir.puts([]))).toBe("\n");
  });

  it("nil prints a blank line (not the display form)", () => {
    expect(capture(() => sir.puts(null))).toBe("\n");
  });

  it("matches the reference program output", () => {
    // `puts "hello"; puts; puts [1,2,3]`
    expect(
      capture(() => {
        sir.puts("hello");
        sir.puts();
        sir.puts([1, 2, 3]);
      }),
    ).toBe("hello\n\n1\n2\n3\n");
  });

  it("routes through callBuiltin by name", () => {
    expect(capture(() => expect(sir.callBuiltin("puts", ["hi"])).toBe(null))).toBe(
      "hi\n",
    );
  });

  it("terminates on a self-referential array (cycle-guarded, CWE-674)", () => {
    // `a = []; a << a; puts a` in Ruby prints `[...]` and terminates.  Without
    // the cycle guard the element-per-line flatten recurses forever and throws
    // `RangeError: Maximum call stack size exceeded` (a DoS).  The guard must
    // both terminate AND render the cycle as `[...]`, matching Ruby.
    const a: unknown[] = [];
    a.push(a);
    expect(capture(() => sir.puts(a))).toBe("[...]\n");
  });

  it("terminates on a mutually-recursive array pair", () => {
    // Two arrays referencing each other (a -> b -> a) still forms a cycle on
    // the flatten path; both must render `[...]` at the back-reference rather
    // than diverging.  `puts a` flattens a's element (b), then b's element (a),
    // which is already on the path → `[...]`.
    const a: unknown[] = [];
    const b: unknown[] = [a];
    a.push(b);
    // a = [b], b = [a].  puts a: flatten a → element b (array, not seen) →
    // flatten b → element a (already on path) → `[...]`.
    expect(capture(() => sir.puts(a))).toBe("[...]\n");
  });
});

describe("arithmetic", () => {
  it("variadic", () => {
    expect(sir.add(1, 2, 3)).toBe(6);
    expect(sir.add()).toBe(0);
    expect(sir.sub(10, 3, 2)).toBe(5);
    expect(sir.sub(5)).toBe(-5);
    expect(sir.sub()).toBe(0);
    expect(sir.mul(2, 3, 4)).toBe(24);
    expect(sir.mul()).toBe(1);
  });

  it("truncating division", () => {
    expect(sir.div(7, 2)).toBe(3);
    expect(sir.div(-7, 2)).toBe(-3);
    expect(sir.div()).toBe(0);
    expect(sir.div(9, 3, 1)).toBe(3);
  });

  it("integer division by zero raises a typed ZeroDivisionError (T2)", () => {
    // Ruby `1 / 0` raises `ZeroDivisionError: divided by 0`; bare JS `/`
    // silently yields Infinity, so the runtime must ADD the check.
    expect(() => sir.div(1, 0)).toThrow(SirError);
    try {
      sir.div(1, 0);
    } catch (e) {
      expect((e as InstanceType<typeof SirError>).sirClass).toBe("ZeroDivisionError");
      expect((e as Error).message).toBe("divided by 0");
    }
  });

  it("float division by zero also raises ZeroDivisionError (T2)", () => {
    // Ruby raises for float `/` by 0 too (`1.0 / 0` → ZeroDivisionError);
    // JS would give Infinity. The divisor is 0 in both int and float cases.
    expect(() => sir.div(1.5, 0)).toThrow(SirError);
  });

  it("a zero divisor anywhere in the fold raises (T2)", () => {
    // The guard is inside the fold, so a trailing 0 divisor still raises.
    expect(() => sir.div(10, 2, 0)).toThrow(SirError);
  });

  it("a non-zero divisor never raises — regression", () => {
    expect(sir.div(6, 3)).toBe(2);
    expect(sir.div(0, 5)).toBe(0); // 0 as the DIVIDEND is fine
  });

  it("comparisons", () => {
    expect(sir.lt(1, 2)).toBe(true);
    expect(sir.gt(2, 1)).toBe(true);
  });
});

describe("polymorphic + (string/array concat)", () => {
  it("string concat", () => {
    expect(sir.add("a", "b")).toBe("ab");
    expect(sir.add("a", "b", "c")).toBe("abc"); // variadic fold
    expect(sir.add("n=", 1)).toBe("n=1"); // non-string operand rendered via display
  });

  it("array concat builds a fresh array (no aliasing)", () => {
    const a = [1];
    const b = [2];
    const r = sir.add(a, b) as unknown[];
    expect(r).toEqual([1, 2]);
    expect(r).not.toBe(a); // fresh — not the receiver
    expect(a).toEqual([1]); // inputs untouched
    expect(b).toEqual([2]);
  });

  it("array concat is variadic and does not rely on [] + []", () => {
    expect(sir.add([], []) as unknown[]).toEqual([]);
    expect(sir.add([1], [2], [3]) as unknown[]).toEqual([1, 2, 3]);
  });

  it("numeric + unchanged", () => {
    expect(sir.add(1, 2)).toBe(3);
  });
});

describe("polymorphic * (string/array repeat and join)", () => {
  it("string repeat", () => {
    expect(sir.mul("ab", 3)).toBe("ababab");
    expect(sir.mul("x", 0)).toBe(""); // non-positive count → empty
    expect(sir.mul("x", -2)).toBe(""); // negative → empty
    expect(sir.mul("x", 1.5)).toBe(""); // non-integer → empty
  });

  it("array repeat builds a fresh array", () => {
    expect(sir.mul([0], 3) as unknown[]).toEqual([0, 0, 0]);
    expect(sir.mul([1, 2], 2) as unknown[]).toEqual([1, 2, 1, 2]);
    expect(sir.mul([1], 0) as unknown[]).toEqual([]); // non-positive → empty
    expect(sir.mul([1], -1) as unknown[]).toEqual([]);
  });

  it("array join with a string separator", () => {
    expect(sir.mul([1, 2], ", ")).toBe("1, 2");
    expect(sir.mul([], ", ")).toBe(""); // empty array joins to empty string
  });

  it("numeric * unchanged", () => {
    expect(sir.mul(2, 3)).toBe(6);
  });

  it("rejects oversize repeat with a Ruby-shaped ArgumentError", () => {
    expect(() => sir.mul("ab", Number.MAX_SAFE_INTEGER)).toThrow("argument too big");
    expect(() => sir.mul([1, 2, 3], Number.MAX_SAFE_INTEGER)).toThrow("argument too big");
  });

  it("empty receiver short-circuits a huge count (no work, no throw)", () => {
    expect(sir.mul("", Number.MAX_SAFE_INTEGER)).toBe("");
    expect(sir.mul([], Number.MAX_SAFE_INTEGER) as unknown[]).toEqual([]);
  });
});

describe("closures, globals, dispatch", () => {
  it("makeClosure prepends captures", () => {
    const f = (a: sir.Val, b: sir.Val, c: sir.Val): sir.Val =>
      (a as number) + (b as number) + (c as number);
    const c = sir.makeClosure(f, [10, 20]);
    expect(sir.apply(c, [5])).toBe(35);
  });

  it("apply rejects non-closures", () => {
    expect(() => sir.apply(42, [])).toThrow(TypeError);
  });

  it("apply on a nil target raises LocalJumpError (no block given)", () => {
    // No-block-given case: a `yield` reached through a nil block parameter.
    expect(() => sir.apply(null, [1, 2])).toThrow(sir.LocalJumpError);
    expect(() => sir.apply(null, [])).toThrow(/no block given/);
    // Distinct from the non-closure TypeError so the two stay separable.
    expect(new sir.LocalJumpError()).not.toBeInstanceOf(TypeError);
  });

  it("global store roundtrip", () => {
    sir.globalSet("g1", 99);
    expect(sir.globalGet("g1")).toBe(99);
    expect(sir.globalGetStatic("g1")).toBe(99);
    sir.globalSet(sir.intern("g2"), 7);
    expect(sir.globalGet(sir.intern("g2"))).toBe(7);
  });

  it("undefined globals throw", () => {
    expect(() => sir.globalGet("nope")).toThrow();
    expect(() => sir.globalGetStatic("nope2")).toThrow();
  });

  it("callBuiltin and builtinClosure", () => {
    expect(sir.callBuiltin("+", [1, 2, 3])).toBe(6);
    expect(() => sir.callBuiltin("nope", [])).toThrow();
    const plus = sir.builtinClosure("+");
    expect(sir.apply(plus, [4, 5])).toBe(9);
  });
});

describe("doubleSplatMerge (call-position **h)", () => {
  it("merges maps left-to-right, later keys win", () => {
    const h1 = new Map<sir.Val, sir.Val>([
      ["a", 1],
      ["b", 2],
    ]);
    const h2 = new Map<sir.Val, sir.Val>([
      ["b", 3],
      ["c", 4],
    ]);
    const merged = sir.doubleSplatMerge(h1, h2);
    expect(merged.get("a")).toBe(1);
    expect(merged.get("b")).toBe(3); // h2 overwrites h1
    expect(merged.get("c")).toBe(4);
  });

  it("returns a fresh map (no aliasing of inputs)", () => {
    const h = new Map<sir.Val, sir.Val>([["k", 1]]);
    const merged = sir.doubleSplatMerge(h);
    merged.set("k", 99);
    expect(h.get("k")).toBe(1); // source untouched
  });

  it("merges zero maps into an empty map", () => {
    expect(sir.doubleSplatMerge().size).toBe(0);
  });

  it("preserves non-string Val keys (symbols)", () => {
    const sym = sir.intern("opt");
    const h = new Map<sir.Val, sir.Val>([[sym, 7]]);
    const merged = sir.doubleSplatMerge(h);
    expect(merged.get(sym)).toBe(7);
  });

  it("throws on a non-map operand (backend coverage gap)", () => {
    expect(() => sir.doubleSplatMerge(5 as unknown as sir.Val)).toThrow();
  });
});
