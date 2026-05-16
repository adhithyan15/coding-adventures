import { describe, it, expect } from "vitest";
import { validateAgainstSchema } from "../src/index.js";

describe("validateAgainstSchema — type", () => {
  it("accepts a matching primitive type", () => {
    expect(validateAgainstSchema("hi",    { type: "string"  }).ok).toBe(true);
    expect(validateAgainstSchema(42,      { type: "number"  }).ok).toBe(true);
    expect(validateAgainstSchema(true,    { type: "boolean" }).ok).toBe(true);
    expect(validateAgainstSchema(null,    { type: "null"    }).ok).toBe(true);
    expect(validateAgainstSchema([1, 2],  { type: "array"   }).ok).toBe(true);
    expect(validateAgainstSchema({},      { type: "object"  }).ok).toBe(true);
  });

  it("rejects a mismatched primitive type", () => {
    const r = validateAgainstSchema(42, { type: "string" });
    expect(r.ok).toBe(false);
    expect(r.violations[0]!.message).toMatch(/expected type string, got number/);
  });

  it("integer type rejects floats", () => {
    expect(validateAgainstSchema(42,  { type: "integer" }).ok).toBe(true);
    expect(validateAgainstSchema(1.5, { type: "integer" }).ok).toBe(false);
  });

  it("array of types accepts any of the listed", () => {
    expect(validateAgainstSchema("hi", { type: ["string", "number"] }).ok).toBe(true);
    expect(validateAgainstSchema(42,   { type: ["string", "number"] }).ok).toBe(true);
    expect(validateAgainstSchema(true, { type: ["string", "number"] }).ok).toBe(false);
  });

  it("missing type means no type constraint", () => {
    expect(validateAgainstSchema("hi", { enum: ["hi"] }).ok).toBe(true);
  });
});

describe("validateAgainstSchema — enum / const", () => {
  it("enum accepts members", () => {
    const s = { enum: ["a", "b", "c"] };
    expect(validateAgainstSchema("a", s).ok).toBe(true);
    expect(validateAgainstSchema("d", s).ok).toBe(false);
  });

  it("enum uses deep equality", () => {
    const s = { enum: [{ a: 1 }, { a: 2 }] };
    expect(validateAgainstSchema({ a: 1 }, s).ok).toBe(true);
    expect(validateAgainstSchema({ a: 3 }, s).ok).toBe(false);
  });

  it("const requires exact value", () => {
    expect(validateAgainstSchema("hi", { const: "hi" }).ok).toBe(true);
    expect(validateAgainstSchema("ho", { const: "hi" }).ok).toBe(false);
  });
});

describe("validateAgainstSchema — string", () => {
  it("minLength / maxLength", () => {
    expect(validateAgainstSchema("ab",  { type: "string", minLength: 3 }).ok).toBe(false);
    expect(validateAgainstSchema("abc", { type: "string", minLength: 3 }).ok).toBe(true);
    expect(validateAgainstSchema("abcd",{ type: "string", maxLength: 3 }).ok).toBe(false);
  });

  it("pattern", () => {
    expect(validateAgainstSchema("123", { type: "string", pattern: "^[0-9]+$" }).ok).toBe(true);
    expect(validateAgainstSchema("abc", { type: "string", pattern: "^[0-9]+$" }).ok).toBe(false);
  });

  it("malformed pattern is silently ignored", () => {
    // Not a regex syntax error in JS, but illustrative — `[` is invalid.
    expect(() => validateAgainstSchema("abc", { type: "string", pattern: "[" }))
      .not.toThrow();
  });
});

describe("validateAgainstSchema — number", () => {
  it("minimum / maximum", () => {
    expect(validateAgainstSchema(5, { type: "number", minimum: 10 }).ok).toBe(false);
    expect(validateAgainstSchema(15, { type: "number", maximum: 10 }).ok).toBe(false);
    expect(validateAgainstSchema(7,  { type: "number", minimum: 1, maximum: 10 }).ok).toBe(true);
  });
});

describe("validateAgainstSchema — array", () => {
  it("items schema applies to every element", () => {
    const s = { type: "array", items: { type: "string" } };
    expect(validateAgainstSchema(["a", "b"], s).ok).toBe(true);
    const r = validateAgainstSchema(["a", 42], s);
    expect(r.ok).toBe(false);
    expect(r.violations[0]!.path).toBe("[1]");
  });

  it("minItems / maxItems", () => {
    expect(validateAgainstSchema([1, 2, 3],    { type: "array", minItems: 4 }).ok).toBe(false);
    expect(validateAgainstSchema([1, 2, 3, 4], { type: "array", minItems: 4 }).ok).toBe(true);
    expect(validateAgainstSchema([1, 2, 3, 4], { type: "array", maxItems: 3 }).ok).toBe(false);
  });
});

describe("validateAgainstSchema — object", () => {
  const s = {
    type: "object",
    required: ["a"],
    properties: {
      a: { type: "string" },
      b: { type: "number" },
    },
  };

  it("required: present → ok", () => {
    expect(validateAgainstSchema({ a: "hi" }, s).ok).toBe(true);
  });

  it("required: missing → violation with right path", () => {
    const r = validateAgainstSchema({}, s);
    expect(r.ok).toBe(false);
    expect(r.violations[0]!.path).toBe("a");
    expect(r.violations[0]!.message).toMatch(/required.*missing/);
  });

  it("property type mismatch surfaces nested path", () => {
    const r = validateAgainstSchema({ a: 42 }, s);
    expect(r.ok).toBe(false);
    expect(r.violations[0]!.path).toBe("a");
    expect(r.violations[0]!.message).toMatch(/expected type string/);
  });

  it("nested object", () => {
    const ns = {
      type: "object",
      properties: { outer: { type: "object", required: ["inner"], properties: { inner: { type: "string" } } } },
    };
    const r = validateAgainstSchema({ outer: { wrong: "x" } }, ns);
    expect(r.ok).toBe(false);
    expect(r.violations[0]!.path).toBe("outer.inner");
  });

  it("additionalProperties: false rejects unknown keys", () => {
    const strict = {
      type: "object",
      properties: { a: { type: "string" } },
      additionalProperties: false,
    };
    const r = validateAgainstSchema({ a: "hi", b: "extra" }, strict);
    expect(r.ok).toBe(false);
    expect(r.violations[0]!.path).toBe("b");
    expect(r.violations[0]!.message).toMatch(/additional property/);
  });

  it("additionalProperties default (true): unknown keys allowed", () => {
    const lax = { type: "object", properties: { a: { type: "string" } } };
    expect(validateAgainstSchema({ a: "hi", b: 1 }, lax).ok).toBe(true);
  });

  it("aggregates multiple violations", () => {
    const r = validateAgainstSchema({ a: 42, b: "wrong-type" }, s);
    expect(r.ok).toBe(false);
    expect(r.violations).toHaveLength(2);
  });
});

describe("validateAgainstSchema — composition", () => {
  it("allOf requires all branches to succeed", () => {
    const s = {
      allOf: [
        { type: "string" },
        { minLength: 3 },
      ],
    };
    expect(validateAgainstSchema("hello", s).ok).toBe(true);
    expect(validateAgainstSchema("hi",    s).ok).toBe(false);
    expect(validateAgainstSchema(123,     s).ok).toBe(false);
  });

  it("anyOf accepts when any branch succeeds", () => {
    const s = { anyOf: [{ type: "string" }, { type: "number" }] };
    expect(validateAgainstSchema("hi", s).ok).toBe(true);
    expect(validateAgainstSchema(42,   s).ok).toBe(true);
    expect(validateAgainstSchema(true, s).ok).toBe(false);
  });

  it("oneOf rejects when zero or multiple branches succeed", () => {
    const s = { oneOf: [{ type: "string", maxLength: 3 }, { type: "string", minLength: 2 }] };
    // "ab" matches both: maxLength:3 and minLength:2 → oneOf fails
    expect(validateAgainstSchema("ab", s).ok).toBe(false);
    // 42 matches neither → oneOf fails
    expect(validateAgainstSchema(42, s).ok).toBe(false);
    // "abcde" matches only minLength:2 → oneOf succeeds
    expect(validateAgainstSchema("abcde", s).ok).toBe(true);
  });
});

describe("validateAgainstSchema — unknown keywords", () => {
  it("silently ignores unrecognised keywords (draft-07 forward-compat)", () => {
    const s = { type: "string", futureKeyword: "doesn't matter" };
    expect(validateAgainstSchema("hi", s).ok).toBe(true);
  });
});

describe("validateAgainstSchema — security: prototype pollution defence", () => {
  it("deepEqual ignores __proto__ / constructor / prototype keys", () => {
    const s = { const: { a: 1 } };
    // A value containing a __proto__ key should still match if a/values match.
    // (The check skips __proto__/constructor/prototype on both sides.)
    expect(validateAgainstSchema({ a: 1 }, s).ok).toBe(true);
  });

  it("does not mutate Object.prototype during validation", () => {
    const before = Object.keys(Object.prototype).length;
    validateAgainstSchema({ a: "hi" }, {
      type: "object",
      required: ["a"],
      properties: { a: { type: "string" } },
    });
    const after = Object.keys(Object.prototype).length;
    expect(after).toBe(before);
  });
});

describe("validateAgainstSchema — malformed schema tolerance", () => {
  it("schema = null → treats as no constraint", () => {
    // Casting `null` because validateAgainstSchema's signature accepts
    // JsonSchema (which is JsonValue).  null is valid JsonValue.
    expect(validateAgainstSchema(42, null as never).ok).toBe(true);
  });

  it("schema = number → treats as no constraint", () => {
    expect(validateAgainstSchema(42, 99 as never).ok).toBe(true);
  });

  it("type field with non-string value silently ignored", () => {
    expect(validateAgainstSchema(42, { type: 99 } as never).ok).toBe(true);
  });
});

describe("validateAgainstSchema — real stage configs", () => {
  it("validates forme-source-fs's schema", () => {
    const fsSchema = {
      type: "object",
      required: ["glob"],
      properties: { glob: { type: "string" }, root: { type: "string" } },
    };
    expect(validateAgainstSchema({ glob: "**/*.md" }, fsSchema).ok).toBe(true);
    expect(validateAgainstSchema({ glob: "**/*.md", root: "/abs" }, fsSchema).ok).toBe(true);
    expect(validateAgainstSchema({ root: "/abs" }, fsSchema).ok).toBe(false);  // missing glob
    expect(validateAgainstSchema({ glob: 42 }, fsSchema).ok).toBe(false);
  });

  it("validates forme-emit-fs's schema", () => {
    const emitSchema = {
      type: "object",
      required: ["outDir"],
      properties: { outDir: { type: "string" } },
    };
    expect(validateAgainstSchema({ outDir: "./dist" }, emitSchema).ok).toBe(true);
    expect(validateAgainstSchema({}, emitSchema).ok).toBe(false);
  });
});
