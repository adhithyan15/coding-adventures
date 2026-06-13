import { describe, it, expect } from "vitest";
import {
  SirError,
  raiseError,
  classOfThrown,
  rescueMatches,
} from "../src/index.js";

describe("SirError", () => {
  it("tags the exception with its SIR class name", () => {
    const e = new SirError("ArgumentError", "bad arg");
    expect(e.sirClass).toBe("ArgumentError");
    expect(e.message).toBe("bad arg");
    expect(e.name).toBe("ArgumentError");
  });

  it("defaults the message to the class name when none is given", () => {
    expect(new SirError("RuntimeError").message).toBe("RuntimeError");
    expect(new SirError("RuntimeError", null).message).toBe("RuntimeError");
  });

  it("stringifies a non-string message", () => {
    expect(new SirError("E", 42).message).toBe("42");
  });

  it("is a real Error and an instanceof SirError", () => {
    const e = new SirError("E");
    expect(e).toBeInstanceOf(Error);
    expect(e).toBeInstanceOf(SirError);
  });
});

describe("raiseError", () => {
  it("throws a SirError of the named class with the message", () => {
    expect(() => raiseError("TypeError", "nope")).toThrow(SirError);
    try {
      raiseError("TypeError", "nope");
    } catch (e) {
      expect((e as SirError).sirClass).toBe("TypeError");
      expect((e as SirError).message).toBe("nope");
    }
  });

  it("defaults bare re-raise to RuntimeError", () => {
    try {
      raiseError();
    } catch (e) {
      expect((e as SirError).sirClass).toBe("RuntimeError");
    }
  });
});

describe("classOfThrown", () => {
  it("reports a SirError's class tag", () => {
    expect(classOfThrown(new SirError("KeyError"))).toBe("KeyError");
  });

  it("buckets native errors and non-errors as StandardError", () => {
    expect(classOfThrown(new Error("native"))).toBe("StandardError");
    expect(classOfThrown("a string")).toBe("StandardError");
    expect(classOfThrown(undefined)).toBe("StandardError");
  });
});

describe("rescueMatches", () => {
  it("empty class list is a catch-all bare rescue", () => {
    expect(rescueMatches(new SirError("Anything"), [])).toBe(true);
    expect(rescueMatches("not even an error", [])).toBe(true);
  });

  it("matches the exact class name", () => {
    expect(rescueMatches(new SirError("ArgumentError"), ["ArgumentError"])).toBe(
      true,
    );
  });

  it("matches an ancestor via the built-in hierarchy", () => {
    expect(rescueMatches(new SirError("ArgumentError"), ["StandardError"])).toBe(
      true,
    );
    expect(rescueMatches(new SirError("KeyError"), ["IndexError"])).toBe(true);
    expect(rescueMatches(new SirError("NoMethodError"), ["NameError"])).toBe(
      true,
    );
  });

  it("Exception is the universal root and matches anything", () => {
    expect(rescueMatches(new SirError("WhateverError"), ["Exception"])).toBe(
      true,
    );
    expect(rescueMatches(new Error("native"), ["Exception"])).toBe(true);
  });

  it("StandardError catches native JS errors", () => {
    expect(rescueMatches(new Error("native"), ["StandardError"])).toBe(true);
  });

  it("does not match an unrelated class", () => {
    expect(rescueMatches(new SirError("TypeError"), ["ArgumentError"])).toBe(
      false,
    );
    expect(rescueMatches(new SirError("RuntimeError"), ["KeyError"])).toBe(
      false,
    );
  });

  it("matches when any of several listed classes match", () => {
    expect(
      rescueMatches(new SirError("TypeError"), ["KeyError", "TypeError"]),
    ).toBe(true);
  });

  it("matches a user class only by exact name (no known ancestry)", () => {
    expect(rescueMatches(new SirError("MyError"), ["MyError"])).toBe(true);
    expect(rescueMatches(new SirError("MyError"), ["StandardError"])).toBe(
      false,
    );
  });
});
