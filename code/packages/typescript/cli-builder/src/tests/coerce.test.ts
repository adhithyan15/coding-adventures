/**
 * coerce.test.ts — Tests for the coerceValue function in positional-resolver.ts.
 *
 * coerceValue is a critical code path: every flag value and positional token
 * passes through it. The existing parser.test.ts covers integer and float
 * indirectly but several branches have no direct coverage:
 *
 * - "boolean" type (returns raw string — unusual path)
 * - "float" with a value that is a valid integer (e.g. "3")
 * - "float" with NaN input
 * - "integer" with a float string (e.g. "3.5") → error
 * - "path" with non-empty string → success
 * - "path" with empty string → error
 * - "file" with a real existing file (use __filename) → success
 * - "file" with a path that is a directory → error
 * - "file" with a non-existent path → error
 * - "directory" with a real existing directory → success
 * - "directory" with a path that is a file → error
 * - "directory" with a non-existent path → error
 * - "enum" with a valid member → success
 * - "enum" with an invalid member → error
 * - "string" with a non-empty value → success
 * - "string" with an empty string → error
 * - unknown type (default branch) → returns raw value
 */

import { describe, it, expect } from "vitest";
import { coerceValue } from "../positional-resolver.js";
import { existsSync } from "fs";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/** A real file we can use for "file" type tests — this test file itself. */
const REAL_FILE = new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");
/** The directory containing this test file — use it for "directory" type tests. */
const REAL_DIR = REAL_FILE.replace(/[\\/][^\\/]+$/, "");

// ---------------------------------------------------------------------------
// boolean
// ---------------------------------------------------------------------------

describe("coerceValue — boolean type", () => {
  it("returns the raw string for boolean type (unusual path)", () => {
    const result = coerceValue("true", "boolean", "flag", ["tool"]);
    expect(result).toEqual({ value: "true" });
  });

  it("returns any raw string for boolean type", () => {
    const result = coerceValue("anything", "boolean", "flag", ["tool"]);
    expect(result).toEqual({ value: "anything" });
  });
});

// ---------------------------------------------------------------------------
// string
// ---------------------------------------------------------------------------

describe("coerceValue — string type", () => {
  it("returns the raw string for a non-empty string", () => {
    const result = coerceValue("hello", "string", "arg", ["tool"]);
    expect(result).toEqual({ value: "hello" });
  });

  it("returns error for an empty string", () => {
    const result = coerceValue("", "string", "arg", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("arg");
    }
  });

  it("returns the string value unchanged (no coercion)", () => {
    const result = coerceValue("hello world", "string", "message", ["tool"]);
    expect(result).toEqual({ value: "hello world" });
  });
});

// ---------------------------------------------------------------------------
// integer
// ---------------------------------------------------------------------------

describe("coerceValue — integer type", () => {
  it("coerces a valid integer string to a number", () => {
    const result = coerceValue("42", "integer", "count", ["tool"]);
    expect(result).toEqual({ value: 42 });
  });

  it("coerces 0 correctly", () => {
    const result = coerceValue("0", "integer", "count", ["tool"]);
    expect(result).toEqual({ value: 0 });
  });

  it("coerces negative integer correctly", () => {
    const result = coerceValue("-5", "integer", "offset", ["tool"]);
    expect(result).toEqual({ value: -5 });
  });

  it("returns error for a float string like '3.5'", () => {
    const result = coerceValue("3.5", "integer", "count", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("count");
    }
  });

  it("returns error for 'abc'", () => {
    const result = coerceValue("abc", "integer", "count", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
    }
  });

  it("returns error for empty string", () => {
    const result = coerceValue("", "integer", "count", ["tool"]);
    expect("error" in result).toBe(true);
  });

  it("includes context in the error", () => {
    const result = coerceValue("bad", "integer", "count", ["tool", "sub"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.context).toEqual(["tool", "sub"]);
    }
  });
});

// ---------------------------------------------------------------------------
// float
// ---------------------------------------------------------------------------

describe("coerceValue — float type", () => {
  it("coerces a decimal float string correctly", () => {
    const result = coerceValue("3.14", "float", "ratio", ["tool"]);
    expect("value" in result).toBe(true);
    if ("value" in result) {
      expect(result.value).toBeCloseTo(3.14);
    }
  });

  it("coerces an integer string to a float", () => {
    const result = coerceValue("3", "float", "ratio", ["tool"]);
    expect("value" in result).toBe(true);
    if ("value" in result) {
      expect(result.value).toBe(3);
    }
  });

  it("coerces negative float correctly", () => {
    const result = coerceValue("-1.5", "float", "delta", ["tool"]);
    expect("value" in result).toBe(true);
    if ("value" in result) {
      expect(result.value).toBeCloseTo(-1.5);
    }
  });

  it("coerces '0' to 0", () => {
    const result = coerceValue("0", "float", "x", ["tool"]);
    expect(result).toEqual({ value: 0 });
  });

  it("returns error for 'abc' (NaN)", () => {
    const result = coerceValue("abc", "float", "ratio", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("ratio");
    }
  });

  it("returns error for empty string (NaN)", () => {
    const result = coerceValue("", "float", "ratio", ["tool"]);
    expect("error" in result).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// path
// ---------------------------------------------------------------------------

describe("coerceValue — path type", () => {
  it("accepts any non-empty string as a valid path", () => {
    const result = coerceValue("/some/path/that/may/not/exist", "path", "dest", ["tool"]);
    expect(result).toEqual({ value: "/some/path/that/may/not/exist" });
  });

  it("accepts the stdin sentinel '-' as a valid path", () => {
    const result = coerceValue("-", "path", "input", ["tool"]);
    expect(result).toEqual({ value: "-" });
  });

  it("accepts a relative path", () => {
    const result = coerceValue("./relative/path", "path", "out", ["tool"]);
    expect(result).toEqual({ value: "./relative/path" });
  });

  it("returns error for an empty string path", () => {
    const result = coerceValue("", "path", "dest", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("dest");
    }
  });
});

// ---------------------------------------------------------------------------
// file
// ---------------------------------------------------------------------------

describe("coerceValue — file type", () => {
  it("accepts a path to an existing file", () => {
    // Use this very test file as a real existing file.
    const result = coerceValue(REAL_FILE, "file", "input", ["tool"]);
    // Should succeed if the file exists
    if (existsSync(REAL_FILE)) {
      expect("value" in result).toBe(true);
      if ("value" in result) {
        expect(result.value).toBe(REAL_FILE);
      }
    }
  });

  it("returns error for a path that is a directory, not a file", () => {
    const result = coerceValue(REAL_DIR, "file", "input", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("not a file");
    }
  });

  it("returns error for a non-existent path", () => {
    const result = coerceValue("/this/path/does/not/exist/at/all.txt", "file", "input", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("not found");
    }
  });

  it("includes the arg id in the file error message", () => {
    const result = coerceValue("/nonexistent/file.txt", "file", "myarg", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.message).toContain("myarg");
    }
  });

  it("includes context in file errors", () => {
    const result = coerceValue("/nonexistent/file.txt", "file", "myarg", ["tool", "cmd"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.context).toEqual(["tool", "cmd"]);
    }
  });
});

// ---------------------------------------------------------------------------
// directory
// ---------------------------------------------------------------------------

describe("coerceValue — directory type", () => {
  it("accepts a path to an existing directory", () => {
    const result = coerceValue(REAL_DIR, "directory", "outdir", ["tool"]);
    if (existsSync(REAL_DIR)) {
      expect("value" in result).toBe(true);
      if ("value" in result) {
        expect(result.value).toBe(REAL_DIR);
      }
    }
  });

  it("returns error for a path that is a file, not a directory", () => {
    const result = coerceValue(REAL_FILE, "directory", "outdir", ["tool"]);
    if (existsSync(REAL_FILE)) {
      expect("error" in result).toBe(true);
      if ("error" in result) {
        expect(result.error.errorType).toBe("invalid_value");
        expect(result.error.message).toContain("not a directory");
      }
    }
  });

  it("returns error for a non-existent directory path", () => {
    const result = coerceValue("/this/path/does/not/exist/at/all", "directory", "outdir", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_value");
      expect(result.error.message).toContain("not found");
    }
  });

  it("includes the arg id in the directory error message", () => {
    const result = coerceValue("/nonexistent/dir", "directory", "mydir", ["tool"]);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.message).toContain("mydir");
    }
  });
});

// ---------------------------------------------------------------------------
// enum
// ---------------------------------------------------------------------------

describe("coerceValue — enum type", () => {
  const enumValues = ["json", "csv", "table"];

  it("accepts a valid enum value", () => {
    const result = coerceValue("json", "enum", "format", ["tool"], enumValues);
    expect(result).toEqual({ value: "json" });
  });

  it("accepts another valid enum value", () => {
    const result = coerceValue("csv", "enum", "format", ["tool"], enumValues);
    expect(result).toEqual({ value: "csv" });
  });

  it("returns error for an invalid enum value", () => {
    const result = coerceValue("xml", "enum", "format", ["tool"], enumValues);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_enum_value");
      expect(result.error.message).toContain("xml");
      expect(result.error.message).toContain("json");
    }
  });

  it("returns error for empty string not in enum", () => {
    const result = coerceValue("", "enum", "format", ["tool"], enumValues);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.errorType).toBe("invalid_enum_value");
    }
  });

  it("is case-sensitive — uppercase is rejected", () => {
    const result = coerceValue("JSON", "enum", "format", ["tool"], enumValues);
    expect("error" in result).toBe(true);
  });

  it("includes the arg id and valid values in the error message", () => {
    const result = coerceValue("bad", "enum", "fmt", ["tool"], enumValues);
    expect("error" in result).toBe(true);
    if ("error" in result) {
      expect(result.error.message).toContain("fmt");
      expect(result.error.message).toContain("json, csv, table");
    }
  });

  it("handles empty enum_values array gracefully (rejects all)", () => {
    const result = coerceValue("anything", "enum", "val", ["tool"], []);
    expect("error" in result).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// unknown type (default branch)
// ---------------------------------------------------------------------------

describe("coerceValue — unknown/default type", () => {
  it("returns the raw value for an unknown type", () => {
    const result = coerceValue("rawval", "unknown-type-xyz", "arg", ["tool"]);
    expect(result).toEqual({ value: "rawval" });
  });
});
