/**
 * validate.test.ts -- Tests for the standalone validateSpec / validateSpecObject functions.
 *
 * These tests verify that the validation adapter layer correctly translates
 * SpecLoader's exception-based API into a result-based API. The tests cover:
 *
 * 1. A valid spec returns { valid: true, errors: [] }
 * 2. Missing cli_builder_spec_version produces a validation error
 * 3. Unsupported spec version (e.g., "2.0") produces a validation error
 * 4. Missing required fields (name, description) produce validation errors
 * 5. Invalid JSON file produces a validation error
 * 6. Nonexistent file produces a validation error
 * 7. A flag with no short/long/single_dash_long produces a validation error
 *
 * Each test uses validateSpecObject() for in-memory specs or validateSpec()
 * for file-based scenarios.
 */

import { describe, it, expect } from "vitest";
import { writeFileSync, unlinkSync, mkdtempSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { validateSpec, validateSpecObject } from "../validate.js";

// ---------------------------------------------------------------------------
// Helper: minimal valid spec object
// ---------------------------------------------------------------------------

/**
 * A minimal spec that passes all validation rules. Tests that need an
 * invalid spec will spread this and override or delete specific fields.
 */
const VALID_SPEC: Record<string, unknown> = {
  cli_builder_spec_version: "1.0",
  name: "test-tool",
  description: "A test tool for validation",
};

// ---------------------------------------------------------------------------
// Tests for validateSpecObject()
// ---------------------------------------------------------------------------

describe("validateSpecObject", () => {
  // -----------------------------------------------------------------------
  // 1. Valid spec
  // -----------------------------------------------------------------------

  it("returns valid for a minimal correct spec", () => {
    const result = validateSpecObject({ ...VALID_SPEC });

    // A minimal spec with just version, name, and description should pass.
    // No flags, arguments, or commands are required at the top level.
    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it("returns valid for a spec with flags and arguments", () => {
    const spec = {
      ...VALID_SPEC,
      flags: [
        {
          id: "verbose",
          short: "v",
          description: "Enable verbose output",
          type: "boolean",
        },
      ],
      arguments: [
        {
          id: "file",
          name: "FILE",
          description: "Input file",
          type: "string",
          required: true,
        },
      ],
    };

    const result = validateSpecObject(spec);
    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  // -----------------------------------------------------------------------
  // 2. Missing cli_builder_spec_version
  // -----------------------------------------------------------------------

  it("rejects a spec with no cli_builder_spec_version", () => {
    // Omitting the version field entirely should fail validation.
    // SpecLoader checks this as Rule 1: version must be "1.0".
    const { cli_builder_spec_version, ...noVersion } = VALID_SPEC;

    const result = validateSpecObject(noVersion);

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBe(1);
    expect(result.errors[0]).toContain("cli_builder_spec_version");
  });

  // -----------------------------------------------------------------------
  // 3. Unsupported spec version
  // -----------------------------------------------------------------------

  it("rejects a spec with unsupported version", () => {
    // Only version "1.0" is supported. Anything else should fail.
    const spec = { ...VALID_SPEC, cli_builder_spec_version: "2.0" };

    const result = validateSpecObject(spec);

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBe(1);
    expect(result.errors[0]).toContain("cli_builder_spec_version");
    expect(result.errors[0]).toContain("2.0");
  });

  it("rejects a spec with numeric version instead of string", () => {
    // The version must be the string "1.0", not the number 1.0.
    const spec = { ...VALID_SPEC, cli_builder_spec_version: 1.0 };

    const result = validateSpecObject(spec);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("cli_builder_spec_version");
  });

  // -----------------------------------------------------------------------
  // 4. Missing required fields
  // -----------------------------------------------------------------------

  it("rejects a spec missing the name field", () => {
    const { name, ...noName } = VALID_SPEC;

    const result = validateSpecObject(noName);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("name");
  });

  it("rejects a spec missing the description field", () => {
    const { description, ...noDesc } = VALID_SPEC;

    const result = validateSpecObject(noDesc);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("description");
  });

  it("rejects a spec with an empty name", () => {
    // An empty string should not satisfy the "required string" check.
    const spec = { ...VALID_SPEC, name: "" };

    const result = validateSpecObject(spec);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("name");
  });

  // -----------------------------------------------------------------------
  // 7. Flag with no short/long/single_dash_long
  // -----------------------------------------------------------------------

  it("rejects a flag that has no short, long, or single_dash_long", () => {
    // Rule 3 in spec-loader: every flag must have at least one of
    // short, long, or single_dash_long. A flag with none of these
    // has no way to be invoked on the command line.
    const spec = {
      ...VALID_SPEC,
      flags: [
        {
          id: "phantom",
          description: "A flag with no way to specify it",
          type: "boolean",
        },
      ],
    };

    const result = validateSpecObject(spec);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("phantom");
    expect(result.errors[0]).toMatch(/short|long|single_dash_long/);
  });

  // -----------------------------------------------------------------------
  // Additional edge cases
  // -----------------------------------------------------------------------

  it("rejects a spec with duplicate flag IDs", () => {
    const spec = {
      ...VALID_SPEC,
      flags: [
        { id: "verbose", short: "v", description: "Verbose", type: "boolean" },
        { id: "verbose", long: "verbose", description: "Verbose again", type: "boolean" },
      ],
    };

    const result = validateSpecObject(spec);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("verbose");
    expect(result.errors[0]).toContain("Duplicate");
  });

  it("rejects a flag with type enum but no enum_values", () => {
    const spec = {
      ...VALID_SPEC,
      flags: [
        {
          id: "format",
          long: "format",
          description: "Output format",
          type: "enum",
        },
      ],
    };

    const result = validateSpecObject(spec);

    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("enum");
  });
});

// ---------------------------------------------------------------------------
// Tests for validateSpec() (file-based)
// ---------------------------------------------------------------------------

describe("validateSpec", () => {
  /**
   * Helper to create a temporary file with the given content.
   * Returns the file path. Caller should clean up with unlinkSync().
   */
  function writeTempFile(content: string, extension = ".json"): string {
    const dir = mkdtempSync(join(tmpdir(), "cli-builder-validate-"));
    const filePath = join(dir, `spec${extension}`);
    writeFileSync(filePath, content, "utf-8");
    return filePath;
  }

  // -----------------------------------------------------------------------
  // 5. Invalid JSON file
  // -----------------------------------------------------------------------

  it("rejects a file containing invalid JSON", () => {
    // A file with broken JSON should produce a validation error, not an
    // uncaught exception. The error message should mention the parse failure.
    const filePath = writeTempFile("{ this is not valid json }");

    try {
      const result = validateSpec(filePath);

      expect(result.valid).toBe(false);
      expect(result.errors.length).toBe(1);
      // The error should mention the file path or the parse failure
      expect(result.errors[0]).toBeTruthy();
    } finally {
      unlinkSync(filePath);
    }
  });

  // -----------------------------------------------------------------------
  // 6. Nonexistent file
  // -----------------------------------------------------------------------

  it("rejects a nonexistent file path", () => {
    const result = validateSpec("/tmp/this-file-does-not-exist-12345.json");

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBe(1);
    expect(result.errors[0]).toBeTruthy();
  });

  // -----------------------------------------------------------------------
  // Valid file
  // -----------------------------------------------------------------------

  it("returns valid for a correct spec file", () => {
    const filePath = writeTempFile(JSON.stringify(VALID_SPEC));

    try {
      const result = validateSpec(filePath);

      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    } finally {
      unlinkSync(filePath);
    }
  });

  it("rejects a valid JSON file with a bad spec", () => {
    // Valid JSON, but missing required spec fields.
    const filePath = writeTempFile(JSON.stringify({ foo: "bar" }));

    try {
      const result = validateSpec(filePath);

      expect(result.valid).toBe(false);
      expect(result.errors.length).toBe(1);
    } finally {
      unlinkSync(filePath);
    }
  });
});
