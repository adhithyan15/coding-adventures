import { describe, it, expect } from "vitest";
import { existsSync, readFileSync, mkdirSync } from "fs";
import { join, resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { tmpdir } from "os";

import {
  dispatch,
  validateCommand,
  validateTokensOnly,
  validateGrammarOnly,
  compileTokensCommand,
  compileGrammarCommand,
} from "./index.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

function findRoot(): string {
  let current = resolve(__dirname);
  for (let i = 0; i < 20; i++) {
    if (existsSync(join(current, "code", "specs", "grammar-tools.json"))) {
      return current;
    }
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
  return resolve(__dirname);
}

const ROOT = findRoot();
const GRAMMARS = join(ROOT, "code", "grammars");

function grammarPath(name: string): string {
  return join(GRAMMARS, name);
}

function exists(name: string): boolean {
  return existsSync(grammarPath(name));
}

// ---------------------------------------------------------------------------
// validateCommand
// ---------------------------------------------------------------------------

describe("validateCommand", () => {
  it("succeeds on json pair", () => {
    if (!exists("json.tokens") || !exists("json.grammar")) return;
    expect(validateCommand(grammarPath("json.tokens"), grammarPath("json.grammar"))).toBe(0);
  });

  it("succeeds on lisp pair", () => {
    if (!exists("lisp.tokens") || !exists("lisp.grammar")) return;
    expect(validateCommand(grammarPath("lisp.tokens"), grammarPath("lisp.grammar"))).toBe(0);
  });

  it("returns 1 on missing tokens file", () => {
    expect(validateCommand("/nonexistent/x.tokens", "any.grammar")).toBe(1);
  });

  it("returns 1 on missing grammar file", () => {
    if (!exists("json.tokens")) return;
    expect(validateCommand(grammarPath("json.tokens"), "/nonexistent/x.grammar")).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// validateTokensOnly
// ---------------------------------------------------------------------------

describe("validateTokensOnly", () => {
  it("succeeds on json.tokens", () => {
    if (!exists("json.tokens")) return;
    expect(validateTokensOnly(grammarPath("json.tokens"))).toBe(0);
  });

  it("returns 1 on missing file", () => {
    expect(validateTokensOnly("/nonexistent/x.tokens")).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// validateGrammarOnly
// ---------------------------------------------------------------------------

describe("validateGrammarOnly", () => {
  it("succeeds on json.grammar", () => {
    if (!exists("json.grammar")) return;
    expect(validateGrammarOnly(grammarPath("json.grammar"))).toBe(0);
  });

  it("returns 1 on missing file", () => {
    expect(validateGrammarOnly("/nonexistent/x.grammar")).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// dispatch
// ---------------------------------------------------------------------------

describe("dispatch", () => {
  it("unknown command returns 2", () => {
    expect(dispatch("unknown", [])).toBe(2);
  });

  it("validate with wrong file count returns 2", () => {
    expect(dispatch("validate", ["one.tokens"])).toBe(2);
  });

  it("validate-tokens with no files returns 2", () => {
    expect(dispatch("validate-tokens", [])).toBe(2);
  });

  it("validate-grammar with no files returns 2", () => {
    expect(dispatch("validate-grammar", [])).toBe(2);
  });

  it("validate dispatches correctly", () => {
    if (!exists("json.tokens") || !exists("json.grammar")) return;
    expect(dispatch("validate", [grammarPath("json.tokens"), grammarPath("json.grammar")])).toBe(0);
  });

  it("validate-tokens dispatches correctly", () => {
    if (!exists("json.tokens")) return;
    expect(dispatch("validate-tokens", [grammarPath("json.tokens")])).toBe(0);
  });

  it("validate-grammar dispatches correctly", () => {
    if (!exists("json.grammar")) return;
    expect(dispatch("validate-grammar", [grammarPath("json.grammar")])).toBe(0);
  });

  it("compile-tokens with no files returns 2", () => {
    expect(dispatch("compile-tokens", [])).toBe(2);
  });

  it("compile-grammar with no files returns 2", () => {
    expect(dispatch("compile-grammar", [])).toBe(2);
  });

  it("compile-tokens dispatches correctly", () => {
    if (!exists("json.tokens")) return;
    expect(dispatch("compile-tokens", [grammarPath("json.tokens")])).toBe(0);
  });

  it("compile-grammar dispatches correctly", () => {
    if (!exists("json.grammar")) return;
    expect(dispatch("compile-grammar", [grammarPath("json.grammar")])).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// compileTokensCommand
// ---------------------------------------------------------------------------

describe("compileTokensCommand", () => {
  it("returns 1 for missing file", () => {
    expect(compileTokensCommand("/nonexistent/x.tokens", undefined)).toBe(1);
  });

  it("returns 0 and writes file when output path given", () => {
    if (!exists("json.tokens")) return;
    const outDir = join(tmpdir(), "grammar-tools-ts-test");
    mkdirSync(outDir, { recursive: true });
    const outPath = join(outDir, "json-tokens.ts");
    const result = compileTokensCommand(grammarPath("json.tokens"), outPath);
    expect(result).toBe(0);
    const content = readFileSync(outPath, "utf-8");
    expect(content).toContain("TOKEN_GRAMMAR");
    expect(content).toContain("DO NOT EDIT");
  });

  it("returns 0 for valid file with no output path (stdout)", () => {
    if (!exists("json.tokens")) return;
    expect(compileTokensCommand(grammarPath("json.tokens"), undefined)).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// compileGrammarCommand
// ---------------------------------------------------------------------------

describe("compileGrammarCommand", () => {
  it("returns 1 for missing file", () => {
    expect(compileGrammarCommand("/nonexistent/x.grammar", undefined)).toBe(1);
  });

  it("returns 0 and writes file when output path given", () => {
    if (!exists("json.grammar")) return;
    const outDir = join(tmpdir(), "grammar-tools-ts-test");
    mkdirSync(outDir, { recursive: true });
    const outPath = join(outDir, "json-grammar.ts");
    const result = compileGrammarCommand(grammarPath("json.grammar"), outPath);
    expect(result).toBe(0);
    const content = readFileSync(outPath, "utf-8");
    expect(content).toContain("PARSER_GRAMMAR");
    expect(content).toContain("DO NOT EDIT");
  });

  it("returns 0 for valid file with no output path (stdout)", () => {
    if (!exists("json.grammar")) return;
    expect(compileGrammarCommand(grammarPath("json.grammar"), undefined)).toBe(0);
  });
});
