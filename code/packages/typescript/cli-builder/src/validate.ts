/**
 * validate.ts -- Standalone spec validation for CLI Builder JSON specs.
 *
 * === Why a standalone validator? ===
 *
 * The SpecLoader class is designed around a "load or throw" model: you give
 * it a file path, call load(), and either get a fully parsed CliSpec object
 * or a SpecError exception blows up in your face. This is perfect for the
 * normal parse-and-run workflow, but sometimes you just want to check
 * whether a spec file is valid -- without parsing it into the internal type,
 * and without catching exceptions.
 *
 * Common use cases for standalone validation:
 *
 * - **Editor tooling**: A language server or linter plugin that highlights
 *   spec errors inline. It needs a list of errors, not a thrown exception.
 *
 * - **CI pipelines**: A validation step that checks all spec files in a
 *   directory and reports all errors, rather than failing on the first one.
 *
 * - **Programmatic checks**: Code that needs to branch on "is this spec
 *   valid?" without try/catch overhead.
 *
 * === How it works ===
 *
 * Under the hood, both validateSpec() and validateSpecObject() delegate to
 * SpecLoader. The validation logic lives in exactly one place -- SpecLoader's
 * private _parseSpec method. The validate functions simply wrap the
 * exception-throwing API in a result-returning API:
 *
 *   1. Create a SpecLoader instance
 *   2. Call load() or loadFromObject() inside a try block
 *   3. If it succeeds, return { valid: true, errors: [] }
 *   4. If it throws SpecError, return { valid: false, errors: [message] }
 *   5. If it throws anything else (e.g., file not found), still capture
 *      the error message and return { valid: false, errors: [message] }
 *
 * This "adapter pattern" means validation rules are never duplicated.
 * When SpecLoader learns a new check, validateSpec() inherits it for free.
 *
 * @module validate
 */

import { readFileSync } from "fs";
import { SpecLoader } from "./spec-loader.js";
import { SpecError } from "./errors.js";

// ---------------------------------------------------------------------------
// ValidationResult -- the return type for both validation functions
// ---------------------------------------------------------------------------

/**
 * The result of validating a CLI Builder JSON spec.
 *
 * When `valid` is true, `errors` is guaranteed to be an empty array.
 * When `valid` is false, `errors` contains one or more human-readable
 * error messages describing what went wrong.
 *
 * This is a simple "either" pattern:
 *
 *   valid === true  -->  errors.length === 0
 *   valid === false -->  errors.length >= 1
 */
export interface ValidationResult {
  /** Whether the spec passed all validation checks. */
  valid: boolean;
  /** Human-readable error messages. Empty array when valid. */
  errors: string[];
}

// ---------------------------------------------------------------------------
// validateSpec -- validate from a file path
// ---------------------------------------------------------------------------

/**
 * Validate a CLI Builder JSON spec file at the given path.
 *
 * This function reads the file, parses it as JSON, and runs all of
 * SpecLoader's validation checks. It never throws -- all failures are
 * captured in the returned ValidationResult.
 *
 * Possible error scenarios:
 * - The file does not exist or cannot be read
 * - The file contains invalid JSON
 * - The JSON is valid but fails spec validation rules (see spec-loader.ts)
 *
 * @param specFilePath - Absolute or relative path to the JSON spec file.
 * @returns A ValidationResult with valid=true if the spec is good,
 *          or valid=false with error messages if it is not.
 *
 * @example
 * ```typescript
 * const result = validateSpec("./my-tool.json");
 * if (!result.valid) {
 *   for (const err of result.errors) {
 *     console.error("Spec error:", err);
 *   }
 *   process.exit(1);
 * }
 * ```
 */
export function validateSpec(specFilePath: string): ValidationResult {
  const loader = new SpecLoader(specFilePath);
  try {
    loader.load();
    return { valid: true, errors: [] };
  } catch (e: unknown) {
    const message =
      e instanceof Error ? e.message : String(e);
    return { valid: false, errors: [message] };
  }
}

// ---------------------------------------------------------------------------
// validateSpecObject -- validate from an already-parsed object
// ---------------------------------------------------------------------------

/**
 * Validate a CLI Builder JSON spec from a plain JavaScript object.
 *
 * This is the "no I/O" variant: you already have the parsed JSON (perhaps
 * from an API response, an embedded literal, or a test fixture), and you
 * want to check whether it conforms to the CLI Builder spec format.
 *
 * Like validateSpec(), this never throws. All validation failures are
 * captured in the returned ValidationResult.
 *
 * @param raw - The parsed JSON object to validate.
 * @returns A ValidationResult indicating whether the spec is valid.
 *
 * @example
 * ```typescript
 * const spec = {
 *   cli_builder_spec_version: "1.0",
 *   name: "my-tool",
 *   description: "Does things",
 * };
 * const result = validateSpecObject(spec);
 * console.log(result.valid); // true
 * ```
 */
export function validateSpecObject(
  raw: Record<string, unknown>,
): ValidationResult {
  // We use a dummy file path since loadFromObject() never reads the file.
  // The path is only stored for error messages in load(), which we bypass.
  const loader = new SpecLoader("<in-memory>");
  try {
    loader.loadFromObject(raw);
    return { valid: true, errors: [] };
  } catch (e: unknown) {
    const message =
      e instanceof Error ? e.message : String(e);
    return { valid: false, errors: [message] };
  }
}
