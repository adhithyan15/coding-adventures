/**
 * registry.test.ts — `createThemeRegistry` semantics.
 *
 * Invariants:
 * - register / lookup / list round-trip
 * - lookup of unknown name returns undefined
 * - list() is sorted lexicographically (deterministic)
 * - register replaces on duplicate name (hot-reload friendly)
 * - register refuses empty name and prototype-pollution names
 * - each createThemeRegistry() call yields an independent instance
 *   (no shared state)
 * - cyclic "theme references itself" via base.theme name is
 *   the orchestrator's concern, not the registry's — but the
 *   registry must not loop on its own lookup chain (it doesn't,
 *   because lookup never recurses)
 */

import { describe, it, expect } from "vitest";
import type { Theme } from "@coding-adventures/forme-style-ir";
import { createThemeRegistry } from "../src/index.js";

function t(name: string): Theme {
  return { name };
}

describe("createThemeRegistry — basic CRUD", () => {
  it("register + lookup round-trips a theme", () => {
    const reg = createThemeRegistry();
    const dark = t("dark");
    reg.register(dark);
    expect(reg.lookup("dark")).toBe(dark);
  });

  it("lookup of an unknown name returns undefined", () => {
    const reg = createThemeRegistry();
    expect(reg.lookup("nonexistent")).toBeUndefined();
  });

  it("list() returns sorted names", () => {
    const reg = createThemeRegistry();
    reg.register(t("zebra"));
    reg.register(t("alpha"));
    reg.register(t("mango"));
    expect(reg.list()).toEqual(["alpha", "mango", "zebra"]);
  });

  it("list() on an empty registry returns []", () => {
    const reg = createThemeRegistry();
    expect(reg.list()).toEqual([]);
  });

  it("list() result is frozen", () => {
    const reg = createThemeRegistry();
    reg.register(t("a"));
    const names = reg.list();
    expect(Object.isFrozen(names)).toBe(true);
  });
});

describe("createThemeRegistry — replace-on-duplicate", () => {
  it("re-registering the same name overwrites the previous theme", () => {
    const reg = createThemeRegistry();
    const v1: Theme = { name: "brand", rules: [] };
    const v2: Theme = { name: "brand", tokens: { colors: {} } };
    reg.register(v1);
    reg.register(v2);
    expect(reg.lookup("brand")).toBe(v2);
    // List still contains a single entry — no duplication.
    expect(reg.list()).toEqual(["brand"]);
  });
});

describe("createThemeRegistry — input validation", () => {
  it("rejects an empty theme name", () => {
    const reg = createThemeRegistry();
    expect(() => reg.register({ name: "" })).toThrow(/non-empty string/);
  });

  it("rejects a non-string theme name", () => {
    const reg = createThemeRegistry();
    expect(() => reg.register({ name: 42 as unknown as string })).toThrow(/non-empty string/);
  });

  it("refuses the forbidden name __proto__", () => {
    const reg = createThemeRegistry();
    expect(() => reg.register({ name: "__proto__" })).toThrow(/forbidden name/);
  });

  it("refuses the forbidden name constructor", () => {
    const reg = createThemeRegistry();
    expect(() => reg.register({ name: "constructor" })).toThrow(/forbidden name/);
  });

  it("refuses the forbidden name prototype", () => {
    const reg = createThemeRegistry();
    expect(() => reg.register({ name: "prototype" })).toThrow(/forbidden name/);
  });
});

describe("createThemeRegistry — isolation", () => {
  it("two registries are independent (no shared state)", () => {
    const a = createThemeRegistry();
    const b = createThemeRegistry();
    a.register(t("only-in-a"));
    b.register(t("only-in-b"));
    expect(a.lookup("only-in-b")).toBeUndefined();
    expect(b.lookup("only-in-a")).toBeUndefined();
  });
});

describe("createThemeRegistry — self-referential lookup is bounded", () => {
  it("a theme registered under its own name doesn't loop on lookup", () => {
    // The registry isn't reentrant — lookup is a single Map.get.
    // This test just pins that property: even if a malicious caller
    // shoves a theme whose `name` matches the lookup key (the normal
    // case) and whose body references the same name (irrelevant to
    // the registry — it doesn't follow references), lookup returns
    // in constant time without recursion.
    const reg = createThemeRegistry();
    const selfRef = { name: "self", refersTo: "self" } as unknown as Theme;
    reg.register(selfRef);
    // Two lookups in a row to "tempt" any latent recursion.
    expect(reg.lookup("self")).toBe(selfRef);
    expect(reg.lookup("self")).toBe(selfRef);
  });
});
