import { describe, it, expect, afterEach } from "vitest";
import { Pair, cons, car, cdr, isPair, setDisplay, type Val } from "../src/index.js";

// The default element renderer, so tests that swap it via setDisplay can put
// it back and leave the module in its pristine state for the next test.
const defaultDisplay = (v: Val): string => String(v);

afterEach(() => {
  setDisplay(defaultDisplay);
});

describe("cons / car / cdr", () => {
  it("builds a pair and round-trips its fields", () => {
    const p = cons(1, 2);
    expect(p).toBeInstanceOf(Pair);
    expect(car(p)).toBe(1);
    expect(cdr(p)).toBe(2);
    // Render here, in the very first test, while the module's *original*
    // default display arrow is still installed (the afterEach hook reinstalls a
    // test-local equivalent after each test, so this is the one place the
    // shipped default itself executes).
    expect(String(p)).toBe("(1 . 2)");
  });

  it("exposes car and cdr as readonly fields", () => {
    const p = cons("a", "b");
    expect(p.car).toBe("a");
    expect(p.cdr).toBe("b");
  });

  it("car throws a TypeError on a non-pair", () => {
    expect(() => car(42)).toThrow(TypeError);
    expect(() => car(42)).toThrow("car on non-pair");
  });

  it("cdr throws a TypeError on a non-pair", () => {
    expect(() => cdr(null)).toThrow(TypeError);
    expect(() => cdr(null)).toThrow("cdr on non-pair");
  });
});

describe("isPair", () => {
  it("is true for a pair", () => {
    expect(isPair(cons(1, 2))).toBe(true);
  });

  it("is false for non-pairs", () => {
    expect(isPair(1)).toBe(false);
    expect(isPair(null)).toBe(false);
    expect(isPair([1, 2])).toBe(false);
  });
});

describe("default display", () => {
  it("renders a proper list", () => {
    const p = cons(1, cons(2, cons(3, null)));
    expect(String(p)).toBe("(1 2 3)");
  });

  it("renders a single-element proper list", () => {
    expect(String(cons(1, null))).toBe("(1)");
  });

  it("renders a dotted (improper) pair", () => {
    expect(String(cons(1, 2))).toBe("(1 . 2)");
  });

  it("renders a nested pair", () => {
    const inner = cons(1, cons(2, null));
    const outer = cons(inner, cons(3, null));
    expect(String(outer)).toBe("((1 2) 3)");
  });

  it("renders an improper tail after several elements", () => {
    expect(String(cons(1, cons(2, 3)))).toBe("(1 2 . 3)");
  });
});

describe("setDisplay", () => {
  it("swaps the renderer used for each element", () => {
    // A renderer that maps null -> "nil" (Lisp-style) and otherwise wraps in
    // <...>. Proves the hook is consulted per element. (A null *tail* is always
    // omitted by the proper-list rule regardless of the hook, so we observe the
    // swap on a null *car* and on the body elements instead.)
    setDisplay((v: Val) => (v === null ? "nil" : `<${v}>`));
    expect(String(cons(null, 2))).toBe("(nil . <2>)");
    expect(String(cons(1, cons(2, null)))).toBe("(<1> <2>)");
  });

  it("default display is restored after the swap", () => {
    // afterEach put the default back: elements print via plain String, not the
    // custom <...> form.
    expect(String(cons(1, cons(2, null)))).toBe("(1 2)");
  });
});
