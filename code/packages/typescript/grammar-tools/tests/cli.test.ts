/**
 * Tests for the grammar-tools CLI entry point (cli.ts).
 *
 * =============================================================================
 * TESTING STRATEGY
 * =============================================================================
 *
 * Rather than spawning a subprocess for every test (which is slow and adds
 * I/O complexity), we test the exported functions directly:
 *
 *   - ``validateCommand(tokensPath, grammarPath)``
 *   - ``validateTokensOnly(tokensPath)``
 *   - ``validateGrammarOnly(grammarPath)``
 *   - ``main()``
 *   - ``printUsage()``
 *
 * We capture stdout by replacing ``process.stdout.write`` with a mock, and
 * we write real temporary files to disk using ``fs.writeFileSync`` so that
 * the file-reading code paths are exercised.
 *
 * =============================================================================
 * WHY NOT SPAWN A SUBPROCESS?
 * =============================================================================
 *
 * Spawning requires the TypeScript to be compiled first (``tsc``), which ties
 * the tests to the build system. Testing the exported functions directly is
 * faster, more focused, and works with ``vitest run`` without a prior build.
 * The CLI shebang and ``process.exit()`` call are tested separately with
 * mocking so they do not kill the test runner.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { writeFileSync, mkdirSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";

import {
  validateCommand,
  validateTokensOnly,
  validateGrammarOnly,
  printUsage,
  main,
} from "../src/cli.js";

// ---------------------------------------------------------------------------
// Fixtures — tiny .tokens and .grammar files written to a temp directory
// ---------------------------------------------------------------------------

/**
 * A valid .tokens file with two tokens: NUMBER and PLUS.
 * Both are referenced in the grammar fixture below, so cross-validation
 * should produce no errors and no unused-token warnings.
 */
const VALID_TOKENS = `NUMBER = /[0-9]+/
PLUS   = "+"
`;

/**
 * A valid .grammar file that uses both tokens defined above.
 * Has one rule: ``expression = NUMBER { PLUS NUMBER } ;``
 */
const VALID_GRAMMAR = `expression = NUMBER { PLUS NUMBER } ;
`;

/**
 * A .tokens file whose only token (MINUS) is NOT used in the grammar.
 * Cross-validation should emit a warning (not an error) for the unused token.
 */
const TOKENS_WITH_UNUSED = `NUMBER = /[0-9]+/
MINUS  = "-"
`;

/**
 * A .grammar file that references PLUS which is not defined in the tokens file.
 */
const GRAMMAR_WITH_MISSING_TOKEN = `expression = NUMBER PLUS NUMBER ;
`;

/**
 * A .tokens file with a duplicate token name — triggers a validation error.
 */
const TOKENS_WITH_DUPLICATE = `NUMBER = /[0-9]+/
NUMBER = /[0-9]+/
`;

/**
 * A .grammar file with an undefined rule reference.
 */
const GRAMMAR_WITH_UNDEFINED_RULE = `expression = term PLUS term ;
`;

// ---------------------------------------------------------------------------
// Setup — write fixture files to a temp directory
// ---------------------------------------------------------------------------

let tmpDir: string;
let validTokensPath: string;
let validGrammarPath: string;
let tokensWithUnusedPath: string;
let grammarWithMissingPath: string;
let tokensWithDuplicatePath: string;
let grammarWithUndefinedRulePath: string;

/**
 * Helper that replaces ``process.stdout.write`` with a mock and returns a
 * function that restores the original and returns all captured output.
 */
function captureStdout(): () => string {
  const chunks: string[] = [];
  const original = process.stdout.write.bind(process.stdout);
  process.stdout.write = (chunk: string | Uint8Array): boolean => {
    chunks.push(chunk.toString());
    return true;
  };
  return () => {
    process.stdout.write = original;
    return chunks.join("");
  };
}

beforeEach(() => {
  /** Create a unique temporary directory for each test. */
  tmpDir = join(tmpdir(), `grammar-tools-cli-test-${Date.now()}`);
  mkdirSync(tmpDir, { recursive: true });

  validTokensPath = join(tmpDir, "valid.tokens");
  validGrammarPath = join(tmpDir, "valid.grammar");
  tokensWithUnusedPath = join(tmpDir, "unused.tokens");
  grammarWithMissingPath = join(tmpDir, "missing.grammar");
  tokensWithDuplicatePath = join(tmpDir, "dup.tokens");
  grammarWithUndefinedRulePath = join(tmpDir, "undef.grammar");

  writeFileSync(validTokensPath, VALID_TOKENS, "utf-8");
  writeFileSync(validGrammarPath, VALID_GRAMMAR, "utf-8");
  writeFileSync(tokensWithUnusedPath, TOKENS_WITH_UNUSED, "utf-8");
  writeFileSync(grammarWithMissingPath, GRAMMAR_WITH_MISSING_TOKEN, "utf-8");
  writeFileSync(tokensWithDuplicatePath, TOKENS_WITH_DUPLICATE, "utf-8");
  writeFileSync(grammarWithUndefinedRulePath, GRAMMAR_WITH_UNDEFINED_RULE, "utf-8");
});

afterEach(() => {
  /** Remove the temp directory after each test so we don't accumulate files. */
  rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// validate command — pair validation
// ---------------------------------------------------------------------------

describe("validateCommand", () => {
  it("should return 0 and print OK for a valid pair", () => {
    /**
     * A fully consistent pair of .tokens and .grammar files should produce
     * no errors, no warnings, and exit with code 0.
     */
    const restore = captureStdout();
    const code = validateCommand(validTokensPath, validGrammarPath);
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("OK");
    expect(output).toContain("All checks passed.");
  });

  it("should include token count in OK line", () => {
    /** The output line should say '2 tokens' for our two-token fixture. */
    const restore = captureStdout();
    validateCommand(validTokensPath, validGrammarPath);
    const output = restore();

    expect(output).toMatch(/OK \(2 tokens\)/);
  });

  it("should include rule count in OK line", () => {
    /** The output line should say '1 rules' for our one-rule grammar. */
    const restore = captureStdout();
    validateCommand(validTokensPath, validGrammarPath);
    const output = restore();

    expect(output).toMatch(/OK \(1 rules\)/);
  });

  it("should return 1 when tokens file has validation errors", () => {
    /**
     * Duplicate token names in the .tokens file are validation errors.
     * The command should print the error details and return exit code 1.
     */
    const restore = captureStdout();
    const code = validateCommand(tokensWithDuplicatePath, validGrammarPath);
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("error(s)");
    expect(output).toContain("Duplicate");
    expect(output).toContain("Fix them and try again.");
  });

  it("should return 1 when grammar has missing token references", () => {
    /**
     * The grammar references PLUS, which is not in the minimal tokens file.
     * Cross-validation should catch this as an error.
     */
    const tokensOnlyNumber = join(tmpDir, "number-only.tokens");
    writeFileSync(tokensOnlyNumber, "NUMBER = /[0-9]+/\n", "utf-8");

    const restore = captureStdout();
    const code = validateCommand(tokensOnlyNumber, grammarWithMissingPath);
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("PLUS");
  });

  it("should return 1 when tokens file does not exist", () => {
    /** A missing file should print an error and return exit code 1. */
    const restore = captureStdout();
    const code = validateCommand("/nonexistent/path/file.tokens", validGrammarPath);
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("Error");
  });

  it("should return 1 when grammar file does not exist", () => {
    const restore = captureStdout();
    const code = validateCommand(validTokensPath, "/nonexistent/path/file.grammar");
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("Error");
  });

  it("should print warnings for unused tokens but still return 0", () => {
    /**
     * An unused token is a warning, not an error. The cross-validation should
     * note the warning but the command should still exit with code 0.
     *
     * We write a custom pair: a tokens file that defines NUMBER and UNUSED,
     * and a grammar that only uses NUMBER. UNUSED should trigger a warning
     * from cross-validation, but no errors, so the exit code is 0.
     */
    const customTokensPath = join(tmpDir, "with-unused.tokens");
    const customGrammarPath = join(tmpDir, "number-only.grammar");
    writeFileSync(customTokensPath, "NUMBER = /[0-9]+/\nUNUSED = /x+/\n", "utf-8");
    writeFileSync(customGrammarPath, "expr = NUMBER ;\n", "utf-8");

    const restore = captureStdout();
    const code = validateCommand(customTokensPath, customGrammarPath);
    const output = restore();

    // Warnings do not cause failure
    expect(code).toBe(0);
    expect(output).toContain("warning(s)");
    expect(output).toContain("All checks passed.");
  });

  it("should show Cross-validating line", () => {
    /** The validate command always runs cross-validation and shows its output. */
    const restore = captureStdout();
    validateCommand(validTokensPath, validGrammarPath);
    const output = restore();

    expect(output).toContain("Cross-validating");
  });
});

// ---------------------------------------------------------------------------
// validate-tokens command — tokens-only validation
// ---------------------------------------------------------------------------

describe("validateTokensOnly", () => {
  it("should return 0 and print OK for a valid tokens file", () => {
    const restore = captureStdout();
    const code = validateTokensOnly(validTokensPath);
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("OK");
    expect(output).toContain("All checks passed.");
  });

  it("should include token count in OK line", () => {
    const restore = captureStdout();
    validateTokensOnly(validTokensPath);
    const output = restore();

    expect(output).toMatch(/OK \(2 tokens\)/);
  });

  it("should return 1 for a tokens file with duplicate names", () => {
    const restore = captureStdout();
    const code = validateTokensOnly(tokensWithDuplicatePath);
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("Duplicate");
    expect(output).toContain("Fix them and try again.");
  });

  it("should return 1 when file does not exist", () => {
    const restore = captureStdout();
    const code = validateTokensOnly("/no/such/file.tokens");
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("Error");
  });
});

// ---------------------------------------------------------------------------
// validate-grammar command — grammar-only validation
// ---------------------------------------------------------------------------

describe("validateGrammarOnly", () => {
  it("should return 0 and print OK for a valid grammar file", () => {
    const restore = captureStdout();
    const code = validateGrammarOnly(validGrammarPath);
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("OK");
    expect(output).toContain("All checks passed.");
  });

  it("should include rule count in OK line", () => {
    const restore = captureStdout();
    validateGrammarOnly(validGrammarPath);
    const output = restore();

    expect(output).toMatch(/OK \(1 rules\)/);
  });

  it("should return 1 for a grammar with duplicate rules", () => {
    /**
     * A grammar that defines the same rule name twice should fail validation.
     * We write a fresh fixture inline for this specific case.
     */
    const dupRulesPath = join(tmpDir, "dup-rules.grammar");
    writeFileSync(dupRulesPath, "expr = NUMBER ;\nexpr = NUMBER ;\n", "utf-8");

    const restore = captureStdout();
    const code = validateGrammarOnly(dupRulesPath);
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("Duplicate");
  });

  it("should return 1 when file does not exist", () => {
    const restore = captureStdout();
    const code = validateGrammarOnly("/no/such/file.grammar");
    const output = restore();

    expect(code).toBe(1);
    expect(output).toContain("Error");
  });
});

// ---------------------------------------------------------------------------
// printUsage — help text
// ---------------------------------------------------------------------------

describe("printUsage", () => {
  it("should print usage information mentioning all three commands", () => {
    const restore = captureStdout();
    printUsage();
    const output = restore();

    expect(output).toContain("validate");
    expect(output).toContain("validate-tokens");
    expect(output).toContain("validate-grammar");
  });

  it("should mention grammar-tools as the binary name", () => {
    const restore = captureStdout();
    printUsage();
    const output = restore();

    expect(output).toContain("grammar-tools");
  });
});

// ---------------------------------------------------------------------------
// main() — argument dispatch
// ---------------------------------------------------------------------------

describe("main", () => {
  afterEach(() => {
    /** Restore process.argv after each test. */
    vi.restoreAllMocks();
  });

  it("should return 0 and print usage for --help", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js", "--help"]);

    const restore = captureStdout();
    const code = main();
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("grammar-tools");
  });

  it("should return 0 and print usage for -h", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js", "-h"]);

    const restore = captureStdout();
    const code = main();
    restore();

    expect(code).toBe(0);
  });

  it("should return 0 and print usage when called with no args", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js"]);

    const restore = captureStdout();
    const code = main();
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("grammar-tools");
  });

  it("should return 2 for unknown command", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js", "unknown-cmd"]);

    const restore = captureStdout();
    const code = main();
    const output = restore();

    expect(code).toBe(2);
    expect(output).toContain("Unknown command");
  });

  it("should return 2 when validate is missing arguments", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js", "validate", "only-one.tokens"]);

    const restore = captureStdout();
    const code = main();
    restore();

    expect(code).toBe(2);
  });

  it("should return 2 when validate-tokens is missing argument", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js", "validate-tokens"]);

    const restore = captureStdout();
    const code = main();
    restore();

    expect(code).toBe(2);
  });

  it("should return 2 when validate-grammar is missing argument", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue(["node", "cli.js", "validate-grammar"]);

    const restore = captureStdout();
    const code = main();
    restore();

    expect(code).toBe(2);
  });

  it("should dispatch validate command successfully", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue([
      "node", "cli.js", "validate", validTokensPath, validGrammarPath,
    ]);

    const restore = captureStdout();
    const code = main();
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("All checks passed.");
  });

  it("should dispatch validate-tokens command successfully", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue([
      "node", "cli.js", "validate-tokens", validTokensPath,
    ]);

    const restore = captureStdout();
    const code = main();
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("All checks passed.");
  });

  it("should dispatch validate-grammar command successfully", () => {
    vi.spyOn(process, "argv", "get").mockReturnValue([
      "node", "cli.js", "validate-grammar", validGrammarPath,
    ]);

    const restore = captureStdout();
    const code = main();
    const output = restore();

    expect(code).toBe(0);
    expect(output).toContain("All checks passed.");
  });
});
