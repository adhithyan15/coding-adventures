import { beforeEach, describe, expect, it } from "vitest";
import { apply, Closure, intern } from "@coding-adventures/sir-runtime-core";
import { SirError } from "@coding-adventures/sir-runtime-exceptions";
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
  extendModule,
  includeModule,
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

  it("class returns the class name; unknown methods raise NoMethodError (T2)", () => {
    expect(callMethod(newInstance("Foo"), "class")).toBe("Foo");
    expect(callMethod(3, "class")).toBe("Integer");
    // T2: a genuinely-unknown method now raises a typed NoMethodError
    // (previously it returned nil), so `rescue NoMethodError` catches it.
    expect(() => callMethod(3, "no_such_method")).toThrow(SirError);
    try {
      callMethod(3, "no_such_method");
    } catch (e) {
      expect((e as InstanceType<typeof SirError>).sirClass).toBe("NoMethodError");
      expect((e as Error).message).toBe("undefined method 'no_such_method' for Integer");
    }
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

  it("fetch returns the element in range, negative indices count from end (T2)", () => {
    expect(callMethod([10, 20, 30], "fetch", 0)).toBe(10);
    expect(callMethod([10, 20, 30], "fetch", 2)).toBe(30);
    expect(callMethod([10, 20, 30], "fetch", -1)).toBe(30); // last
    expect(callMethod([10, 20, 30], "fetch", -3)).toBe(10); // first
  });

  it("fetch out of bounds raises a typed IndexError (T2)", () => {
    // Ruby `arr.fetch(oob)` raises IndexError (unlike `arr[oob]`, which is nil).
    expect(() => callMethod([1, 2, 3], "fetch", 100)).toThrow(SirError);
    try {
      callMethod([1, 2, 3], "fetch", 100);
    } catch (e) {
      expect((e as InstanceType<typeof SirError>).sirClass).toBe("IndexError");
      expect((e as Error).message).toBe("index 100 outside of array bounds: -3...3");
    }
    // Negative OOB raises too.
    expect(() => callMethod([1, 2, 3], "fetch", -4)).toThrow(SirError);
  });

  it("fetch with an explicit default returns it instead of raising (T2)", () => {
    // Ruby `arr.fetch(oob, default)` returns the default — no raise.
    expect(callMethod([1, 2, 3], "fetch", 100, "def")).toBe("def");
    expect(callMethod([1, 2, 3], "fetch", -4, 0)).toBe(0);
  });

  it("take / drop split the array at a count clamped to [0, len]", () => {
    // Over-long counts saturate; a negative count folds to 0 (never-raise floor).
    expect(callMethod([1, 2, 3, 4], "take", 2)).toEqual([1, 2]);
    expect(callMethod([1, 2, 3, 4], "drop", 2)).toEqual([3, 4]);
    expect(callMethod([1, 2, 3], "take", 0)).toEqual([]);
    expect(callMethod([1, 2, 3], "drop", 0)).toEqual([1, 2, 3]);
    expect(callMethod([1, 2, 3], "take", 99)).toEqual([1, 2, 3]);
    expect(callMethod([1, 2, 3], "drop", 99)).toEqual([]);
    expect(callMethod([1, 2, 3], "take", -5)).toEqual([]);
    expect(callMethod([1, 2, 3], "drop", -5)).toEqual([1, 2, 3]);
  });

  it("values_at selects one element per index, folding negatives, nil OOB", () => {
    expect(callMethod([10, 20, 30], "values_at", 0, 2)).toEqual([10, 30]);
    expect(callMethod([10, 20, 30], "values_at", -1, -2)).toEqual([30, 20]);
    expect(callMethod([10, 20, 30], "values_at", 5, -9)).toEqual([null, null]);
    expect(callMethod([10, 20, 30], "values_at")).toEqual([]);
  });

  it("rotate wraps left (default 1) and right (negative n), any magnitude", () => {
    expect(callMethod([1, 2, 3, 4], "rotate")).toEqual([2, 3, 4, 1]);
    expect(callMethod([1, 2, 3, 4], "rotate", 2)).toEqual([3, 4, 1, 2]);
    expect(callMethod([1, 2, 3, 4], "rotate", -1)).toEqual([4, 1, 2, 3]);
    expect(callMethod([1, 2, 3, 4], "rotate", 6)).toEqual([3, 4, 1, 2]);
    expect(callMethod([1, 2, 3], "rotate", 0)).toEqual([1, 2, 3]);
    expect(callMethod([], "rotate", 3)).toEqual([]);
  });

  it("zip pads a shorter operand with nil and truncates to receiver length", () => {
    expect(callMethod([1, 2, 3], "zip", [4, 5, 6])).toEqual([[1, 4], [2, 5], [3, 6]]);
    expect(callMethod([1, 2, 3], "zip", [4, 5])).toEqual([[1, 4], [2, 5], [3, null]]);
    expect(callMethod([1, 2], "zip", [3, 4, 5])).toEqual([[1, 3], [2, 4]]);
    expect(callMethod([1, 2], "zip", [3, 4], [5, 6])).toEqual([[1, 3, 5], [2, 4, 6]]);
    expect(callMethod([1, 2], "zip", 99)).toEqual([[1, null], [2, null]]);
    expect(callMethod([1, 2], "zip")).toEqual([[1], [2]]);
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

  it("known block method WITHOUT a block bottoms out at nil (no over-raise, T2)", () => {
    // These names ARE in the catalog (respond_to? == true) — Ruby returns an
    // Enumerator when they're called block-less. We have no Enumerator in v0, so
    // the honest floor stays nil; crucially it must NOT raise NoMethodError,
    // since the method is not unknown.
    expect(callMethod([1, 2, 3], "map")).toBeNull(); // Array block method, no block
    expect(callMethod(5, "times")).toBeNull(); // Integer block method, no block
  });

  it("genuinely-unknown method raises NoMethodError (T2)", () => {
    // `scan` is not in the String catalog (needs a regex engine — later PR), so
    // it is a genuinely-unknown method → typed NoMethodError, not nil.
    expect(() => callMethod("hi", "scan")).toThrow(SirError);
    try {
      callMethod("hi", "scan");
    } catch (e) {
      expect((e as InstanceType<typeof SirError>).sirClass).toBe("NoMethodError");
      expect((e as Error).message).toBe("undefined method 'scan' for String");
    }
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

  it("array block-method breadth: sort_by/min_by/max_by/group_by/partition/…", () => {
    const id = new Closure((x: Val) => x);
    const even = new Closure((x: Val) => x % 2 === 0);
    expect(callMethod([3, 1, 2], "sort_by", id)).toEqual([1, 2, 3]);
    expect(callMethod(["aaa", "a", "aa"], "sort_by", new Closure((s: Val) => s.length))).toEqual([
      "a",
      "aa",
      "aaa",
    ]);
    expect(callMethod([3, 1, 2], "min_by", id)).toBe(1);
    expect(callMethod([3, 1, 2], "max_by", id)).toBe(3);
    expect(callMethod([], "min_by", id)).toBeNull();
    // group_by returns a Hash (Map).
    expect(callMethod([1, 2, 3, 4], "group_by", even)).toEqual(
      new Map<Val, Val>([
        [false, [1, 3]],
        [true, [2, 4]],
      ]),
    );
    expect(callMethod([1, 2, 3, 4], "partition", even)).toEqual([
      [2, 4],
      [1, 3],
    ]);
    expect(callMethod([1, 2], "collect_concat", new Closure((x: Val) => [x, x]))).toEqual([
      1, 1, 2, 2,
    ]);
    expect(callMethod([1, 2, 3, 4], "take_while", new Closure((x: Val) => x < 3))).toEqual([1, 2]);
    expect(callMethod([1, 2, 3, 4], "drop_while", new Closure((x: Val) => x < 3))).toEqual([3, 4]);
    expect(callMethod([1, 2, 3, 4], "count", even)).toBe(2);
    expect(
      callMethod([1, 2, 3], "each_with_object", [], new Closure((x: Val, o: Val) => o.push(x * 10))),
    ).toEqual([10, 20, 30]);
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
    // T2: a MISSING key with no default now raises KeyError (was nil).
    expect(() => callMethod(h, "fetch", "z")).toThrow(SirError);
    try {
      callMethod(h, "fetch", "z");
    } catch (e) {
      expect((e as InstanceType<typeof SirError>).sirClass).toBe("KeyError");
      expect((e as Error).message).toBe('key not found: "z"');
    }
    // An explicit default is still returned — no raise (Ruby semantics).
    expect(callMethod(h, "fetch", "z", 99)).toBe(99);
    expect(callMethod(h, "dig", "b")).toBe(2);
    expect(callMethod(h, "to_a")).toEqual([
      ["a", 1],
      ["b", 2],
    ]);
  });

  it("hash [] (index op) still returns nil on a miss — regression (T2)", () => {
    // Only `.fetch` raises KeyError; plain `hash[k]` (the index op) must still
    // return nil on a missing key. The index op does not route through
    // callMethod — this asserts the map get semantics `dig` mirrors.
    const h = new Map<Val, Val>([["a", 1]]);
    expect(callMethod(h, "dig", "missing")).toBeNull();
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
    // T2: an unknown Hash method now raises NoMethodError (was nil).
    expect(() => callMethod(h, "transform_keys")).toThrow(SirError);
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

  it("justify (ljust / rjust / center) and swapcase", () => {
    // pad to `width` runes with a cyclic pad; center's odd extra pad on the
    // RIGHT; width <= length is a no-op.
    expect(callMethod("hi", "ljust", 5)).toBe("hi   ");
    expect(callMethod("hi", "ljust", 5, "*")).toBe("hi***");
    expect(callMethod("hi", "rjust", 5, "*")).toBe("***hi");
    expect(callMethod("hi", "center", 6, "*")).toBe("**hi**");
    expect(callMethod("hi", "center", 5, "*")).toBe("*hi**");
    expect(callMethod("abc", "ljust", 1)).toBe("abc");
    expect(callMethod("abcdef", "ljust", 10, "xy")).toBe("abcdefxyxy");
    // swapcase flips each ASCII letter, leaving other characters untouched.
    expect(callMethod("Hello World", "swapcase")).toBe("hELLO wORLD");
    expect(callMethod("a1B!c", "swapcase")).toBe("A1b!C");
  });

  it("char-set methods (tr / count / delete / squeeze)", () => {
    // tr: positional translate; shorter `to` repeats its last char; empty `to`
    // deletes; last mapping wins on repeated `from` chars.
    expect(callMethod("hello", "tr", "el", "ip")).toBe("hippo");
    expect(callMethod("hello", "tr", "l", "r")).toBe("herro");
    expect(callMethod("hello", "tr", "aeiou", "*")).toBe("h*ll*");
    expect(callMethod("hello", "tr", "l", "")).toBe("heo");
    // A non-string arg (or missing `to`) is a no-op, holding the never-raise floor.
    expect(callMethod("hello", "tr", "l")).toBe("hello");
    // count / delete over a literal char set.
    expect(callMethod("hello", "count", "l")).toBe(2);
    expect(callMethod("hello", "count", "lo")).toBe(3);
    expect(callMethod("hello", "delete", "l")).toBe("heo");
    // Multiple set args INTERSECT (only chars in every set count/delete).
    expect(callMethod("hello", "count", "lo", "o")).toBe(1);
    // squeeze: bare form collapses every run; with a set only those runs.
    expect(callMethod("mississippi", "squeeze")).toBe("misisipi");
    expect(callMethod("aaabbbccc", "squeeze", "a")).toBe("abbbccc");
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
    // T2: an unknown String method now raises NoMethodError (was nil).
    expect(() => callMethod("x", "scan")).toThrow(SirError);
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

  it("numeric breadth (N1): round(ndigits) / divmod / fdiv / clamp / between?", () => {
    // round(ndigits): positive → decimals half-away; ndigits <= 0 → power of ten.
    expect(callMethod(3.14159, "round", 2)).toBe(3.14);
    expect(callMethod(1250, "round", -2)).toBe(1300); // half away from zero
    expect(callMethod(2.675, "round", 2)).toBe(2.68);
    // divmod: floored quotient, divisor-signed remainder.
    expect(callMethod(13, "divmod", 4)).toEqual([3, 1]);
    expect(callMethod(13, "divmod", -4)).toEqual([-4, -3]);
    // divmod by zero raises a typed ZeroDivisionError; a non-numeric divisor
    // degrades to 0 → the same typed error (never an untyped throw).
    expect(() => callMethod(1, "divmod", 0)).toThrow();
    expect(() => callMethod(1, "divmod", "x")).toThrow();
    // fdiv: float division that never throws (zero divisor → ±Infinity).
    expect(callMethod(7, "fdiv", 2)).toBe(3.5);
    expect(callMethod(1, "fdiv", 0)).toBe(Infinity);
    expect(callMethod(-1, "fdiv", 0)).toBe(-Infinity);
    // clamp / between?.
    expect(callMethod(5, "clamp", 1, 10)).toBe(5);
    expect(callMethod(-3, "clamp", 1, 10)).toBe(1);
    expect(callMethod(99, "clamp", 1, 10)).toBe(10);
    expect(callMethod(5, "between?", 1, 10)).toBe(true);
    expect(callMethod(0, "between?", 1, 10)).toBe(false);
    expect(callMethod(10, "between?", 1, 10)).toBe(true);
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
    // T2: an unknown numeric method now raises NoMethodError (was nil)...
    expect(() => callMethod(5, "bit_length")).toThrow(SirError);
    // ...but a KNOWN block method called block-less stays nil (Enumerator floor).
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
    // T2: `even?` is not on TrueClass → NoMethodError (was nil).
    expect(() => callMethod(true, "even?")).toThrow(SirError);
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

  it("symToProc raises NoMethodError for an out-of-catalog method (T2)", () => {
    // `&:no_such_method` re-enters dispatch; the unknown method now raises a
    // typed NoMethodError (was a silent nil) so `map(&:bad)` surfaces the fault.
    expect(() => apply(symToProc(intern("no_such_method")), [42])).toThrow(SirError);
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
    // An out-of-catalog name is respond_to? == false and (T2) raises
    // NoMethodError when actually called.
    expect(() => callMethod(true, "nonexistent_method")).toThrow(SirError);
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

// ── Mixins: include / extend / MRO (MX3) ─────────────────────────────────────
//
// A module registers its `def`s via `defMethod` keyed on the MODULE name
// (exactly as the frontend emits `__def_method__("M", …)`); `includeModule`
// then weaves the module into an owner's ancestry, and the method-resolution
// walk (`callMethod`) finds the mixed-in method.  These tests exercise the four
// spec-mandated behaviours end-to-end through the real dispatch path.
describe("mixins: include / extend / MRO (MX3)", () => {
  it("a module method included into a class is callable on an instance", () => {
    defineClass("Person", null);
    defMethod("Greeter", "greet", new Closure(() => "hello"));
    includeModule("Person", "Greeter");
    expect(callMethod(callNew("Person"), "greet")).toBe("hello");
  });

  it("a class method shadows an included module's method (class-first MRO)", () => {
    defineClass("Person", null);
    defMethod("Greeter", "greet", new Closure(() => "from module"));
    defMethod("Person", "greet", new Closure(() => "from class"));
    includeModule("Person", "Greeter");
    expect(callMethod(callNew("Person"), "greet")).toBe("from class");
  });

  it("the most recently included module wins (reverse include order)", () => {
    defineClass("C", null);
    defMethod("A", "who", new Closure(() => "A"));
    defMethod("B", "who", new Closure(() => "B"));
    includeModule("C", "A");
    includeModule("C", "B");
    expect(callMethod(callNew("C"), "who")).toBe("B");
  });

  it("a superclass's included module is reachable (module shadows superclass, class shadows module)", () => {
    // Base defines `rank`; a mixed-in module on the subclass shadows it; the
    // subclass's own method shadows the module — full class→module→super MRO.
    defineClass("Animal", null);
    defineClass("Dog", "Animal");
    defMethod("Animal", "rank", new Closure(() => "animal"));
    defMethod("Trainable", "rank", new Closure(() => "trainable"));
    includeModule("Dog", "Trainable");
    // module (on Dog) shadows the Animal superclass method.
    expect(callMethod(callNew("Dog"), "rank")).toBe("trainable");
    // adding a Dog-own method shadows the module in turn.
    defMethod("Dog", "rank", new Closure(() => "dog"));
    expect(callMethod(callNew("Dog"), "rank")).toBe("dog");
  });

  it("a diamond include resolves the shared module once (cycle/dedup guard)", () => {
    // C includes X and Y; both include Base.  Base#tag is found once and the
    // walk terminates — no infinite loop even though Base is reachable twice.
    defineClass("C", null);
    defMethod("Base", "tag", new Closure(() => "base"));
    includeModule("X", "Base");
    includeModule("Y", "Base");
    includeModule("C", "X");
    includeModule("C", "Y");
    expect(callMethod(callNew("C"), "tag")).toBe("base");
  });

  it("a self-including module terminates rather than looping", () => {
    defineClass("C", null);
    defMethod("Loopy", "ping", new Closure(() => "pong"));
    includeModule("Loopy", "Loopy"); // pathological self-include
    includeModule("C", "Loopy");
    expect(callMethod(callNew("C"), "ping")).toBe("pong");
    // an unresolved method still bottoms out (NoMethodError), not a hang.
    expect(() => callMethod(callNew("C"), "nope")).toThrow(SirError);
  });

  it("a re-included module is not duplicated (first position kept)", () => {
    defineClass("C", null);
    defMethod("M", "hi", new Closure(() => "hi"));
    includeModule("C", "M");
    includeModule("C", "M"); // repeat is a no-op
    expect(callMethod(callNew("C"), "hi")).toBe("hi");
  });

  it("extend makes a module's instance methods class methods on the owner", () => {
    defineClass("Widget", null);
    defMethod("Describable", "describe", new Closure(() => "a widget"));
    extendModule("Widget", "Describable");
    expect(callClassMethod("Widget", "describe")).toBe("a widget");
    // extend does NOT make it an instance method.
    expect(() => callMethod(callNew("Widget"), "describe")).toThrow(SirError);
  });

  it("extend copies every current module method into the class-method table", () => {
    defineClass("W", null);
    defMethod("Two", "a", new Closure(() => "a!"));
    defMethod("Two", "b", new Closure(() => "b!"));
    extendModule("W", "Two");
    expect(callClassMethod("W", "a")).toBe("a!");
    expect(callClassMethod("W", "b")).toBe("b!");
  });

  it("resetOop clears the included-modules table", () => {
    defineClass("Person", null);
    defMethod("Greeter", "greet", new Closure(() => "hi"));
    includeModule("Person", "Greeter");
    resetOop();
    // After reset, re-register only the class + module def (no include): the
    // mixed-in method must no longer resolve, proving the include list cleared.
    defineClass("Person", null);
    defMethod("Greeter", "greet", new Closure(() => "hi"));
    expect(() => callMethod(callNew("Person"), "greet")).toThrow(SirError);
  });
});
