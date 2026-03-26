import { describe, it, expect } from "vitest";
import { existsSync } from "fs";
import { join, resolve, dirname } from "path";
import { fileURLToPath } from "url";

import { dispatch, validateCommand, validateTokensOnly, validateGrammarOnly } from "./index.js";

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
});
