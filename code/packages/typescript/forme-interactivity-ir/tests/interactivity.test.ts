import { describe, expect, it } from "vitest";
import {
  DEFAULT_INTERACTIVITY_LIMITS,
  EMPTY_INTERACTIVITY,
  InteractivityError,
  canonicalInteractivityDocument,
  validateInteractivityDocument,
  type InteractivityDocument,
} from "../src/index.js";

function validDocument(): Record<string, unknown> {
  return {
    kind: "Interactivity",
    version: 1,
    state: [
      {
        name: "menu-open",
        scope: "document",
        type: "boolean",
        initial: false,
        persist: "none",
      },
      {
        name: "theme",
        scope: "session",
        type: "enum",
        initial: "system",
        persist: "session",
        values: ["system", "light", "dark"],
      },
    ],
    bindings: [
      {
        id: "menu-visibility",
        target: { kind: "element", id: "site-menu" },
        when: { kind: "truthy", value: { kind: "state", state: "menu-open" } },
        apply: { kind: "visible", whenTrue: true },
      },
    ],
    handlers: [
      {
        id: "toggle-menu",
        target: { kind: "element", id: "menu-button" },
        on: { kind: "click" },
        effects: [{ kind: "toggle-state", state: "menu-open" }],
      },
      {
        id: "enhance-search",
        target: { kind: "element", id: "search-form" },
        on: { kind: "focus" },
        effects: [{ kind: "run-island", island: "search", args: { mode: "quick" } }],
      },
    ],
    islands: [
      {
        id: "search",
        packageName: "@example/search-island",
        export: "mountSearch",
        target: { kind: "element", id: "search-form" },
        fallback: { kind: "element", id: "search-form" },
        activation: "interaction",
        config: { limit: 10 },
      },
    ],
  };
}

function expectCode(fn: () => unknown, code: string, path?: string): void {
  try {
    fn();
    throw new Error("expected validation to throw");
  } catch (error) {
    expect(error).toBeInstanceOf(InteractivityError);
    expect((error as InteractivityError).code).toBe(code);
    if (path !== undefined) expect((error as InteractivityError).path).toBe(path);
  }
}

describe("Interactivity IR validation", () => {
  it("accepts and deeply freezes the normative document", () => {
    const input = validDocument();
    const result = validateInteractivityDocument(input);
    expect(result.kind).toBe("Interactivity");
    expect(result.version).toBe(1);
    expect(result.state.map(entry => entry.name)).toEqual(["menu-open", "theme"]);
    expect(Object.isFrozen(result)).toBe(true);
    expect(Object.isFrozen(result.handlers)).toBe(true);
    expect(Object.isFrozen(result.handlers[1]!.effects[0])).toBe(true);
    expect(result).not.toBe(input);
  });

  it("exports a frozen empty zero-JavaScript document", () => {
    expect(EMPTY_INTERACTIVITY).toEqual({
      kind: "Interactivity",
      version: 1,
      state: [],
      bindings: [],
      handlers: [],
      islands: [],
    });
    expect(Object.isFrozen(EMPTY_INTERACTIVITY)).toBe(true);
    expect(Object.isFrozen(EMPTY_INTERACTIVITY.islands)).toBe(true);
  });

  it("snapshots caller data before later mutation", () => {
    const input = validDocument();
    const result = validateInteractivityDocument(input);
    (input.state as Array<Record<string, unknown>>)[0]!.name = "changed";
    ((input.islands as Array<Record<string, unknown>>)[0]!.config as Record<string, unknown>).limit = 999;
    expect(result.state[0]!.name).toBe("menu-open");
    expect(result.islands[0]!.config).toEqual({ limit: 10 });
  });

  it("rejects top-level shape and version drift", () => {
    expectCode(() => validateInteractivityDocument(null), "INVALID_TYPE", "$");
    expectCode(() => validateInteractivityDocument({ ...validDocument(), kind: "Other" }), "INVALID_VALUE", "$.kind");
    expectCode(() => validateInteractivityDocument({ ...validDocument(), version: 2 }), "INVALID_VALUE", "$.version");
    const missing = validDocument();
    delete missing.handlers;
    expectCode(() => validateInteractivityDocument(missing), "MISSING_FIELD", "$.handlers");
  });

  it("rejects unknown fields, accessors, symbols, and non-plain objects", () => {
    expectCode(() => validateInteractivityDocument({ ...validDocument(), surprise: true }), "UNKNOWN_FIELD", "$.surprise");

    const accessor = validDocument();
    Object.defineProperty(accessor, "kind", { enumerable: true, get: () => "Interactivity" });
    expectCode(() => validateInteractivityDocument(accessor), "ACCESSOR", "$.kind");

    const symbol = validDocument();
    Object.defineProperty(symbol, Symbol("hidden"), { enumerable: true, value: true });
    expectCode(() => validateInteractivityDocument(symbol), "SYMBOL_KEY", "$");

    const classed = Object.assign(Object.create({ inherited: true }), validDocument());
    expectCode(() => validateInteractivityDocument(classed), "INVALID_TYPE", "$");
  });

  it("rejects sparse arrays and repeated or cyclic object identity", () => {
    const sparse = validDocument();
    sparse.state = new Array(1);
    expectCode(() => validateInteractivityDocument(sparse), "SPARSE_ARRAY", "$.state[0]");

    const shared = { kind: "element", id: "shared" };
    const repeated = validDocument();
    (repeated.bindings as Array<Record<string, unknown>>)[0]!.target = shared;
    (repeated.handlers as Array<Record<string, unknown>>)[0]!.target = shared;
    expectCode(() => validateInteractivityDocument(repeated), "REPEATED_VALUE");

    const cyclic = validDocument();
    const config = (cyclic.islands as Array<Record<string, unknown>>)[0]!.config as Record<string, unknown>;
    config.self = config;
    expectCode(() => validateInteractivityDocument(cyclic), "REPEATED_VALUE");
  });

  it("validates identifiers and duplicate declaration IDs", () => {
    for (const bad of ["", "Menu", "two--parts", "with space", "a".repeat(65)]) {
      const input = validDocument();
      (input.state as Array<Record<string, unknown>>)[0]!.name = bad;
      expectCode(() => validateInteractivityDocument(input), "INVALID_IDENTIFIER", "$.state[0].name");
    }
    const duplicate = validDocument();
    (duplicate.state as Array<Record<string, unknown>>)[1]!.name = "menu-open";
    expectCode(() => validateInteractivityDocument(duplicate), "DUPLICATE_ID", "$.state[1].name");
  });

  it("requires block owners and forbids owners on broader state", () => {
    const missingOwner = validDocument();
    (missingOwner.state as Array<Record<string, unknown>>)[0]!.scope = "block";
    expectCode(() => validateInteractivityDocument(missingOwner), "MISSING_FIELD", "$.state[0].owner");

    const broadOwner = validDocument();
    (broadOwner.state as Array<Record<string, unknown>>)[0]!.owner = { kind: "element", id: "menu" };
    expectCode(() => validateInteractivityDocument(broadOwner), "UNKNOWN_FIELD", "$.state[0].owner");
  });

  it("validates state initials and enum domains", () => {
    const wrongBoolean = validDocument();
    (wrongBoolean.state as Array<Record<string, unknown>>)[0]!.initial = "false";
    expectCode(() => validateInteractivityDocument(wrongBoolean), "TYPE_MISMATCH", "$.state[0].initial");

    const nonFinite = validDocument();
    Object.assign((nonFinite.state as Array<Record<string, unknown>>)[0], {
      type: "number",
      initial: Number.NaN,
    });
    expectCode(() => validateInteractivityDocument(nonFinite), "INVALID_VALUE", "$.state[0].initial");

    const badEnum = validDocument();
    (badEnum.state as Array<Record<string, unknown>>)[1]!.initial = "sepia";
    expectCode(() => validateInteractivityDocument(badEnum), "TYPE_MISMATCH", "$.state[1].initial");

    const repeatedEnum = validDocument();
    (repeatedEnum.state as Array<Record<string, unknown>>)[1]!.values = ["system", "system"];
    expectCode(() => validateInteractivityDocument(repeatedEnum), "DUPLICATE_ID", "$.state[1].values[1]");
  });

  it("resolves predicate state references and bounds predicate lists", () => {
    const dangling = validDocument();
    (((dangling.bindings as Array<Record<string, unknown>>)[0]!.when as Record<string, unknown>).value as Record<string, unknown>).state = "missing";
    expectCode(() => validateInteractivityDocument(dangling), "DANGLING_REFERENCE");

    const emptyAll = validDocument();
    (emptyAll.bindings as Array<Record<string, unknown>>)[0]!.when = { kind: "all", predicates: [] };
    expectCode(() => validateInteractivityDocument(emptyAll), "INVALID_VALUE", "$.bindings[0].when.predicates");

    const tooMany = validDocument();
    (tooMany.bindings as Array<Record<string, unknown>>)[0]!.when = {
      kind: "any",
      predicates: Array.from({ length: 3 }, () => ({ kind: "truthy", value: { kind: "literal", value: true } })),
    };
    expectCode(
      () => validateInteractivityDocument(tooMany, { maxPredicateOperands: 2 }),
      "LIMIT_EXCEEDED",
      "$.bindings[0].when.predicates",
    );
  });

  it("bounds expression depth and total expression nodes", () => {
    let nested: Record<string, unknown> = { kind: "truthy", value: { kind: "literal", value: true } };
    for (let i = 0; i < 4; i++) nested = { kind: "not", predicate: nested };
    const deep = validDocument();
    (deep.bindings as Array<Record<string, unknown>>)[0]!.when = nested;
    expectCode(() => validateInteractivityDocument(deep, { maxExpressionDepth: 3 }), "LIMIT_EXCEEDED");

    const many = validDocument();
    (many.bindings as Array<Record<string, unknown>>)[0]!.when = {
      kind: "equals",
      left: { kind: "literal", value: true },
      right: { kind: "literal", value: true },
    };
    expectCode(() => validateInteractivityDocument(many, { maxExpressionNodes: 2 }), "LIMIT_EXCEEDED");
  });

  it("validates binding applications", () => {
    const badVisible = validDocument();
    (badVisible.bindings as Array<Record<string, unknown>>)[0]!.apply = { kind: "visible", whenTrue: "yes" };
    expectCode(() => validateInteractivityDocument(badVisible), "INVALID_TYPE");

    const text = validDocument();
    (text.bindings as Array<Record<string, unknown>>)[0]!.apply = {
      kind: "text",
      value: { kind: "literal", value: "Menu" },
    };
    expect(validateInteractivityDocument(text).bindings[0]!.apply.kind).toBe("text");

    const extension = validDocument();
    (extension.bindings as Array<Record<string, unknown>>)[0]!.apply = {
      kind: "extension",
      name: "aria-state",
      value: { expanded: true },
    };
    expect(validateInteractivityDocument(extension).bindings[0]!.apply.kind).toBe("extension");
  });

  it("validates trigger-specific fields and timer bounds", () => {
    const threshold = validDocument();
    (threshold.handlers as Array<Record<string, unknown>>)[0]!.on = { kind: "visible", threshold: 1.1 };
    expectCode(() => validateInteractivityDocument(threshold), "INVALID_VALUE");

    const timer = validDocument();
    (timer.handlers as Array<Record<string, unknown>>)[0]!.on = { kind: "timer", afterMs: 86_400_001 };
    expectCode(() => validateInteractivityDocument(timer), "INVALID_VALUE");

    const custom = validDocument();
    (custom.handlers as Array<Record<string, unknown>>)[0]!.on = { kind: "custom", name: "menu-ready" };
    expect(validateInteractivityDocument(custom).handlers[0]!.on.kind).toBe("custom");
  });

  it("requires effects and validates state effect types", () => {
    const empty = validDocument();
    (empty.handlers as Array<Record<string, unknown>>)[0]!.effects = [];
    expectCode(() => validateInteractivityDocument(empty), "INVALID_VALUE");

    const toggleEnum = validDocument();
    (toggleEnum.handlers as Array<Record<string, unknown>>)[0]!.effects = [{ kind: "toggle-state", state: "theme" }];
    expectCode(() => validateInteractivityDocument(toggleEnum), "TYPE_MISMATCH");

    const wrongLiteral = validDocument();
    (wrongLiteral.handlers as Array<Record<string, unknown>>)[0]!.effects = [{
      kind: "set-state",
      state: "menu-open",
      value: { kind: "literal", value: "yes" },
    }];
    expectCode(() => validateInteractivityDocument(wrongLiteral), "TYPE_MISMATCH");

    const incompatibleState = validDocument();
    (incompatibleState.handlers as Array<Record<string, unknown>>)[0]!.effects = [{
      kind: "set-state",
      state: "menu-open",
      value: { kind: "state", state: "theme" },
    }];
    expectCode(() => validateInteractivityDocument(incompatibleState), "TYPE_MISMATCH");

    const incompatibleEnum = validDocument();
    (incompatibleEnum.state as Array<Record<string, unknown>>).push({
      name: "accent",
      scope: "document",
      type: "enum",
      initial: "blue",
      persist: "none",
      values: ["blue", "red"],
    });
    (incompatibleEnum.handlers as Array<Record<string, unknown>>)[0]!.effects = [{
      kind: "set-state",
      state: "theme",
      value: { kind: "state", state: "accent" },
    }];
    expectCode(() => validateInteractivityDocument(incompatibleEnum), "TYPE_MISMATCH");
  });

  it("accepts safe navigation and rejects unsafe targets", () => {
    for (const to of ["/docs/start", "#details", "https://example.com/path?q=1"]) {
      const input = validDocument();
      (input.handlers as Array<Record<string, unknown>>)[0]!.effects = [{ kind: "navigate", to }];
      expect(validateInteractivityDocument(input).handlers[0]!.effects[0]).toMatchObject({ to });
    }
    for (const to of ["javascript:alert(1)", "//evil.example/x", "/a\\b", "https://user@example.com/x", "/a\u0000b"]) {
      const input = validDocument();
      (input.handlers as Array<Record<string, unknown>>)[0]!.effects = [{ kind: "navigate", to }];
      expectCode(() => validateInteractivityDocument(input), "INVALID_URL");
    }
  });

  it("validates island package/export/reference and interaction use", () => {
    const badPackage = validDocument();
    (badPackage.islands as Array<Record<string, unknown>>)[0]!.packageName = "../search";
    expectCode(() => validateInteractivityDocument(badPackage), "INVALID_IDENTIFIER");

    const badExport = validDocument();
    (badExport.islands as Array<Record<string, unknown>>)[0]!.export = "mount-search";
    expectCode(() => validateInteractivityDocument(badExport), "INVALID_IDENTIFIER");

    const documentTarget = validDocument();
    (documentTarget.islands as Array<Record<string, unknown>>)[0]!.target = { kind: "document" };
    expectCode(() => validateInteractivityDocument(documentTarget), "INVALID_VALUE");

    const unused = validDocument();
    (unused.handlers as Array<Record<string, unknown>>)[1]!.effects = [{ kind: "dispatch", event: "search-ready", detail: null }];
    expectCode(() => validateInteractivityDocument(unused), "DANGLING_REFERENCE", "$.islands[0].id");

    const dangling = validDocument();
    ((dangling.handlers as Array<Record<string, unknown>>)[1]!.effects as Array<Record<string, unknown>>)[0]!.island = "missing";
    expectCode(() => validateInteractivityDocument(dangling), "DANGLING_REFERENCE");
  });

  it("allows load and visible islands without a run-island handler", () => {
    for (const activation of ["load", "visible"]) {
      const input = validDocument();
      (input.islands as Array<Record<string, unknown>>)[0]!.activation = activation;
      (input.handlers as Array<Record<string, unknown>>)[1]!.effects = [{ kind: "dispatch", event: "search-ready", detail: null }];
      expect(validateInteractivityDocument(input).islands[0]!.activation).toBe(activation);
    }
  });

  it("validates and bounds JSON values", () => {
    expect(validateInteractivityDocument({
      kind: "Interactivity",
      version: 1,
      state: [],
      bindings: [],
      handlers: [],
      islands: [],
    }, { maxJsonNodes: 1 })).toEqual(EMPTY_INTERACTIVITY);

    const unsupported = validDocument();
    (unsupported.islands as Array<Record<string, unknown>>)[0]!.config = undefined;
    expectCode(() => validateInteractivityDocument(unsupported), "INVALID_TYPE");

    const sparse = validDocument();
    (sparse.islands as Array<Record<string, unknown>>)[0]!.config = new Array(1);
    expectCode(() => validateInteractivityDocument(sparse), "SPARSE_ARRAY");

    const deep = validDocument();
    (deep.islands as Array<Record<string, unknown>>)[0]!.config = { a: { b: { c: true } } };
    expectCode(() => validateInteractivityDocument(deep, { maxJsonDepth: 2 }), "LIMIT_EXCEEDED");

    const many = validDocument();
    (many.islands as Array<Record<string, unknown>>)[0]!.config = [1, 2, 3, 4];
    expectCode(() => validateInteractivityDocument(many, { maxJsonNodes: 3 }), "LIMIT_EXCEEDED");
  });

  it("enforces collection, string-byte, and canonical-byte limits", () => {
    expectCode(() => validateInteractivityDocument(validDocument(), { maxState: 1 }), "LIMIT_EXCEEDED", "$.state");

    const long = validDocument();
    (long.handlers as Array<Record<string, unknown>>)[0]!.id = "ééé";
    expectCode(() => validateInteractivityDocument(long, { maxStringBytes: 5 }), "LIMIT_EXCEEDED");

    expectCode(() => validateInteractivityDocument(validDocument(), { maxCanonicalBytes: 32 }), "LIMIT_EXCEEDED", "$");

    const enormousSparse = validDocument();
    enormousSparse.state = new Array(20_000);
    expectCode(() => validateInteractivityDocument(enormousSparse), "LIMIT_EXCEEDED", "$.state");
  });

  it("validates, snapshots, and freezes caller-provided limits", () => {
    const limits = { maxHandlers: 8 };
    const result = validateInteractivityDocument(validDocument(), limits);
    limits.maxHandlers = 0;
    expect(result.handlers).toHaveLength(2);
    expect(Object.isFrozen(DEFAULT_INTERACTIVITY_LIMITS)).toBe(true);

    for (const bad of [0, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1]) {
      expectCode(() => validateInteractivityDocument(validDocument(), { maxHandlers: bad }), "INVALID_LIMIT", "$.limits.maxHandlers");
    }

    let reads = 0;
    const accessor = Object.defineProperty({}, "maxHandlers", {
      enumerable: true,
      get() {
        reads++;
        return 8;
      },
    });
    expectCode(() => validateInteractivityDocument(validDocument(), accessor), "ACCESSOR", "$.limits.maxHandlers");
    expect(reads).toBe(0);
  });

  it("rejects malformed limit containers and properties", () => {
    expectCode(() => validateInteractivityDocument(validDocument(), null as never), "INVALID_TYPE", "$.limits");
    expectCode(() => validateInteractivityDocument(validDocument(), [] as never), "INVALID_TYPE", "$.limits");
    expectCode(() => validateInteractivityDocument(validDocument(), new (class Limits {})() as never), "INVALID_TYPE", "$.limits");

    const symbol = { maxHandlers: 8 };
    Object.defineProperty(symbol, Symbol("hidden"), { enumerable: true, value: 1 });
    expectCode(() => validateInteractivityDocument(validDocument(), symbol), "SYMBOL_KEY", "$.limits");
    expectCode(() => validateInteractivityDocument(validDocument(), { surprise: 1 } as never), "UNKNOWN_FIELD", "$.limits.surprise");

    const hidden = {};
    Object.defineProperty(hidden, "maxHandlers", { enumerable: false, value: 8 });
    expectCode(() => validateInteractivityDocument(validDocument(), hidden), "INVALID_LIMIT", "$.limits.maxHandlers");

    const tooMany = Object.fromEntries(Array.from({ length: 13 }, (_, index) => [`limit-${index}`, 1]));
    expectCode(() => validateInteractivityDocument(validDocument(), tooMany), "INVALID_LIMIT", "$.limits");
  });

  it("rejects hostile nested objects and arrays without invoking accessors", () => {
    const nonPlain = validDocument();
    (nonPlain.islands as Array<Record<string, unknown>>)[0]!.config = new Date();
    expectCode(() => validateInteractivityDocument(nonPlain), "INVALID_TYPE");

    let reads = 0;
    const accessorConfig = Object.defineProperty({}, "secret", {
      enumerable: true,
      get() {
        reads++;
        return true;
      },
    });
    const accessor = validDocument();
    (accessor.islands as Array<Record<string, unknown>>)[0]!.config = accessorConfig;
    expectCode(() => validateInteractivityDocument(accessor), "ACCESSOR");
    expect(reads).toBe(0);

    const extraArray = validDocument();
    const array = [true] as boolean[] & { note?: string };
    array.note = "hidden";
    (extraArray.islands as Array<Record<string, unknown>>)[0]!.config = array;
    expectCode(() => validateInteractivityDocument(extraArray), "UNKNOWN_FIELD");
  });

  it("rejects proxies before invoking their meta-traps", () => {
    let traps = 0;
    const proxied = new Proxy(validDocument(), {
      getPrototypeOf() {
        traps++;
        throw new Error("must not run");
      },
    });
    expectCode(() => validateInteractivityDocument(proxied), "INVALID_TYPE", "$");
    expect(traps).toBe(0);

    const limits = new Proxy({ maxHandlers: 8 }, {
      ownKeys() {
        traps++;
        throw new Error("must not run");
      },
    });
    expectCode(() => validateInteractivityDocument(validDocument(), limits), "INVALID_TYPE", "$.limits");
    expect(traps).toBe(0);

    const revocable = Proxy.revocable(validDocument(), {});
    revocable.revoke();
    expectCode(() => validateInteractivityDocument(revocable.proxy), "INVALID_TYPE", "$");

    const revokedLimits = Proxy.revocable({ maxHandlers: 8 }, {});
    revokedLimits.revoke();
    expectCode(() => validateInteractivityDocument(validDocument(), revokedLimits.proxy), "INVALID_TYPE", "$.limits");
  });

  it("snapshots prototype-shaped JSON keys without prototype mutation", () => {
    const config = Object.create(null) as Record<string, unknown>;
    Object.defineProperty(config, "__proto__", { enumerable: true, value: { polluted: true } });
    const input = validDocument();
    (input.islands as Array<Record<string, unknown>>)[0]!.config = config;
    const result = validateInteractivityDocument(input);
    expect(Object.getPrototypeOf(result.islands[0]!.config)).toBeNull();
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(canonicalInteractivityDocument(result)).toContain('"__proto__":{"polluted":true}');
  });

  it("rejects unknown discriminants and accepts document-wide targets", () => {
    const target = validDocument();
    (target.bindings as Array<Record<string, unknown>>)[0]!.target = { kind: "document" };
    expect(validateInteractivityDocument(target).bindings[0]!.target).toEqual({ kind: "document" });

    const badRef = validDocument();
    (badRef.bindings as Array<Record<string, unknown>>)[0]!.target = { kind: "selector", id: "x" };
    expectCode(() => validateInteractivityDocument(badRef), "INVALID_VALUE");

    const badExpr = validDocument();
    (badExpr.bindings as Array<Record<string, unknown>>)[0]!.when = {
      kind: "truthy",
      value: { kind: "call" },
    };
    expectCode(() => validateInteractivityDocument(badExpr), "INVALID_VALUE");

    const badEffect = validDocument();
    (badEffect.handlers as Array<Record<string, unknown>>)[0]!.effects = [{ kind: "fetch" }];
    expectCode(() => validateInteractivityDocument(badEffect), "INVALID_VALUE");
  });

  it("bounds and escapes attacker-controlled diagnostic paths", () => {
    const input = validDocument();
    const hostileKey = `bad\n\u001b${"x".repeat(200)}`;
    Object.defineProperty(input, hostileKey, { enumerable: true, value: true });
    try {
      validateInteractivityDocument(input);
      throw new Error("expected validation to throw");
    } catch (error) {
      expect(error).toBeInstanceOf(InteractivityError);
      const path = (error as InteractivityError).path;
      expect(path.length).toBeLessThan(300);
      expect(path).not.toContain("\n");
      expect(path).not.toContain("\u001b");
      expect(path).toContain("\\u000a\\u001b");
    }
  });

  it("rejects ill-formed Unicode strings", () => {
    const input = validDocument();
    (input.islands as Array<Record<string, unknown>>)[0]!.config = "\ud800";
    expectCode(() => validateInteractivityDocument(input), "INVALID_VALUE", "$.islands[0].config");
  });
});

describe("Interactivity IR canonical serialization", () => {
  it("emits the exact canonical empty document", () => {
    expect(canonicalInteractivityDocument(EMPTY_INTERACTIVITY)).toBe(
      '{"bindings":[],"handlers":[],"islands":[],"kind":"Interactivity","state":[],"version":1}',
    );
  });

  it("is independent of input object-key insertion order", () => {
    const firstRaw = validDocument();
    (firstRaw.islands as Array<Record<string, unknown>>)[0]!.config = { z: 1, limit: 10, a: 2 };
    const secondRaw = validDocument();
    (secondRaw.islands as Array<Record<string, unknown>>)[0]!.config = { a: 2, limit: 10, z: 1 };
    const first = validateInteractivityDocument(firstRaw);
    const second = validateInteractivityDocument(secondRaw);
    expect(canonicalInteractivityDocument(second)).toContain('"config":{"a":2,"limit":10,"z":1}');
    expect(canonicalInteractivityDocument(first)).toBe(canonicalInteractivityDocument(second));
  });

  it("preserves authored array order", () => {
    const document = validateInteractivityDocument(validDocument());
    const canonical = canonicalInteractivityDocument(document);
    expect(canonical.indexOf('"name":"menu-open"')).toBeLessThan(canonical.indexOf('"name":"theme"'));
    expect(canonical.indexOf('"id":"toggle-menu"')).toBeLessThan(canonical.indexOf('"id":"enhance-search"'));
  });

  it("accepts a validated document type", () => {
    const document: InteractivityDocument = validateInteractivityDocument(validDocument());
    expect(canonicalInteractivityDocument(document)).toMatch(/^\{"bindings":/);
  });

  it("orders object keys by Unicode scalar value", () => {
    const input = validDocument();
    (input.islands as Array<Record<string, unknown>>)[0]!.config = { "\u{10000}": 1, "\ue000": 2 };
    const canonical = canonicalInteractivityDocument(validateInteractivityDocument(input));
    expect(canonical.indexOf('"\ue000"')).toBeLessThan(canonical.indexOf('"\u{10000}"'));
  });

  it("refuses unvalidated typed objects without reading their fields", () => {
    let reads = 0;
    const raw = Object.defineProperty({}, "kind", {
      enumerable: true,
      get() {
        reads++;
        return "Interactivity";
      },
    }) as InteractivityDocument;
    expect(() => canonicalInteractivityDocument(raw)).toThrow(TypeError);
    expect(reads).toBe(0);
  });
});
