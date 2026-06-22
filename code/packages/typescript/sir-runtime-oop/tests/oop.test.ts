import { beforeEach, describe, expect, it } from "vitest";
import {
  callMethod,
  classOf,
  cvarGet,
  cvarSet,
  defineClass,
  defineMethod,
  isA,
  ivarGet,
  ivarSet,
  newInstance,
  popSelf,
  pushSelf,
  resetOop,
  SirInstance,
  superclassOf,
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
    expect(callMethod([1], "respond_to?", "map")).toBe(false);
  });

  it("unknown method returns nil, never throws", () => {
    expect(callMethod([1, 2, 3], "map")).toBeNull();
    expect(callMethod("hi", "upcase")).toBeNull();
    expect(callMethod(5, "times")).toBeNull();
  });
});
