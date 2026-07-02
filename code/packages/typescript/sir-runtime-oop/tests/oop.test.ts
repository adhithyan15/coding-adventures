import { beforeEach, describe, expect, it } from "vitest";
import { apply, Closure, intern } from "@coding-adventures/sir-runtime-core";
import type { Val } from "../src/index.js";
import {
  callClassMethod,
  callMethod,
  callNew,
  callSuper,
  caseEq,
  classOf,
  currentSelfVal,
  cvarGet,
  cvarSet,
  defClassMethod,
  defineClass,
  defineMethod,
  defMethod,
  isA,
  ivarGet,
  ivarSet,
  newInstance,
  popSelf,
  pushSelf,
  resetOop,
  SirInstance,
  superclassOf,
  symToProc,
} from "../src/index.js";

beforeEach(() => {
  resetOop();
});

describe("class registry + ancestry", () => {
  it("records superclass and resolves direct is_a?", () => {
    defineClass("Animal", null);
    defineClass("Dog", "Animal");
    expect(superclassOf("Dog")).toBe("Animal");
    expect(superclassOf("Animal")).toBeNull();
    const d = newInstance("Dog");
    expect(isA(d, "Dog")).toBe(true);
    expect(isA(d, "Animal")).toBe(true);
    expect(isA(d, "Cat")).toBe(false);
  });

  it("Object/BasicObject match everything; cycles terminate", () => {
    defineClass("A", "B");
    defineClass("B", "A"); // pathological cycle
    const a = newInstance("A");
    expect(isA(a, "Object")).toBe(true);
    expect(isA(a, "BasicObject")).toBe(true);
    expect(isA(a, "A")).toBe(true);
    expect(isA(a, "B")).toBe(true);
    expect(isA(a, "Z")).toBe(false);
  });

  it("re-defining a class replaces its registration", () => {
    defineClass("C", "X");
    defineClass("C", "Y");
    expect(superclassOf("C")).toBe("Y");
  });
});

describe("classOf primitive mapping", () => {
  it("maps JS values to Ruby class names", () => {
    expect(classOf(null)).toBe("NilClass");
    expect(classOf(undefined)).toBe("NilClass");
    expect(classOf(true)).toBe("TrueClass");
    expect(classOf(false)).toBe("FalseClass");
    expect(classOf(3)).toBe("Integer");
    expect(classOf(3.5)).toBe("Float");
    expect(classOf("hi")).toBe("String");
    expect(classOf([1, 2])).toBe("Array");
    expect(classOf(new Map())).toBe("Hash");
    expect(classOf(newInstance("Foo"))).toBe("Foo");
  });

  it("isA handles primitives and the Numeric umbrella", () => {
    expect(isA(3, "Integer")).toBe(true);
    expect(isA(3, "Numeric")).toBe(true);
    expect(isA(3.5, "Numeric")).toBe(true);
    expect(isA("x", "Numeric")).toBe(false);
    expect(isA("x", "String")).toBe(true);
  });
});

describe("instance-variable store via current-self stack", () => {
  it("reads nil before set; round-trips after set on the default self", () => {
    expect(ivarGet("@x")).toBeNull();
    expect(ivarSet("@x", 7)).toBe(7);
    expect(ivarGet("@x")).toBe(7);
  });

  it("push/pop self isolates instance variables per object", () => {
    const a = newInstance("Foo");
    const b = newInstance("Foo");
    pushSelf(a);
    ivarSet("@v", "a");
    popSelf();
    pushSelf(b);
    ivarSet("@v", "b");
    expect(ivarGet("@v")).toBe("b");
    popSelf();
    pushSelf(a);
    expect(ivarGet("@v")).toBe("a");
    popSelf();
  });
});

describe("class-variable store", () => {
  it("reads nil before set; round-trips after set", () => {
    expect(cvarGet("@@count")).toBeNull();
    expect(cvarSet("@@count", 0)).toBe(0);
    expect(cvarSet("@@count", 1)).toBe(1);
    expect(cvarGet("@@count")).toBe(1);
  });
});

describe("method dispatch", () => {
  it("is_a?/kind_of? accept a class-name string or a value", () => {
    defineClass("Animal", null);
    defineClass("Dog", "Animal");
    const d = newInstance("Dog");
    expect(callMethod(d, "is_a?", "Animal")).toBe(true);
    expect(callMethod(d, "kind_of?", "Dog")).toBe(true);
    expect(callMethod(3, "is_a?", "Integer")).toBe(true);
    // Class given as a value whose class is taken.
    expect(callMethod(3, "is_a?", 99)).toBe(true);
  });

  it("instance_of? requires an exact (non-ancestor) match", () => {
    defineClass("Animal", null);
    defineClass("Dog", "Animal");
    const d = newInstance("Dog");
    expect(callMethod(d, "instance_of?", "Dog")).toBe(true);
    expect(callMethod(d, "instance_of?", "Animal")).toBe(false);
  });

  it("class returns the class name; unknown methods return nil", () => {
    expect(callMethod(newInstance("Foo"), "class")).toBe("Foo");
    expect(callMethod(3, "class")).toBe("Integer");
    expect(callMethod(3, "no_such_method")).toBeNull();
  });

  it("defineMethod backs the dispatch fallback", () => {
    defineMethod("double", (recv) => (recv as number) * 2);
    expect(callMethod(21, "double")).toBe(42);
  });
});

describe("SirInstance", () => {
  it("carries its class tag and an empty ivar bag", () => {
    const i = new SirInstance("Widget");
    expect(i.sirClass).toBe("Widget");
    expect(i.ivars.size).toBe(0);
  });
});

describe("built-in method catalog: non-block Array (M1a)", () => {
  it("length / size / count", () => {
    expect(callMethod([1, 2, 3], "length")).toBe(3);
    expect(callMethod([1, 2, 3], "size")).toBe(3);
    expect(callMethod([1, 2, 3], "count")).toBe(3);
    expect(callMethod([1, 2, 2, 3], "count", 2)).toBe(2);
  });

  it("first / last with and without count", () => {
    expect(callMethod([1, 2, 3], "first")).toBe(1);
    expect(callMethod([1, 2, 3], "last")).toBe(3);
    expect(callMethod([1, 2, 3], "first", 2)).toEqual([1, 2]);
    expect(callMethod([1, 2, 3], "last", 2)).toEqual([2, 3]);
    expect(callMethod([], "first")).toBeNull();
    expect(callMethod([], "last")).toBeNull();
    expect(callMethod([1, 2], "last", 0)).toEqual([]);
  });

  it("include? / index (value equality)", () => {
    expect(callMethod([1, 2, 3], "include?", 2)).toBe(true);
    expect(callMethod([1, 2, 3], "include?", 9)).toBe(false);
    expect(callMethod([[1], [2]], "include?", [2])).toBe(true);
    expect(callMethod([1, 2, 3], "index", 3)).toBe(2);
    expect(callMethod([1, 2, 3], "index", 9)).toBeNull();
  });

  it("mutating push / << / pop / shift / unshift", () => {
    const a: number[] = [1, 2];
    expect(callMethod(a, "push", 3)).toEqual([1, 2, 3]);
    expect(a).toEqual([1, 2, 3]);
    expect(callMethod(a, "<<", 4)).toEqual([1, 2, 3, 4]);
    expect(callMethod(a, "pop")).toBe(4);
    expect(a).toEqual([1, 2, 3]);
    expect(callMethod(a, "shift")).toBe(1);
    expect(a).toEqual([2, 3]);
    expect(callMethod(a, "unshift", 0)).toEqual([0, 2, 3]);
  });

  it("reverse / sort / min / max / sum", () => {
    expect(callMethod([1, 2, 3], "reverse")).toEqual([3, 2, 1]);
    expect(callMethod([3, 1, 2], "sort")).toEqual([1, 2, 3]);
    expect(callMethod([10, 2, 30], "sort")).toEqual([2, 10, 30]);
    expect(callMethod([3, 1, 2], "min")).toBe(1);
    expect(callMethod([3, 1, 2], "max")).toBe(3);
    expect(callMethod([1, 2, 3], "sum")).toBe(6);
    expect(callMethod([1, 2, 3], "sum", 10)).toBe(16);
    expect(callMethod([], "min")).toBeNull();
  });

  it("reverse / sort are non-mutating", () => {
    const a = [1, 2, 3];
    expect(callMethod(a, "reverse")).toEqual([3, 2, 1]);
    expect(a).toEqual([1, 2, 3]);
  });

  it("uniq / flatten / compact / empty?", () => {
    expect(callMethod([1, 1, 2, 3, 3], "uniq")).toEqual([1, 2, 3]);
    expect(callMethod([1, [2, [3, 4]], 5], "flatten")).toEqual([1, 2, 3, 4, 5]);
    expect(callMethod([1, null, 2, null], "compact")).toEqual([1, 2]);
    expect(callMethod([], "empty?")).toBe(true);
    expect(callMethod([1], "empty?")).toBe(false);
  });
});

describe("built-in method catalog: universal Object (M1a)", () => {
  it("nil? / == / != / equal?", () => {
    expect(callMethod(null, "nil?")).toBe(true);
    expect(callMethod(0, "nil?")).toBe(false);
    expect(callMethod([1, 2], "==", [1, 2])).toBe(true);
    expect(callMethod([1, 2], "==", [1, 3])).toBe(false);
    expect(callMethod(1, "!=", 2)).toBe(true);
    const x = [1];
    expect(callMethod(x, "equal?", x)).toBe(true);
    expect(callMethod([1], "equal?", [1])).toBe(false);
  });

  it("dup / clone / itself / freeze / frozen?", () => {
    const a = [1, 2];
    const dup = callMethod(a, "dup");
    expect(dup).toEqual([1, 2]);
    expect(dup).not.toBe(a);
    expect(callMethod(5, "itself")).toBe(5);
    expect(callMethod(a, "freeze")).toBe(a);
    expect(callMethod(5, "frozen?")).toBe(true);
    expect(callMethod([1], "frozen?")).toBe(false);
  });

  it("to_a on nil and array", () => {
    expect(callMethod(null, "to_a")).toEqual([]);
    const a = [1, 2];
    expect(callMethod(a, "to_a")).toBe(a);
  });
});

describe("respond_to? honesty + nil floor (M1a)", () => {
  it("respond_to? reports catalog membership", () => {
    expect(callMethod([1], "respond_to?", "reverse")).toBe(true);
    expect(callMethod([1], "respond_to?", "nil?")).toBe(true);
    expect(callMethod([1], "respond_to?", "is_a?")).toBe(true);
    expect(callMethod([1], "respond_to?", "map")).toBe(true); // block method (M1b)
    expect(callMethod([1], "respond_to?", "each_slice")).toBe(false);
  });

  it("unknown method returns nil, never throws", () => {
    // A block method called WITHOUT a block bottoms out at nil (v0 floor).
    expect(callMethod([1, 2, 3], "map")).toBeNull();
    // An out-of-catalog String method (scan needs a regex engine — later PR).
    expect(callMethod("hi", "scan")).toBeNull();
    // Numeric has no catalog yet (M1c-Numeric), so every method is the nil floor.
    expect(callMethod(5, "times")).toBeNull();
  });
});

describe("built-in method catalog: block-taking Array/Enumerable (M1b)", () => {
  it("each runs the block and returns the receiver", () => {
    const seen: Val[] = [];
    const a = [1, 2, 3];
    const result = callMethod(a, "each", new Closure((x: Val) => seen.push(x)));
    expect(seen).toEqual([1, 2, 3]);
    expect(result).toBe(a);
  });

  it("each_with_index", () => {
    const pairs: Val[] = [];
    callMethod(["a", "b"], "each_with_index", new Closure((x: Val, i: Val) => pairs.push([x, i])));
    expect(pairs).toEqual([
      ["a", 0],
      ["b", 1],
    ]);
  });

  it("map/collect, select/filter, reject", () => {
    expect(callMethod([1, 2, 3], "map", new Closure((x: Val) => x * 2))).toEqual([2, 4, 6]);
    expect(callMethod([1, 2, 3], "collect", new Closure((x: Val) => x + 1))).toEqual([2, 3, 4]);
    expect(callMethod([1, 2, 3, 4], "select", new Closure((x: Val) => x % 2 === 0))).toEqual([2, 4]);
    expect(callMethod([1, 2, 3, 4], "filter", new Closure((x: Val) => x > 2))).toEqual([3, 4]);
    expect(callMethod([1, 2, 3, 4], "reject", new Closure((x: Val) => x % 2 === 0))).toEqual([1, 3]);
  });

  it("reduce/inject with and without initial", () => {
    expect(callMethod([1, 2, 3, 4], "reduce", new Closure((a: Val, b: Val) => a + b))).toBe(10);
    expect(callMethod([1, 2, 3], "inject", 100, new Closure((a: Val, b: Val) => a + b))).toBe(106);
    expect(callMethod([], "reduce", new Closure((a: Val, b: Val) => a + b))).toBeNull();
  });

  it("find/detect and flat_map", () => {
    expect(callMethod([1, 2, 3, 4], "find", new Closure((x: Val) => x > 2))).toBe(3);
    expect(callMethod([1, 2], "detect", new Closure((x: Val) => x > 9))).toBeNull();
    expect(callMethod([1, 2, 3], "flat_map", new Closure((x: Val) => [x, x * 10]))).toEqual([
      1, 10, 2, 20, 3, 30,
    ]);
  });

  it("any?/all?/none? use SIR truthiness", () => {
    expect(callMethod([1, 2, 3], "any?", new Closure((x: Val) => x > 2))).toBe(true);
    expect(callMethod([1, 2, 3], "any?", new Closure((x: Val) => x > 9))).toBe(false);
    expect(callMethod([2, 4, 6], "all?", new Closure((x: Val) => x % 2 === 0))).toBe(true);
    expect(callMethod([2, 3], "all?", new Closure((x: Val) => x % 2 === 0))).toBe(false);
    expect(callMethod([1, 2, 3], "none?", new Closure((x: Val) => x > 9))).toBe(true);
    expect(callMethod([1, 2, 3], "none?", new Closure((x: Val) => x > 2))).toBe(false);
  });

  it("select uses SIR truthiness (0 and '' are truthy)", () => {
    expect(callMethod([0, 1, null, 2], "select", new Closure((x: Val) => x))).toEqual([0, 1, 2]);
  });
});

describe("built-in method catalog: Hash (M1c)", () => {
  it("keys / values / size / empty?", () => {
    const h = new Map<Val, Val>([["a", 1], ["b", 2]]);
    expect(callMethod(h, "keys")).toEqual(["a", "b"]);
    expect(callMethod(h, "values")).toEqual([1, 2]);
    expect(callMethod(h, "size")).toBe(2);
    expect(callMethod(h, "length")).toBe(2);
    expect(callMethod(h, "empty?")).toBe(false);
    expect(callMethod(new Map(), "empty?")).toBe(true);
  });

  it("key/value membership", () => {
    const h = new Map<Val, Val>([["a", 1]]);
    expect(callMethod(h, "has_key?", "a")).toBe(true);
    expect(callMethod(h, "key?", "z")).toBe(false);
    expect(callMethod(h, "include?", "a")).toBe(true);
    expect(callMethod(h, "member?", "a")).toBe(true);
    expect(callMethod(h, "has_value?", 1)).toBe(true);
    expect(callMethod(h, "value?", 9)).toBe(false);
  });

  it("fetch / dig / to_a", () => {
    const h = new Map<Val, Val>([["a", 1], ["b", 2]]);
    expect(callMethod(h, "fetch", "a")).toBe(1);
    expect(callMethod(h, "fetch", "z")).toBeNull();
    expect(callMethod(h, "fetch", "z", 99)).toBe(99);
    expect(callMethod(h, "dig", "b")).toBe(2);
    expect(callMethod(h, "to_a")).toEqual([
      ["a", 1],
      ["b", 2],
    ]);
  });

  it("store / merge / delete / clear / invert", () => {
    const h = new Map<Val, Val>([["a", 1]]);
    expect(callMethod(h, "store", "b", 2)).toBe(2);
    expect(h.get("b")).toBe(2);
    expect(callMethod(h, "[]=", "c", 3)).toBe(3);
    const merged = callMethod(new Map<Val, Val>([["a", 1]]), "merge", new Map<Val, Val>([["b", 2]]));
    expect([...(merged as Map<Val, Val>).entries()]).toEqual([
      ["a", 1],
      ["b", 2],
    ]);
    expect(callMethod(h, "delete", "a")).toBe(1);
    expect(h.has("a")).toBe(false);
    const inv = callMethod(new Map<Val, Val>([["a", 1]]), "invert") as Map<Val, Val>;
    expect(inv.get(1)).toBe("a");
    const cleared = new Map<Val, Val>([["a", 1]]);
    expect((callMethod(cleared, "clear") as Map<Val, Val>).size).toBe(0);
  });

  it("block each / map / select / reject", () => {
    const seen: Val[] = [];
    const h = new Map<Val, Val>([["a", 1], ["b", 2]]);
    const result = callMethod(h, "each", new Closure((k: Val, v: Val) => seen.push([k, v])));
    expect(seen).toEqual([
      ["a", 1],
      ["b", 2],
    ]);
    expect(result).toBe(h);
    expect(callMethod(h, "map", new Closure((k: Val, v: Val) => `${k}=${v}`))).toEqual(["a=1", "b=2"]);
    const sel = callMethod(h, "select", new Closure((k: Val, v: Val) => v > 1)) as Map<Val, Val>;
    expect([...sel.entries()]).toEqual([["b", 2]]);
    const rej = callMethod(h, "reject", new Closure((k: Val, v: Val) => v > 1)) as Map<Val, Val>;
    expect([...rej.entries()]).toEqual([["a", 1]]);
  });

  it("each_key / each_value", () => {
    const ks: Val[] = [];
    const vs: Val[] = [];
    const h = new Map<Val, Val>([["a", 1], ["b", 2]]);
    callMethod(h, "each_key", new Closure((k: Val) => ks.push(k)));
    callMethod(h, "each_value", new Closure((v: Val) => vs.push(v)));
    expect(ks).toEqual(["a", "b"]);
    expect(vs).toEqual([1, 2]);
  });

  it("respond_to? honesty + nil floor", () => {
    const h = new Map<Val, Val>([["a", 1]]);
    expect(callMethod(h, "respond_to?", "keys")).toBe(true);
    expect(callMethod(h, "respond_to?", "each")).toBe(true);
    expect(callMethod(h, "respond_to?", "transform_keys")).toBe(false);
    expect(callMethod(h, "transform_keys")).toBeNull();
    expect(callMethod(h, "nil?")).toBe(false);
  });
});

describe("built-in method catalog: String (M1c)", () => {
  it("length / case / reverse", () => {
    expect(callMethod("hello", "length")).toBe(5);
    expect(callMethod("hello", "size")).toBe(5);
    expect(callMethod("hello", "upcase")).toBe("HELLO");
    expect(callMethod("HELLO", "downcase")).toBe("hello");
    expect(callMethod("hello world", "capitalize")).toBe("Hello world");
    expect(callMethod("abc", "reverse")).toBe("cba");
  });

  it("strip family and chomp", () => {
    expect(callMethod("  hi  ", "strip")).toBe("hi");
    expect(callMethod("  hi  ", "lstrip")).toBe("hi  ");
    expect(callMethod("  hi  ", "rstrip")).toBe("  hi");
    expect(callMethod("line\n", "chomp")).toBe("line");
    expect(callMethod("line\r\n", "chomp")).toBe("line");
    expect(callMethod("hello", "chomp", "lo")).toBe("hel");
    expect(callMethod("hello", "chomp")).toBe("hello");
  });

  it("chars / bytes / split", () => {
    expect(callMethod("abc", "chars")).toEqual(["a", "b", "c"]);
    expect(callMethod("AB", "bytes")).toEqual([65, 66]);
    expect(callMethod("a,b,c", "split", ",")).toEqual(["a", "b", "c"]);
    expect(callMethod("a  b\tc", "split")).toEqual(["a", "b", "c"]);
  });

  it("predicates and index", () => {
    expect(callMethod("hello", "include?", "ell")).toBe(true);
    expect(callMethod("hello", "start_with?", "he")).toBe(true);
    expect(callMethod("hello", "end_with?", "lo")).toBe(true);
    expect(callMethod("hello", "index", "l")).toBe(2);
    expect(callMethod("hello", "index", "z")).toBeNull();
    expect(callMethod("", "empty?")).toBe(true);
    expect(callMethod("x", "empty?")).toBe(false);
  });

  it("replace / sub / gsub are literal (no $& expansion)", () => {
    expect(callMethod("old", "replace", "new")).toBe("new");
    expect(callMethod("a.a.a", "sub", "a", "X")).toBe("X.a.a");
    expect(callMethod("a.a.a", "gsub", "a", "X")).toBe("X.X.X");
    // A replacement containing regex/backref syntax is inserted verbatim.
    expect(callMethod("ab", "gsub", "a", "$&")).toBe("$&b");
  });

  it("to_i / to_f / to_sym", () => {
    expect(callMethod("42abc", "to_i")).toBe(42);
    expect(callMethod("  -7", "to_i")).toBe(-7);
    expect(callMethod("nope", "to_i")).toBe(0);
    expect(callMethod("3.14xyz", "to_f")).toBe(3.14);
    expect(callMethod("nope", "to_f")).toBe(0);
    const sym = callMethod("name", "to_sym") as { name?: string };
    expect(sym.name).toBe("name");
  });

  it("repeat (*) and concat (+)", () => {
    expect(callMethod("ab", "*", 3)).toBe("ababab");
    expect(callMethod("foo", "+", "bar")).toBe("foobar");
    // Non-positive counts yield "" (never throw); a hostile count is capped,
    // never a RangeError ("Invalid string length").
    expect(callMethod("ab", "*", 0)).toBe("");
    expect(callMethod("ab", "*", -5)).toBe("");
    expect((callMethod("ab", "*", 1e9) as string).length).toBeLessThanOrEqual(100_000_000);
  });

  it("each_char runs the block and returns the receiver", () => {
    const seen: Val[] = [];
    const result = callMethod("abc", "each_char", new Closure((ch: Val) => seen.push(ch)));
    expect(seen).toEqual(["a", "b", "c"]);
    expect(result).toBe("abc");
  });

  it("respond_to? honesty + nil floor", () => {
    expect(callMethod("x", "respond_to?", "upcase")).toBe(true);
    expect(callMethod("x", "respond_to?", "each_char")).toBe(true);
    expect(callMethod("x", "respond_to?", "scan")).toBe(false);
    expect(callMethod("x", "scan")).toBeNull();
    expect(callMethod("x", "nil?")).toBe(false);
    expect(callMethod("x", "class")).toBe("String");
  });
});

describe("built-in method catalog: Numeric (Integer/Float) (M1c)", () => {
  it("predicates and sign", () => {
    expect(callMethod(4, "even?")).toBe(true);
    expect(callMethod(3, "odd?")).toBe(true);
    expect(callMethod(0, "zero?")).toBe(true);
    expect(callMethod(5, "positive?")).toBe(true);
    expect(callMethod(-5, "negative?")).toBe(true);
    expect(callMethod(-7, "abs")).toBe(7);
    expect(callMethod(-3.5, "abs")).toBe(3.5);
  });

  it("conversions and succ/pred", () => {
    expect(callMethod(3.9, "to_i")).toBe(3);
    expect(callMethod(4, "to_f")).toBe(4);
    expect(callMethod(5, "succ")).toBe(6);
    expect(callMethod(5, "next")).toBe(6);
    expect(callMethod(5, "pred")).toBe(4);
  });

  it("floor / ceil / round (half away from zero)", () => {
    expect(callMethod(3.2, "floor")).toBe(3);
    expect(callMethod(3.2, "ceil")).toBe(4);
    expect(callMethod(7, "floor")).toBe(7);
    expect(callMethod(2.5, "round")).toBe(3);
    expect(callMethod(-2.5, "round")).toBe(-3);
    expect(callMethod(5, "round")).toBe(5);
  });

  it("gcd / pow / digits", () => {
    expect(callMethod(12, "gcd", 18)).toBe(6);
    expect(callMethod(2, "**", 10)).toBe(1024);
    expect(callMethod(2, "pow", 5)).toBe(32);
    expect(callMethod(123, "digits")).toEqual([3, 2, 1]);
    expect(callMethod(0, "digits")).toEqual([0]);
  });

  it("to_s / inspect", () => {
    expect(callMethod(42, "to_s")).toBe("42");
    expect(callMethod(3.14, "to_s")).toBe("3.14");
    expect(callMethod(7, "inspect")).toBe("7");
  });

  it("block times / upto / downto / step", () => {
    const seen: Val[] = [];
    expect(callMethod(3, "times", new Closure((i: Val) => seen.push(i)))).toBe(3);
    expect(seen).toEqual([0, 1, 2]);
    const up: Val[] = [];
    callMethod(1, "upto", 4, new Closure((i: Val) => up.push(i)));
    expect(up).toEqual([1, 2, 3, 4]);
    const down: Val[] = [];
    callMethod(3, "downto", 1, new Closure((i: Val) => down.push(i)));
    expect(down).toEqual([3, 2, 1]);
    const step: Val[] = [];
    callMethod(0, "step", 10, 5, new Closure((i: Val) => step.push(i)));
    expect(step).toEqual([0, 5, 10]);
  });

  it("respond_to? honesty + nil floor", () => {
    expect(callMethod(5, "respond_to?", "even?")).toBe(true);
    expect(callMethod(5, "respond_to?", "times")).toBe(true);
    expect(callMethod(5, "respond_to?", "bit_length")).toBe(false);
    expect(callMethod(5, "bit_length")).toBeNull();
    expect(callMethod(5, "times")).toBeNull(); // block method without a block
  });
});

describe("built-in method catalog: Symbol (M1c)", () => {
  it("to_s / length / inspect / upcase / downcase", () => {
    const sym = callMethod("hello", "to_sym");
    expect(callMethod(sym, "to_s")).toBe("hello");
    expect(callMethod(sym, "length")).toBe(5);
    expect(callMethod(sym, "size")).toBe(5);
    expect(callMethod(sym, "inspect")).toBe(":hello");
    expect(callMethod(sym, "empty?")).toBe(false);
    expect((callMethod(sym, "upcase") as { name: string }).name).toBe("HELLO");
    const abc = callMethod("ABC", "to_sym");
    expect((callMethod(abc, "downcase") as { name: string }).name).toBe("abc");
    expect(callMethod(sym, "to_sym")).toBe(sym);
  });
});

describe("nil / true / false + Object to_s/inspect (M1c)", () => {
  it("nil / true / false display", () => {
    expect(callMethod(null, "to_s")).toBe("");
    expect(callMethod(null, "inspect")).toBe("nil");
    expect(callMethod(null, "to_a")).toEqual([]);
    expect(callMethod(true, "to_s")).toBe("true");
    expect(callMethod(false, "to_s")).toBe("false");
    expect(callMethod(true, "inspect")).toBe("true");
    // boolean resolves only Object methods, never the numeric catalog.
    expect(callMethod(true, "respond_to?", "even?")).toBe(false);
    expect(callMethod(true, "even?")).toBeNull();
  });

  it("Object to_s / inspect on collections and strings", () => {
    expect(callMethod([1, 2, 3], "to_s")).toBe("[1, 2, 3]");
    expect(callMethod(["a", "b"], "inspect")).toBe('["a", "b"]');
    expect(callMethod("hi", "inspect")).toBe('"hi"');
    expect(callMethod("hi", "to_s")).toBe("hi");
  });

  it("Array#join", () => {
    expect(callMethod([1, 2, 3], "join")).toBe("123");
    expect(callMethod([1, 2, 3], "join", "-")).toBe("1-2-3");
    expect(callMethod(["a", "b"], "join", ", ")).toBe("a, b");
  });

  it("digits on a non-finite number does not hang", () => {
    // `2 ** 1e9` saturates to Infinity in JS; digits must not spin forever.
    expect(callMethod(2 ** 1e9, "digits")).toEqual([0]);
    expect(callMethod(123, "digits")).toEqual([3, 2, 1]);
  });

  it("inspect handles cycles without overflowing the stack", () => {
    const a: Val[] = [];
    a.push(a); // self-referential array
    expect(callMethod(a, "inspect")).toBe("[[...]]");
    const m = new Map<Val, Val>();
    m.set("self", m);
    expect(callMethod(m, "inspect")).toBe('{"self"=>{...}}');
  });

  // ── Symbol#to_proc (&:sym) — M2 ─────────────────────────────────────────

  it("symToProc maps over an array via apply", () => {
    const proc = symToProc(intern("to_s"));
    expect(proc).toBeInstanceOf(Closure);
    // [1, 2, 3].map(&:to_s) — map drives the proc through apply with one arg.
    expect([1, 2, 3].map((x) => apply(proc, [x]))).toEqual(["1", "2", "3"]);
  });

  it("symToProc forwards extra args to the dispatched method", () => {
    // Two-arg apply binds the first as receiver, forwards the rest:
    // ["hello", "ell"] → "hello".include?("ell").
    const proc = symToProc(intern("include?"));
    expect(apply(proc, ["hello", "ell"])).toBe(true);
    expect(apply(proc, ["hello", "xyz"])).toBe(false);
  });

  it("symToProc accepts a bare string name", () => {
    expect(apply(symToProc("upcase" as unknown as Val), ["hi"])).toBe("HI");
  });

  it("symToProc bottoms out at nil for an out-of-catalog method", () => {
    expect(apply(symToProc(intern("no_such_method")), [42])).toBeNull();
  });

  it("symToProc drives array block-method dispatch end to end", () => {
    // [1, 2, 3].map(&:to_s) through callMethod.
    expect(callMethod([1, 2, 3], "map", symToProc(intern("to_s")))).toEqual([
      "1",
      "2",
      "3",
    ]);
  });
});

describe("case-equality (M5)", () => {
  it("matches a regex pattern against a string", () => {
    expect(caseEq(/ell/, "hello")).toBe(true);
    expect(caseEq(/ell/, "world")).toBe(false);
  });

  it("never matches a regex against a non-string scrutinee", () => {
    expect(caseEq(/1/, 1)).toBe(false);
  });

  it("tests Range membership (structural detection)", () => {
    // A stand-in named `Range` with an `includes` method exercises the path
    // without importing sir-runtime-range.
    class Range {
      constructor(
        private lo: number,
        private hi: number,
      ) {}
      includes(value: Val): boolean {
        return (value as number) >= this.lo && (value as number) <= this.hi;
      }
    }
    const r = new Range(1, 5) as unknown as Val;
    expect(caseEq(r, 3)).toBe(true);
    expect(caseEq(r, 9)).toBe(false);
  });

  it("falls back to value equality for a plain literal", () => {
    expect(caseEq(5, 5)).toBe(true);
    expect(caseEq(5, 6)).toBe(false);
    expect(caseEq("a", "a")).toBe(true);
  });
});

describe("Kernel flow-control + boolean operators (M6)", () => {
  it("send routes to a named method (string or Symbol)", () => {
    expect(callMethod("hello", "send", "upcase")).toBe("HELLO");
    expect(callMethod([3, 1, 2], "send", "sort")).toEqual([1, 2, 3]);
    // A Symbol method name (interned) routes identically.
    expect(callMethod("hi", "send", intern("upcase"))).toBe("HI");
    // `__send__` is the alias used when `send` itself is shadowed in real Ruby.
    expect(callMethod("hi", "__send__", "reverse")).toBe("ih");
    // Sanity: plain dispatch of a non-send method is unaffected.
    expect(callMethod("__send__", "size")).toBe(8);
  });

  it("send forwards arguments and a trailing block", () => {
    expect(callMethod("a,b,c", "send", "split", ",")).toEqual(["a", "b", "c"]);
    const seen: Val[] = [];
    callMethod([1, 2], "send", "each", new Closure((x: Val) => seen.push(x)));
    expect(seen).toEqual([1, 2]);
  });

  it("a user-defined send override wins (resolution order #2)", () => {
    defineMethod("send", () => "overridden");
    expect(callMethod("x", "send", "upcase")).toBe("overridden");
  });

  it("send without a method name is nil", () => {
    expect(callMethod("x", "send")).toBeNull();
  });

  it("tap yields the receiver and returns it", () => {
    const captured: Val[] = [];
    const result = callMethod([1, 2, 3], "tap", new Closure((x: Val) => captured.push(x)));
    expect(captured).toEqual([[1, 2, 3]]);
    expect(result).toEqual([1, 2, 3]);
    // Block-less tap returns the receiver (v0 floor).
    expect(callMethod(42, "tap")).toBe(42);
  });

  it("then/yield_self returns the block result", () => {
    expect(callMethod(5, "then", new Closure((x: Val) => x * 2))).toBe(10);
    expect(callMethod("hi", "yield_self", new Closure((s: Val) => s + "!"))).toBe("hi!");
    // Block-less then returns the receiver.
    expect(callMethod(7, "then")).toBe(7);
  });

  it("boolean & | ^ are eager logical operators", () => {
    expect(callMethod(true, "&", true)).toBe(true);
    expect(callMethod(true, "&", false)).toBe(false);
    expect(callMethod(false, "|", true)).toBe(true);
    expect(callMethod(true, "^", true)).toBe(false);
    expect(callMethod(true, "^", false)).toBe(true);
    // Ruby truthiness on the argument: null is falsy, 0/"" are truthy.
    expect(callMethod(true, "&", null)).toBe(false);
    expect(callMethod(false, "|", 0)).toBe(true);
    expect(callMethod(false, "|", "")).toBe(true);
  });

  it("respond_to? is honest for the Kernel + boolean surface", () => {
    expect(callMethod(1, "respond_to?", "tap")).toBe(true);
    expect(callMethod("x", "respond_to?", "then")).toBe(true);
    expect(callMethod([], "respond_to?", "send")).toBe(true);
    expect(callMethod(true, "respond_to?", "&")).toBe(true);
    // A non-bool receiver does not respond to the boolean operators.
    expect(callMethod(5, "respond_to?", "^")).toBe(false);
    // An out-of-catalog name is still both null and respond_to? == false.
    expect(callMethod(true, "nonexistent_method")).toBeNull();
    expect(callMethod(true, "respond_to?", "nonexistent_method")).toBe(false);
  });
});

describe("O1: user method tables, callNew / callSuper / self / class methods", () => {
  it("callNew runs initialize and binds self to the new object", () => {
    defineClass("Dog", null);
    defMethod("Dog", "initialize", new Closure((name: Val) => ivarSet("@name", name)));
    const dog = callNew("Dog", "Rex");
    expect(dog).toBeInstanceOf(SirInstance);
    expect(dog.sirClass).toBe("Dog");
    expect(dog.ivars.get("@name")).toBe("Rex");
    // Self-stack balanced after construction.
    expect(currentSelfVal()).toBeNull();
  });

  it("callNew without initialize is a plain allocation", () => {
    defineClass("Empty", null);
    const obj = callNew("Empty");
    expect(obj).toBeInstanceOf(SirInstance);
    expect(obj.ivars.size).toBe(0);
  });

  it("callNew inherits initialize from an ancestor", () => {
    defineClass("Base", null);
    defineClass("Derived", "Base");
    defMethod("Base", "initialize", new Closure((v: Val) => ivarSet("@v", v)));
    const obj = callNew("Derived", 7);
    expect(obj.sirClass).toBe("Derived");
    expect(obj.ivars.get("@v")).toBe(7);
  });

  it("callMethod dispatches a user instance method with self bound", () => {
    defineClass("Dog", null);
    defMethod("Dog", "initialize", new Closure((n: Val) => ivarSet("@name", n)));
    defMethod("Dog", "speak", new Closure(() => ivarGet("@name") + " says woof"));
    const dog = callNew("Dog", "Rex");
    expect(callMethod(dog, "speak")).toBe("Rex says woof");
    expect(currentSelfVal()).toBeNull();
  });

  it("callMethod walks ancestry for a user method", () => {
    defineClass("Animal", null);
    defineClass("Cat", "Animal");
    defMethod("Animal", "legs", new Closure(() => 4));
    const cat = callNew("Cat");
    expect(callMethod(cat, "legs")).toBe(4);
  });

  it("callMethod falls through to built-ins for instances (no regression)", () => {
    defineClass("Widget", null);
    const w = callNew("Widget");
    expect(callMethod(w, "class")).toBe("Widget");
    expect(callMethod(w, "is_a?", "Widget")).toBe(true);
    expect(callMethod(w, "nil?")).toBe(false);
  });

  it("callSuper walks to the parent implementation, same receiver", () => {
    defineClass("Animal", null);
    defineClass("Cat", "Animal");
    defMethod("Animal", "describe", new Closure(() => ivarGet("@name") + " with 4 legs"));
    defMethod("Cat", "initialize", new Closure((n: Val) => ivarSet("@name", n)));
    defMethod("Cat", "describe", new Closure(() => callSuper("describe", "Cat")));
    const cat = callNew("Cat", "Tom");
    expect(callMethod(cat, "describe")).toBe("Tom with 4 legs");
  });

  it("callSuper returns nil when no ancestor defines the method", () => {
    defineClass("Lonely", null);
    expect(callSuper("whatever", "Lonely")).toBeNull();
    defineClass("Base", null);
    defineClass("Sub", "Base");
    expect(callSuper("missing", "Sub")).toBeNull();
  });

  it("callClassMethod dispatches and walks ancestry", () => {
    defineClass("Counter", null);
    defClassMethod("Counter", "zero", new Closure(() => 0));
    expect(callClassMethod("Counter", "zero")).toBe(0);
    defineClass("Sub", "Counter");
    expect(callClassMethod("Sub", "zero")).toBe(0);
    expect(callClassMethod("Counter", "nope")).toBeNull();
  });

  it("currentSelfVal reflects the stack top", () => {
    expect(currentSelfVal()).toBeNull();
    const obj = newInstance("Thing");
    pushSelf(obj);
    expect(currentSelfVal()).toBe(obj);
    popSelf();
    expect(currentSelfVal()).toBeNull();
  });

  it("self-return enables method chaining", () => {
    defineClass("Counter", null);
    defMethod("Counter", "initialize", new Closure(() => ivarSet("@n", 0)));
    defMethod(
      "Counter",
      "inc",
      new Closure(() => {
        ivarSet("@n", ivarGet("@n") + 1);
        return currentSelfVal();
      }),
    );
    defMethod("Counter", "count", new Closure(() => ivarGet("@n")));
    const c = callNew("Counter");
    const chained = callMethod(callMethod(c, "inc"), "inc");
    expect(chained).toBe(c);
    expect(callMethod(c, "count")).toBe(2);
  });

  it("resetOop clears the method tables", () => {
    defineClass("Dog", null);
    defMethod("Dog", "speak", new Closure(() => "woof"));
    defClassMethod("Dog", "make", new Closure(() => "made"));
    resetOop();
    expect(callClassMethod("Dog", "make")).toBeNull();
    expect(callSuper("speak", "Dog")).toBeNull();
  });
});
