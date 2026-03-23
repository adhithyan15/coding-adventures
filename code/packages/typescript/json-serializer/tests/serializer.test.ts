/**
 * JSON Serializer -- Comprehensive Test Suite
 *
 * This test suite covers:
 *
 *   1. serialize() -- compact JsonValue to text
 *   2. serializePretty() -- pretty-printed output with config options
 *   3. stringify() / stringifyPretty() -- native types to text
 *   4. String escaping -- all RFC 8259 required escapes
 *   5. Error cases -- Infinity, NaN
 *   6. Full round-trip tests -- parse then serialize
 *
 * Coverage Target: 95%+
 */

import { describe, it, expect } from "vitest";

import {
  serialize,
  serializePretty,
  stringify,
  stringifyPretty,
  JsonSerializerError,
} from "../src/index.js";

import {
  jsonObject,
  jsonArray,
  jsonString,
  jsonNumber,
  jsonBool,
  jsonNull,
  parseNative,
  parse,
  type JsonValue,
} from "@coding-adventures/json-value";

// =============================================================================
// serialize() -- COMPACT MODE
// =============================================================================

describe("serialize (compact)", () => {
  // ---- Primitives ----

  it("serializes null", () => {
    expect(serialize(jsonNull())).toBe("null");
  });

  it("serializes true", () => {
    expect(serialize(jsonBool(true))).toBe("true");
  });

  it("serializes false", () => {
    expect(serialize(jsonBool(false))).toBe("false");
  });

  it("serializes integer number", () => {
    expect(serialize(jsonNumber(42))).toBe("42");
  });

  it("serializes negative number", () => {
    expect(serialize(jsonNumber(-5))).toBe("-5");
  });

  it("serializes zero", () => {
    expect(serialize(jsonNumber(0))).toBe("0");
  });

  it("serializes float number", () => {
    expect(serialize(jsonNumber(3.14))).toBe("3.14");
  });

  it("serializes simple string", () => {
    expect(serialize(jsonString("hello"))).toBe('"hello"');
  });

  it("serializes empty string", () => {
    expect(serialize(jsonString(""))).toBe('""');
  });

  // ---- String Escaping ----

  it("escapes newline in string", () => {
    expect(serialize(jsonString("a\nb"))).toBe('"a\\nb"');
  });

  it("escapes tab in string", () => {
    expect(serialize(jsonString("\t"))).toBe('"\\t"');
  });

  it("escapes double quote in string", () => {
    expect(serialize(jsonString('say "hi"'))).toBe('"say \\"hi\\""');
  });

  it("escapes backslash in string", () => {
    expect(serialize(jsonString("a\\b"))).toBe('"a\\\\b"');
  });

  it("escapes carriage return", () => {
    expect(serialize(jsonString("\r"))).toBe('"\\r"');
  });

  it("escapes backspace", () => {
    expect(serialize(jsonString("\b"))).toBe('"\\b"');
  });

  it("escapes form feed", () => {
    expect(serialize(jsonString("\f"))).toBe('"\\f"');
  });

  it("escapes null byte as \\u0000", () => {
    expect(serialize(jsonString("\x00"))).toBe('"\\u0000"');
  });

  it("escapes control character U+001F", () => {
    expect(serialize(jsonString("\x1f"))).toBe('"\\u001f"');
  });

  it("does NOT escape forward slash", () => {
    /**
     * RFC 8259 allows escaping / but does not require it.
     * We choose not to escape it for readability.
     */
    expect(serialize(jsonString("a/b"))).toBe('"a/b"');
  });

  it("preserves non-ASCII Unicode characters", () => {
    expect(serialize(jsonString("\u00e9"))).toBe('"\u00e9"');
  });

  // ---- Containers ----

  it("serializes empty object", () => {
    expect(serialize(jsonObject())).toBe("{}");
  });

  it("serializes simple object", () => {
    const obj = jsonObject([["a", jsonNumber(1)]]);
    expect(serialize(obj)).toBe('{"a":1}');
  });

  it("serializes multi-key object", () => {
    const obj = jsonObject([
      ["a", jsonNumber(1)],
      ["b", jsonNumber(2)],
    ]);
    expect(serialize(obj)).toBe('{"a":1,"b":2}');
  });

  it("serializes empty array", () => {
    expect(serialize(jsonArray())).toBe("[]");
  });

  it("serializes simple array", () => {
    const arr = jsonArray([jsonNumber(1)]);
    expect(serialize(arr)).toBe("[1]");
  });

  it("serializes multi-element array", () => {
    const arr = jsonArray([jsonNumber(1), jsonNumber(2), jsonNumber(3)]);
    expect(serialize(arr)).toBe("[1,2,3]");
  });

  it("serializes nested object", () => {
    const obj = jsonObject([
      [
        "a",
        jsonObject([["b", jsonNumber(1)]]),
      ],
    ]);
    expect(serialize(obj)).toBe('{"a":{"b":1}}');
  });

  it("serializes nested array", () => {
    const arr = jsonArray([
      jsonArray([jsonNumber(1), jsonNumber(2)]),
      jsonArray([jsonNumber(3), jsonNumber(4)]),
    ]);
    expect(serialize(arr)).toBe("[[1,2],[3,4]]");
  });

  it("serializes complex nested structure", () => {
    const value = jsonObject([
      [
        "users",
        jsonArray([
          jsonObject([
            ["name", jsonString("Alice")],
            ["age", jsonNumber(30)],
            ["active", jsonBool(true)],
          ]),
        ]),
      ],
      ["count", jsonNumber(1)],
    ]);
    expect(serialize(value)).toBe(
      '{"users":[{"name":"Alice","age":30,"active":true}],"count":1}'
    );
  });

  // ---- Error Cases ----

  it("throws on Infinity", () => {
    expect(() => serialize(jsonNumber(Infinity, false))).toThrow(
      JsonSerializerError
    );
  });

  it("throws on -Infinity", () => {
    expect(() => serialize(jsonNumber(-Infinity, false))).toThrow(
      JsonSerializerError
    );
  });

  it("throws on NaN", () => {
    expect(() => serialize(jsonNumber(NaN, false))).toThrow(
      JsonSerializerError
    );
  });
});

// =============================================================================
// serializePretty() -- PRETTY MODE
// =============================================================================

describe("serializePretty", () => {
  it("pretty-prints empty object as {}", () => {
    expect(serializePretty(jsonObject())).toBe("{}");
  });

  it("pretty-prints simple object with 2-space indent", () => {
    const obj = jsonObject([["a", jsonNumber(1)]]);
    expect(serializePretty(obj)).toBe('{\n  "a": 1\n}');
  });

  it("pretty-prints multi-key object", () => {
    const obj = jsonObject([
      ["a", jsonNumber(1)],
      ["b", jsonNumber(2)],
    ]);
    expect(serializePretty(obj)).toBe('{\n  "a": 1,\n  "b": 2\n}');
  });

  it("pretty-prints nested object with increasing indentation", () => {
    const obj = jsonObject([
      [
        "outer",
        jsonObject([["inner", jsonNumber(1)]]),
      ],
    ]);
    expect(serializePretty(obj)).toBe(
      '{\n  "outer": {\n    "inner": 1\n  }\n}'
    );
  });

  it("pretty-prints empty array as []", () => {
    expect(serializePretty(jsonArray())).toBe("[]");
  });

  it("pretty-prints simple array", () => {
    const arr = jsonArray([jsonNumber(1), jsonNumber(2)]);
    expect(serializePretty(arr)).toBe("[\n  1,\n  2\n]");
  });

  it("pretty-prints nested array", () => {
    const arr = jsonArray([
      jsonArray([jsonNumber(1)]),
      jsonArray([jsonNumber(2)]),
    ]);
    expect(serializePretty(arr)).toBe(
      "[\n  [\n    1\n  ],\n  [\n    2\n  ]\n]"
    );
  });

  it("pretty-prints primitives same as compact", () => {
    expect(serializePretty(jsonNull())).toBe("null");
    expect(serializePretty(jsonBool(true))).toBe("true");
    expect(serializePretty(jsonBool(false))).toBe("false");
    expect(serializePretty(jsonNumber(42))).toBe("42");
    expect(serializePretty(jsonString("hi"))).toBe('"hi"');
  });

  // ---- Configuration Options ----

  it("uses 4-space indent when configured", () => {
    const obj = jsonObject([["a", jsonNumber(1)]]);
    expect(serializePretty(obj, { indentSize: 4 })).toBe(
      '{\n    "a": 1\n}'
    );
  });

  it("uses tab indent when configured", () => {
    const obj = jsonObject([["a", jsonNumber(1)]]);
    expect(serializePretty(obj, { indentChar: "\t", indentSize: 1 })).toBe(
      '{\n\t"a": 1\n}'
    );
  });

  it("sorts keys alphabetically when configured", () => {
    const obj = jsonObject([
      ["c", jsonNumber(3)],
      ["a", jsonNumber(1)],
      ["b", jsonNumber(2)],
    ]);
    expect(serializePretty(obj, { sortKeys: true })).toBe(
      '{\n  "a": 1,\n  "b": 2,\n  "c": 3\n}'
    );
  });

  it("preserves insertion order when sortKeys is false", () => {
    const obj = jsonObject([
      ["c", jsonNumber(3)],
      ["a", jsonNumber(1)],
    ]);
    expect(serializePretty(obj, { sortKeys: false })).toBe(
      '{\n  "c": 3,\n  "a": 1\n}'
    );
  });

  it("adds trailing newline when configured", () => {
    const obj = jsonObject([["a", jsonNumber(1)]]);
    const result = serializePretty(obj, { trailingNewline: true });
    expect(result).toBe('{\n  "a": 1\n}\n');
  });

  it("no trailing newline by default", () => {
    const result = serializePretty(jsonNull());
    expect(result).toBe("null");
    expect(result.endsWith("\n")).toBe(false);
  });

  it("trailing newline on primitives", () => {
    expect(serializePretty(jsonNull(), { trailingNewline: true })).toBe(
      "null\n"
    );
  });
});

// =============================================================================
// stringify() and stringifyPretty() -- NATIVE TO TEXT
// =============================================================================

describe("stringify", () => {
  it("stringifies object", () => {
    expect(stringify({ a: 1 })).toBe('{"a":1}');
  });

  it("stringifies array", () => {
    expect(stringify([1, 2])).toBe("[1,2]");
  });

  it("stringifies string", () => {
    expect(stringify("hello")).toBe('"hello"');
  });

  it("stringifies integer", () => {
    expect(stringify(42)).toBe("42");
  });

  it("stringifies float", () => {
    expect(stringify(3.14)).toBe("3.14");
  });

  it("stringifies boolean", () => {
    expect(stringify(true)).toBe("true");
    expect(stringify(false)).toBe("false");
  });

  it("stringifies null", () => {
    expect(stringify(null)).toBe("null");
  });

  it("stringifies nested object", () => {
    expect(stringify({ a: { b: 1 } })).toBe('{"a":{"b":1}}');
  });

  it("stringifies nested array", () => {
    expect(stringify([[1], [2]])).toBe("[[1],[2]]");
  });

  it("stringifies mixed nested structure", () => {
    const result = stringify({
      name: "Alice",
      scores: [95, 87],
      active: true,
      address: null,
    });
    expect(result).toBe(
      '{"name":"Alice","scores":[95,87],"active":true,"address":null}'
    );
  });
});

describe("stringifyPretty", () => {
  it("pretty-stringifies object", () => {
    expect(stringifyPretty({ a: 1 })).toBe('{\n  "a": 1\n}');
  });

  it("pretty-stringifies with custom config", () => {
    expect(
      stringifyPretty({ b: 2, a: 1 }, { sortKeys: true, indentSize: 4 })
    ).toBe('{\n    "a": 1,\n    "b": 2\n}');
  });

  it("pretty-stringifies array", () => {
    expect(stringifyPretty([1, 2])).toBe("[\n  1,\n  2\n]");
  });

  it("pretty-stringifies with trailing newline", () => {
    expect(stringifyPretty({ a: 1 }, { trailingNewline: true })).toBe(
      '{\n  "a": 1\n}\n'
    );
  });
});

// =============================================================================
// FULL ROUND-TRIP TESTS (parse + serialize)
// =============================================================================

describe("Round-trip: parse then serialize", () => {
  /**
   * For round-trip tests, we parse JSON text using our parser, then serialize
   * it back. The output should match the canonical compact form (no whitespace
   * in the original).
   */

  it("round-trips simple object", () => {
    const json = '{"a":1}';
    const value = parse(json);
    expect(serialize(value)).toBe(json);
  });

  it("round-trips complex object", () => {
    const json = '{"name":"Alice","age":30,"active":true}';
    const value = parse(json);
    expect(serialize(value)).toBe(json);
  });

  it("round-trips array", () => {
    const json = "[1,2,3]";
    const value = parse(json);
    expect(serialize(value)).toBe(json);
  });

  it("round-trips nested structure", () => {
    const json = '{"users":[{"name":"Alice"}],"count":1}';
    const value = parse(json);
    expect(serialize(value)).toBe(json);
  });

  it("round-trips empty containers", () => {
    expect(serialize(parse("{}"))).toBe("{}");
    expect(serialize(parse("[]"))).toBe("[]");
  });

  it("round-trips primitives", () => {
    expect(serialize(parse("null"))).toBe("null");
    expect(serialize(parse("true"))).toBe("true");
    expect(serialize(parse("false"))).toBe("false");
    expect(serialize(parse("42"))).toBe("42");
    expect(serialize(parse('"hello"'))).toBe('"hello"');
  });

  it("round-trips number types", () => {
    expect(serialize(parse("0"))).toBe("0");
    expect(serialize(parse("-17"))).toBe("-17");
    expect(serialize(parse("3.14"))).toBe("3.14");
  });

  it("round-trips string with escapes", () => {
    /**
     * Parse a string with escape sequences, then serialize it back.
     * The serializer must re-escape the characters.
     */
    const json = '"hello\\nworld"';
    const value = parse(json);
    expect(serialize(value)).toBe('"hello\\nworld"');
  });

  it("round-trips via parseNative -> stringify", () => {
    const json = '{"a":1}';
    const native = parseNative(json);
    expect(stringify(native)).toBe(json);
  });

  it("complex round-trip via parseNative -> stringify", () => {
    const json =
      '{"users":[{"name":"Alice","age":30}],"count":1}';
    const native = parseNative(json);
    expect(stringify(native)).toBe(json);
  });
});

// =============================================================================
// ERROR CLASS
// =============================================================================

describe("JsonSerializerError", () => {
  it("has correct name and message", () => {
    const error = new JsonSerializerError("test error");
    expect(error.name).toBe("JsonSerializerError");
    expect(error.message).toBe("test error");
    expect(error instanceof Error).toBe(true);
  });
});
