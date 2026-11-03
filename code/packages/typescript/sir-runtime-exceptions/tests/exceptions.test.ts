import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  SirError,
  raiseError,
  classOfThrown,
  registerAncestry,
  rescueMatches,
} from "../src/index.js";

describe("SirError", () => {
  it("tags the exception with its SIR class name", () => {
    const e = new SirError("ArgumentError", "bad arg");
    assert.equal(e.sirClass, "ArgumentError");
    assert.equal(e.message, "bad arg");
    assert.equal(e.name, "ArgumentError");
  });

  it("defaults the message to the class name when none is given", () => {
    assert.equal(new SirError("RuntimeError").message, "RuntimeError");
    assert.equal(new SirError("RuntimeError", null).message, "RuntimeError");
  });

  it("stringifies a non-string message", () => {
    assert.equal(new SirError("E", 42).message, "42");
  });

  it("is a real Error and an instanceof SirError", () => {
    const e = new SirError("E");
    assert.ok(e instanceof Error);
    assert.ok(e instanceof SirError);
  });
});

describe("raiseError", () => {
  it("throws a SirError of the named class with the message", () => {
    assert.throws(() => raiseError("TypeError", "nope"), SirError);
    try {
      raiseError("TypeError", "nope");
    } catch (e) {
      assert.equal((e as SirError).sirClass, "TypeError");
      assert.equal((e as SirError).message, "nope");
    }
  });

  it("defaults bare re-raise to RuntimeError", () => {
    try {
      raiseError();
    } catch (e) {
      assert.equal((e as SirError).sirClass, "RuntimeError");
    }
  });
});

describe("classOfThrown", () => {
  it("reports a SirError's class tag", () => {
    assert.equal(classOfThrown(new SirError("KeyError")), "KeyError");
  });

  it("buckets native errors and non-errors as StandardError", () => {
    assert.equal(classOfThrown(new Error("native")), "StandardError");
    assert.equal(classOfThrown("a string"), "StandardError");
    assert.equal(classOfThrown(undefined), "StandardError");
  });
});

describe("rescueMatches", () => {
  it("empty class list is a catch-all bare rescue", () => {
    assert.equal(rescueMatches(new SirError("Anything"), []), true);
    assert.equal(rescueMatches("not even an error", []), true);
  });

  it("matches the exact class name", () => {
    assert.equal(
      rescueMatches(new SirError("ArgumentError"), ["ArgumentError"]),
      true,
    );
  });

  it("matches an ancestor via the built-in hierarchy", () => {
    assert.equal(
      rescueMatches(new SirError("ArgumentError"), ["StandardError"]),
      true,
    );
    assert.equal(rescueMatches(new SirError("KeyError"), ["IndexError"]), true);
    assert.equal(
      rescueMatches(new SirError("NoMethodError"), ["NameError"]),
      true,
    );
  });

  it("Exception is the universal root and matches anything", () => {
    assert.equal(
      rescueMatches(new SirError("WhateverError"), ["Exception"]),
      true,
    );
    assert.equal(rescueMatches(new Error("native"), ["Exception"]), true);
  });

  it("StandardError catches native JS errors", () => {
    assert.equal(rescueMatches(new Error("native"), ["StandardError"]), true);
  });

  it("does not match an unrelated class", () => {
    assert.equal(
      rescueMatches(new SirError("TypeError"), ["ArgumentError"]),
      false,
    );
    assert.equal(
      rescueMatches(new SirError("RuntimeError"), ["KeyError"]),
      false,
    );
  });

  it("matches when any of several listed classes match", () => {
    assert.equal(
      rescueMatches(new SirError("TypeError"), ["KeyError", "TypeError"]),
      true,
    );
  });

  it("matches a user class only by exact name (no known ancestry)", () => {
    assert.equal(rescueMatches(new SirError("MyError"), ["MyError"]), true);
    assert.equal(
      rescueMatches(new SirError("MyError"), ["StandardError"]),
      false,
    );
  });
});

describe("registerAncestry (E2)", () => {
  // Each test uses distinct class names so a registration in one cannot leak
  // into another (the live ancestry table is module-global and mutable).
  it("threads a user edge so rescue matches a built-in ancestor", () => {
    // Before registration: matches only by exact name.
    assert.equal(rescueMatches(new SirError("E2Sub"), ["StandardError"]), false);
    registerAncestry({ E2Sub: "StandardError" });
    // After: it descends from StandardError and on up to Exception.
    assert.equal(rescueMatches(new SirError("E2Sub"), ["StandardError"]), true);
    assert.equal(rescueMatches(new SirError("E2Sub"), ["Exception"]), true);
    assert.equal(rescueMatches(new SirError("E2Sub"), ["E2Sub"]), true);
  });

  it("leaves an unrelated user class unmatched", () => {
    registerAncestry({ E2Known: "StandardError" });
    assert.equal(
      rescueMatches(new SirError("E2Unknown"), ["StandardError"]),
      false,
    );
    assert.equal(rescueMatches(new SirError("E2Known"), ["TypeError"]), false);
  });

  it("walks a multi-level user chain up into the built-in table", () => {
    registerAncestry({ E2Child: "RuntimeError", E2Grand: "E2Child" });
    assert.equal(rescueMatches(new SirError("E2Grand"), ["RuntimeError"]), true);
    assert.equal(rescueMatches(new SirError("E2Grand"), ["StandardError"]), true);
    assert.equal(rescueMatches(new SirError("E2Grand"), ["E2Child"]), true);
  });

  it("is additive: built-in edges are untouched by user registration", () => {
    registerAncestry({ E2Add: "StandardError" });
    assert.equal(
      rescueMatches(new SirError("ArgumentError"), ["StandardError"]),
      true,
    );
    assert.equal(rescueMatches(new SirError("E2Add"), ["StandardError"]), true);
  });

  it("does not loop on a self-referential edge (cycle guard)", () => {
    registerAncestry({ E2Loop: "E2Loop" });
    assert.equal(rescueMatches(new SirError("E2Loop"), ["E2Loop"]), true);
    assert.equal(rescueMatches(new SirError("E2Loop"), ["StandardError"]), false);
  });
});
