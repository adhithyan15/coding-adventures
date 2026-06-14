/**
 * language-labels.test.ts — raw → display label tests.
 */

import { describe, it, expect } from "vitest";
import { languageLabel } from "../src/index.js";

describe("languageLabel — known languages", () => {
  it("ts → TypeScript", () => expect(languageLabel("ts")).toBe("TypeScript"));
  it("tsx → TypeScript", () => expect(languageLabel("tsx")).toBe("TypeScript"));
  it("typescript → TypeScript", () => expect(languageLabel("typescript")).toBe("TypeScript"));
  it("js → JavaScript", () => expect(languageLabel("js")).toBe("JavaScript"));
  it("py → Python", () => expect(languageLabel("py")).toBe("Python"));
  it("rb → Ruby", () => expect(languageLabel("rb")).toBe("Ruby"));
  it("go → Go", () => expect(languageLabel("go")).toBe("Go"));
  it("rs → Rust", () => expect(languageLabel("rs")).toBe("Rust"));
  it("sh → Bash", () => expect(languageLabel("sh")).toBe("Bash"));
  it("bash → Bash", () => expect(languageLabel("bash")).toBe("Bash"));
  it("zsh → Bash", () => expect(languageLabel("zsh")).toBe("Bash"));
  it("json → JSON", () => expect(languageLabel("json")).toBe("JSON"));
  it("html → HTML", () => expect(languageLabel("html")).toBe("HTML"));
  it("css → CSS", () => expect(languageLabel("css")).toBe("CSS"));
  it("md → Markdown", () => expect(languageLabel("md")).toBe("Markdown"));
  it("yaml → YAML", () => expect(languageLabel("yaml")).toBe("YAML"));
  it("yml → YAML", () => expect(languageLabel("yml")).toBe("YAML"));
  it("toml → TOML", () => expect(languageLabel("toml")).toBe("TOML"));
  it("sql → SQL", () => expect(languageLabel("sql")).toBe("SQL"));
  it("cpp → C++", () => expect(languageLabel("cpp")).toBe("C++"));
  it("dockerfile → Dockerfile", () => expect(languageLabel("dockerfile")).toBe("Dockerfile"));
});

describe("languageLabel — case-insensitive lookup", () => {
  it("TypeScript → TypeScript (already-cased input)", () => {
    expect(languageLabel("TypeScript")).toBe("TypeScript");
  });
  it("TS → TypeScript", () => {
    expect(languageLabel("TS")).toBe("TypeScript");
  });
  it("PYTHON → Python", () => {
    expect(languageLabel("PYTHON")).toBe("Python");
  });
});

describe("languageLabel — fallthrough", () => {
  it("unknown raw passes through verbatim (preserves author capitalisation)", () => {
    expect(languageLabel("Cobol")).toBe("Cobol");
  });
  it("custom DSL hint passes through", () => {
    expect(languageLabel("my-dsl")).toBe("my-dsl");
  });
});

describe("languageLabel — null and empty", () => {
  it("null → null", () => expect(languageLabel(null)).toBeNull());
  it("empty string → null", () => expect(languageLabel("")).toBeNull());
  it("whitespace-only → null", () => expect(languageLabel("   ")).toBeNull());
  it("leading/trailing whitespace is trimmed", () => {
    expect(languageLabel("  ts  ")).toBe("TypeScript");
  });
});

describe("languageLabel — prototype-pollution defence", () => {
  it("'__proto__' falls through to the raw string (no inherited accessor leak)", () => {
    // If LABELS were a plain `{}`, LABELS["__proto__"] would return
    // Object.prototype's __proto__ accessor (an object, not a string).
    // With Object.create(null) it returns undefined, so we fall through.
    expect(languageLabel("__proto__")).toBe("__proto__");
  });
  it("'constructor' falls through (not Object's constructor)", () => {
    expect(languageLabel("constructor")).toBe("constructor");
  });
  it("'toString' falls through (not Object.prototype.toString)", () => {
    expect(languageLabel("toString")).toBe("toString");
  });
});
