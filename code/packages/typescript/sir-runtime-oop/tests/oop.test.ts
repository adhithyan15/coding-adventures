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
