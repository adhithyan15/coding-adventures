/**
 * JSON Value -- Comprehensive Test Suite
 *
 * This test suite covers the full json-value API:
 *
 *   1. Factory functions (jsonObject, jsonArray, jsonString, etc.)
 *   2. fromAST() -- AST to JsonValue conversion
 *   3. toNative() -- JsonValue to native JS types
 *   4. fromNative() -- native JS types to JsonValue
 *   5. parse() and parseNative() -- end-to-end text to JsonValue/native
 *   6. Round-trip tests -- ensure conversions are reversible
 *
 * Test Organization
 * -----------------
 *
 * Tests are grouped by function/module, then by scenario. Each test name
 * describes the input and expected output to serve as documentation.
 *
 * Coverage Target: 95%+
 */

import { describe, it, expect } from "vitest";

import {
  type JsonValue,
  jsonObject,
  jsonArray,
  jsonString,
  jsonNumber,
  jsonBool,
  jsonNull,
  fromAST,
  toNative,
  fromNative,
  parse,
  parseNative,
  JsonValueError,
} from "../src/index.js";

// =============================================================================
// FACTORY FUNCTIONS
// =============================================================================

describe("Factory Functions", () => {
  /**
   * Each factory function creates a JsonValue with the correct `type`
   * discriminant field. These tests verify the shape of each variant.
   */

  it("jsonNull creates a null value", () => {
    const value = jsonNull();
    expect(value.type).toBe("null");
  });

  it("jsonBool(true) creates a boolean with value true", () => {
    const value = jsonBool(true);
    expect(value.type).toBe("boolean");
    expect(value.value).toBe(true);
  });

  it("jsonBool(false) creates a boolean with value false", () => {
    const value = jsonBool(false);
    expect(value.type).toBe("boolean");
    expect(value.value).toBe(false);
  });

  it("jsonString creates a string value", () => {
    const value = jsonString("hello");
    expect(value.type).toBe("string");
    expect(value.value).toBe("hello");
  });

  it("jsonString with empty string", () => {
    const value = jsonString("");
    expect(value.type).toBe("string");
    expect(value.value).toBe("");
  });

  it("jsonNumber with integer auto-detects isInteger", () => {
    const value = jsonNumber(42);
    expect(value.type).toBe("number");
    expect(value.value).toBe(42);
    expect(value.isInteger).toBe(true);
  });

  it("jsonNumber with float auto-detects isInteger as false", () => {
    const value = jsonNumber(3.14);
    expect(value.type).toBe("number");
    expect(value.value).toBe(3.14);
    expect(value.isInteger).toBe(false);
  });

  it("jsonNumber with explicit isInteger override", () => {
    /**
     * Scientific notation like 1e10 evaluates to 10000000000, which
     * Number.isInteger() says is true. But the JSON source used exponent
     * notation, so we want isInteger = false. The factory accepts an
     * explicit override for this case.
     */
    const value = jsonNumber(1e10, false);
    expect(value.isInteger).toBe(false);
  });

  it("jsonNumber with zero", () => {
    const value = jsonNumber(0);
    expect(value.value).toBe(0);
    expect(value.isInteger).toBe(true);
  });

  it("jsonNumber with negative integer", () => {
    const value = jsonNumber(-17);
    expect(value.value).toBe(-17);
    expect(value.isInteger).toBe(true);
  });

  it("jsonArray with no arguments creates empty array", () => {
    const value = jsonArray();
    expect(value.type).toBe("array");
    expect(value.elements).toEqual([]);
  });

  it("jsonArray with elements", () => {
    const value = jsonArray([jsonNumber(1), jsonNumber(2)]);
    expect(value.elements.length).toBe(2);
  });

  it("jsonObject with no arguments creates empty object", () => {
    const value = jsonObject();
    expect(value.type).toBe("object");
    expect(value.pairs.size).toBe(0);
  });

  it("jsonObject with Map", () => {
    const pairs = new Map<string, JsonValue>([
      ["a", jsonNumber(1)],
    ]);
    const value = jsonObject(pairs);
    expect(value.pairs.size).toBe(1);
    expect(value.pairs.get("a")).toEqual(jsonNumber(1));
  });

  it("jsonObject with array of tuples", () => {
    const value = jsonObject([
      ["x", jsonString("hello")],
      ["y", jsonBool(true)],
    ]);
    expect(value.pairs.size).toBe(2);
    expect(value.pairs.get("x")).toEqual(jsonString("hello"));
    expect(value.pairs.get("y")).toEqual(jsonBool(true));
  });
});

// =============================================================================
// fromAST -- AST to JsonValue conversion
// =============================================================================
//
// These tests use parse() internally (which calls parseJSON then fromAST).
// We test fromAST indirectly through parse() because constructing AST nodes
// manually would be fragile and couple tests to the parser's internal format.

describe("fromAST (via parse)", () => {
  it("parses empty object", () => {
    const value = parse("{}");
    expect(value.type).toBe("object");
    if (value.type === "object") {
      expect(value.pairs.size).toBe(0);
    }
  });

  it("parses empty array", () => {
    const value = parse("[]");
    expect(value.type).toBe("array");
    if (value.type === "array") {
      expect(value.elements.length).toBe(0);
    }
  });

  it("parses string value", () => {
    const value = parse('"hello"');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("hello");
    }
  });

  it("parses empty string", () => {
    const value = parse('""');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("");
    }
  });

  it("parses integer number", () => {
    const value = parse("42");
    expect(value.type).toBe("number");
    if (value.type === "number") {
      expect(value.value).toBe(42);
      expect(value.isInteger).toBe(true);
    }
  });

  it("parses zero", () => {
    const value = parse("0");
    expect(value.type).toBe("number");
    if (value.type === "number") {
      expect(value.value).toBe(0);
      expect(value.isInteger).toBe(true);
    }
  });

  it("parses negative integer", () => {
    const value = parse("-17");
    expect(value.type).toBe("number");
    if (value.type === "number") {
      expect(value.value).toBe(-17);
      expect(value.isInteger).toBe(true);
    }
  });

  it("parses float number", () => {
    const value = parse("3.14");
    expect(value.type).toBe("number");
    if (value.type === "number") {
      expect(value.value).toBe(3.14);
      expect(value.isInteger).toBe(false);
    }
  });

  it("parses exponent number as float", () => {
    const value = parse("1e10");
    expect(value.type).toBe("number");
    if (value.type === "number") {
      expect(value.value).toBe(1e10);
      expect(value.isInteger).toBe(false);
    }
  });

  it("parses negative exponent", () => {
    const value = parse("2.5e-3");
    expect(value.type).toBe("number");
    if (value.type === "number") {
      expect(value.value).toBeCloseTo(0.0025);
      expect(value.isInteger).toBe(false);
    }
  });

  it("parses true", () => {
    const value = parse("true");
    expect(value.type).toBe("boolean");
    if (value.type === "boolean") {
      expect(value.value).toBe(true);
    }
  });

  it("parses false", () => {
    const value = parse("false");
    expect(value.type).toBe("boolean");
    if (value.type === "boolean") {
      expect(value.value).toBe(false);
    }
  });

  it("parses null", () => {
    const value = parse("null");
    expect(value.type).toBe("null");
  });

  it("parses simple object with one pair", () => {
    const value = parse('{"a": 1}');
    expect(value.type).toBe("object");
    if (value.type === "object") {
      expect(value.pairs.size).toBe(1);
      const a = value.pairs.get("a");
      expect(a?.type).toBe("number");
      if (a?.type === "number") {
        expect(a.value).toBe(1);
      }
    }
  });

  it("parses multi-key object", () => {
    const value = parse('{"a": 1, "b": 2}');
    expect(value.type).toBe("object");
    if (value.type === "object") {
      expect(value.pairs.size).toBe(2);
      const a = value.pairs.get("a");
      const b = value.pairs.get("b");
      expect(a?.type).toBe("number");
      expect(b?.type).toBe("number");
    }
  });

  it("parses simple array", () => {
    const value = parse("[1, 2, 3]");
    expect(value.type).toBe("array");
    if (value.type === "array") {
      expect(value.elements.length).toBe(3);
      expect(value.elements[0].type).toBe("number");
      expect(value.elements[1].type).toBe("number");
      expect(value.elements[2].type).toBe("number");
    }
  });

  it("parses mixed-type array", () => {
    const value = parse('[1, "two", true, null]');
    expect(value.type).toBe("array");
    if (value.type === "array") {
      expect(value.elements.length).toBe(4);
      expect(value.elements[0].type).toBe("number");
      expect(value.elements[1].type).toBe("string");
      expect(value.elements[2].type).toBe("boolean");
      expect(value.elements[3].type).toBe("null");
    }
  });

  it("parses nested object", () => {
    const value = parse('{"a": {"b": 1}}');
    expect(value.type).toBe("object");
    if (value.type === "object") {
      const a = value.pairs.get("a");
      expect(a?.type).toBe("object");
      if (a?.type === "object") {
        const b = a.pairs.get("b");
        expect(b?.type).toBe("number");
      }
    }
  });

  it("parses nested array", () => {
    const value = parse("[[1, 2], [3, 4]]");
    expect(value.type).toBe("array");
    if (value.type === "array") {
      expect(value.elements.length).toBe(2);
      expect(value.elements[0].type).toBe("array");
      expect(value.elements[1].type).toBe("array");
    }
  });

  it("parses complex nested structure", () => {
    const value = parse('{"users": [{"name": "Alice"}]}');
    expect(value.type).toBe("object");
    if (value.type === "object") {
      const users = value.pairs.get("users");
      expect(users?.type).toBe("array");
      if (users?.type === "array") {
        expect(users.elements.length).toBe(1);
        const first = users.elements[0];
        expect(first.type).toBe("object");
        if (first.type === "object") {
          const name = first.pairs.get("name");
          expect(name?.type).toBe("string");
          if (name?.type === "string") {
            expect(name.value).toBe("Alice");
          }
        }
      }
    }
  });

  it("parses string with escape sequences", () => {
    const value = parse('"hello\\nworld"');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("hello\nworld");
    }
  });

  it("parses string with tab escape", () => {
    const value = parse('"a\\tb"');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("a\tb");
    }
  });

  it("parses string with escaped quotes", () => {
    const value = parse('"say \\"hi\\""');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe('say "hi"');
    }
  });

  it("parses string with backslash escape", () => {
    const value = parse('"a\\\\b"');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("a\\b");
    }
  });

  it("parses string with unicode escape", () => {
    const value = parse('"\\u0041"');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("A");
    }
  });

  it("parses string with forward slash escape", () => {
    const value = parse('"a\\/b"');
    expect(value.type).toBe("string");
    if (value.type === "string") {
      expect(value.value).toBe("a/b");
    }
  });
});

// =============================================================================
// toNative -- JsonValue to native JS types
// =============================================================================

describe("toNative", () => {
  it("converts JsonNull to null", () => {
    expect(toNative(jsonNull())).toBe(null);
  });

  it("converts JsonBoolean true to true", () => {
    expect(toNative(jsonBool(true))).toBe(true);
  });

  it("converts JsonBoolean false to false", () => {
    expect(toNative(jsonBool(false))).toBe(false);
  });

  it("converts JsonString to string", () => {
    expect(toNative(jsonString("hello"))).toBe("hello");
  });

  it("converts JsonNumber integer to number", () => {
    expect(toNative(jsonNumber(42))).toBe(42);
  });

  it("converts JsonNumber float to number", () => {
    expect(toNative(jsonNumber(3.14))).toBe(3.14);
  });

  it("converts JsonArray to array", () => {
    const result = toNative(jsonArray([jsonNumber(1), jsonNumber(2)]));
    expect(result).toEqual([1, 2]);
  });

  it("converts JsonObject to plain object", () => {
    const result = toNative(
      jsonObject(
        new Map<string, JsonValue>([
          ["a", jsonNumber(1)],
          ["b", jsonString("two")],
        ])
      )
    );
    expect(result).toEqual({ a: 1, b: "two" });
  });

  it("converts deeply nested structure", () => {
    const value = jsonObject(
      new Map<string, JsonValue>([
        [
          "users",
          jsonArray([
            jsonObject(
              new Map<string, JsonValue>([
                ["name", jsonString("Alice")],
                ["age", jsonNumber(30)],
                ["active", jsonBool(true)],
                ["address", jsonNull()],
              ])
            ),
          ]),
        ],
      ])
    );

    const result = toNative(value);
    expect(result).toEqual({
      users: [{ name: "Alice", age: 30, active: true, address: null }],
    });
  });

  it("converts empty object to empty plain object", () => {
    expect(toNative(jsonObject())).toEqual({});
  });

  it("converts empty array to empty array", () => {
    expect(toNative(jsonArray())).toEqual([]);
  });
});

// =============================================================================
// fromNative -- native JS types to JsonValue
// =============================================================================

describe("fromNative", () => {
  it("converts null to JsonNull", () => {
    const result = fromNative(null);
    expect(result.type).toBe("null");
  });

  it("converts true to JsonBoolean", () => {
    const result = fromNative(true);
    expect(result.type).toBe("boolean");
    if (result.type === "boolean") expect(result.value).toBe(true);
  });

  it("converts false to JsonBoolean", () => {
    const result = fromNative(false);
    expect(result.type).toBe("boolean");
    if (result.type === "boolean") expect(result.value).toBe(false);
  });

  it("converts string to JsonString", () => {
    const result = fromNative("hello");
    expect(result.type).toBe("string");
    if (result.type === "string") expect(result.value).toBe("hello");
  });

  it("converts integer to JsonNumber", () => {
    const result = fromNative(42);
    expect(result.type).toBe("number");
    if (result.type === "number") {
      expect(result.value).toBe(42);
      expect(result.isInteger).toBe(true);
    }
  });

  it("converts float to JsonNumber", () => {
    const result = fromNative(3.14);
    expect(result.type).toBe("number");
    if (result.type === "number") {
      expect(result.value).toBe(3.14);
      expect(result.isInteger).toBe(false);
    }
  });

  it("converts array to JsonArray", () => {
    const result = fromNative([1, 2, 3]);
    expect(result.type).toBe("array");
    if (result.type === "array") {
      expect(result.elements.length).toBe(3);
      expect(result.elements[0].type).toBe("number");
    }
  });

  it("converts object to JsonObject", () => {
    const result = fromNative({ a: 1, b: "two" });
    expect(result.type).toBe("object");
    if (result.type === "object") {
      expect(result.pairs.size).toBe(2);
      expect(result.pairs.get("a")?.type).toBe("number");
      expect(result.pairs.get("b")?.type).toBe("string");
    }
  });

  it("converts nested structure", () => {
    const input = {
      users: [{ name: "Alice", scores: [95, 87] }],
      active: true,
    };
    const result = fromNative(input);
    expect(result.type).toBe("object");
    if (result.type === "object") {
      const users = result.pairs.get("users");
      expect(users?.type).toBe("array");
    }
  });

  it("throws on undefined", () => {
    expect(() => fromNative(undefined)).toThrow(JsonValueError);
  });

  it("throws on function", () => {
    expect(() => fromNative(() => {})).toThrow(JsonValueError);
  });

  it("throws on symbol", () => {
    expect(() => fromNative(Symbol())).toThrow(JsonValueError);
  });

  it("throws on BigInt", () => {
    expect(() => fromNative(BigInt(42))).toThrow(JsonValueError);
  });

  it("throws on Infinity", () => {
    expect(() => fromNative(Infinity)).toThrow(JsonValueError);
  });

  it("throws on NaN", () => {
    expect(() => fromNative(NaN)).toThrow(JsonValueError);
  });

  it("throws on -Infinity", () => {
    expect(() => fromNative(-Infinity)).toThrow(JsonValueError);
  });

  it("converts empty object", () => {
    const result = fromNative({});
    expect(result.type).toBe("object");
    if (result.type === "object") {
      expect(result.pairs.size).toBe(0);
    }
  });

  it("converts empty array", () => {
    const result = fromNative([]);
    expect(result.type).toBe("array");
    if (result.type === "array") {
      expect(result.elements.length).toBe(0);
    }
  });

  it("converts zero", () => {
    const result = fromNative(0);
    expect(result.type).toBe("number");
    if (result.type === "number") {
      expect(result.value).toBe(0);
      expect(result.isInteger).toBe(true);
    }
  });

  it("converts negative number", () => {
    const result = fromNative(-5);
    expect(result.type).toBe("number");
    if (result.type === "number") {
      expect(result.value).toBe(-5);
      expect(result.isInteger).toBe(true);
    }
  });

  it("converts empty string", () => {
    const result = fromNative("");
    expect(result.type).toBe("string");
    if (result.type === "string") expect(result.value).toBe("");
  });
});

// =============================================================================
// parse and parseNative -- end-to-end text parsing
// =============================================================================

describe("parse", () => {
  it("returns JsonValue for valid JSON object", () => {
    const result = parse('{"a": 1}');
    expect(result.type).toBe("object");
  });

  it("returns JsonValue for valid JSON array", () => {
    const result = parse("[1, 2, 3]");
    expect(result.type).toBe("array");
  });

  it("returns JsonValue for valid JSON string", () => {
    const result = parse('"hello"');
    expect(result.type).toBe("string");
  });

  it("returns JsonValue for valid JSON number", () => {
    const result = parse("42");
    expect(result.type).toBe("number");
  });

  it("returns JsonValue for true", () => {
    const result = parse("true");
    expect(result.type).toBe("boolean");
  });

  it("returns JsonValue for null", () => {
    const result = parse("null");
    expect(result.type).toBe("null");
  });

  it("throws JsonValueError for invalid JSON", () => {
    expect(() => parse("not json")).toThrow(JsonValueError);
  });

  it("throws JsonValueError for incomplete JSON", () => {
    expect(() => parse("{")).toThrow(JsonValueError);
  });
});

describe("parseNative", () => {
  it("returns native object for valid JSON", () => {
    const result = parseNative('{"a": 1}');
    expect(result).toEqual({ a: 1 });
  });

  it("returns native array for valid JSON", () => {
    const result = parseNative("[1, 2, 3]");
    expect(result).toEqual([1, 2, 3]);
  });

  it("returns string for valid JSON string", () => {
    expect(parseNative('"hello"')).toBe("hello");
  });

  it("returns number for valid JSON number", () => {
    expect(parseNative("42")).toBe(42);
  });

  it("returns boolean for valid JSON boolean", () => {
    expect(parseNative("true")).toBe(true);
    expect(parseNative("false")).toBe(false);
  });

  it("returns null for valid JSON null", () => {
    expect(parseNative("null")).toBe(null);
  });

  it("throws JsonValueError for invalid JSON", () => {
    expect(() => parseNative("not json")).toThrow(JsonValueError);
  });

  it("parses complex nested JSON to native", () => {
    const json = '{"users": [{"name": "Alice", "age": 30}], "count": 1}';
    const result = parseNative(json);
    expect(result).toEqual({
      users: [{ name: "Alice", age: 30 }],
      count: 1,
    });
  });
});

// =============================================================================
// ROUND-TRIP TESTS
// =============================================================================
//
// These tests verify that conversion is reversible:
//   native --> fromNative --> toNative --> matches original
//   text   --> parse      --> toNative --> matches JSON.parse

describe("Round-trip: fromNative -> toNative", () => {
  it("round-trips simple object", () => {
    const original = { name: "Alice", age: 30 };
    const result = toNative(fromNative(original));
    expect(result).toEqual(original);
  });

  it("round-trips array of mixed types", () => {
    const original = [1, "two", true, null, { key: "val" }];
    const result = toNative(fromNative(original));
    expect(result).toEqual(original);
  });

  it("round-trips deeply nested structure", () => {
    const original = {
      level1: {
        level2: {
          level3: [1, 2, { deep: true }],
        },
      },
    };
    const result = toNative(fromNative(original));
    expect(result).toEqual(original);
  });

  it("round-trips empty containers", () => {
    expect(toNative(fromNative({}))).toEqual({});
    expect(toNative(fromNative([]))).toEqual([]);
  });

  it("round-trips primitives", () => {
    expect(toNative(fromNative("hello"))).toBe("hello");
    expect(toNative(fromNative(42))).toBe(42);
    expect(toNative(fromNative(3.14))).toBe(3.14);
    expect(toNative(fromNative(true))).toBe(true);
    expect(toNative(fromNative(false))).toBe(false);
    expect(toNative(fromNative(null))).toBe(null);
  });
});

describe("Round-trip: parse -> toNative (matches JSON.parse)", () => {
  it("round-trips object", () => {
    const json = '{"name": "Alice", "age": 30}';
    const ours = parseNative(json);
    const stdlib = JSON.parse(json);
    expect(ours).toEqual(stdlib);
  });

  it("round-trips array", () => {
    const json = "[1, 2, 3]";
    const ours = parseNative(json);
    const stdlib = JSON.parse(json);
    expect(ours).toEqual(stdlib);
  });

  it("round-trips complex nested JSON", () => {
    const json =
      '{"users": [{"name": "Alice", "scores": [95, 87]}, {"name": "Bob", "scores": []}], "total": 2}';
    const ours = parseNative(json);
    const stdlib = JSON.parse(json);
    expect(ours).toEqual(stdlib);
  });

  it("round-trips strings with escapes", () => {
    const json = '"hello\\nworld\\ttab"';
    const ours = parseNative(json);
    const stdlib = JSON.parse(json);
    expect(ours).toEqual(stdlib);
  });

  it("round-trips number types", () => {
    expect(parseNative("42")).toBe(42);
    expect(parseNative("3.14")).toBe(3.14);
    expect(parseNative("-17")).toBe(-17);
    expect(parseNative("0")).toBe(0);
  });

  it("round-trips empty containers", () => {
    expect(parseNative("{}")).toEqual({});
    expect(parseNative("[]")).toEqual([]);
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe("Edge cases", () => {
  it("handles object with all value types", () => {
    const json =
      '{"str": "hello", "num": 42, "flt": 3.14, "t": true, "f": false, "n": null, "arr": [1], "obj": {}}';
    const result = parse(json);
    expect(result.type).toBe("object");
    if (result.type === "object") {
      expect(result.pairs.size).toBe(8);
      expect(result.pairs.get("str")?.type).toBe("string");
      expect(result.pairs.get("num")?.type).toBe("number");
      expect(result.pairs.get("flt")?.type).toBe("number");
      expect(result.pairs.get("t")?.type).toBe("boolean");
      expect(result.pairs.get("f")?.type).toBe("boolean");
      expect(result.pairs.get("n")?.type).toBe("null");
      expect(result.pairs.get("arr")?.type).toBe("array");
      expect(result.pairs.get("obj")?.type).toBe("object");
    }
  });

  it("handles deeply nested arrays", () => {
    const json = "[[[1]]]";
    const result = parseNative(json);
    expect(result).toEqual([[[1]]]);
  });

  it("handles JSON with whitespace", () => {
    const json = '  {  "a"  :  1  }  ';
    const result = parseNative(json);
    expect(result).toEqual({ a: 1 });
  });

  it("JsonValueError has correct name", () => {
    const error = new JsonValueError("test");
    expect(error.name).toBe("JsonValueError");
    expect(error.message).toBe("test");
    expect(error instanceof Error).toBe(true);
  });
});
