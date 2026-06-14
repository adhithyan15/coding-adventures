import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, expect, it } from "vitest";

import {
  compileGrammarFile,
  compileGrammarSource,
  inferGrammarSourceKind,
} from "../program/compiler.js";

describe("grammar-tools program compiler", () => {
  it("infers grammar kind from standard extensions", () => {
    expect(inferGrammarSourceKind("json.tokens")).toBe("tokens");
    expect(inferGrammarSourceKind("json.grammar")).toBe("grammar");
  });

  it("compiles any token grammar source to a static TypeScript module", () => {
    const code = compileGrammarSource("NAME = /[a-z]+/\nkeywords:\n  if\n", {
      kind: "tokens",
      sourcePath: "example.tokens",
    });

    expect(code).toContain("Source: example.tokens");
    expect(code).toContain("export const TOKEN_GRAMMAR");
    expect(code).toContain("NAME");
    expect(code).toContain("if");
  });

  it("compiles any parser grammar source to a static TypeScript module", () => {
    const code = compileGrammarSource("value = NUMBER ;\n", {
      kind: "grammar",
      sourcePath: "example.grammar",
    });

    expect(code).toContain("Source: example.grammar");
    expect(code).toContain("export const PARSER_GRAMMAR");
    expect(code).toContain("value");
  });

  it("writes compiled modules for files", async () => {
    const dir = await mkdtemp(join(tmpdir(), "grammar-tools-"));
    try {
      const outputPath = join(dir, "generated", "token-grammar.ts");
      await compileGrammarFile("../../../grammars/macsyma/macsyma.tokens", { outputPath });
      const generated = await readFile(outputPath, "utf8");

      expect(generated).toContain("Source: ../../../grammars/macsyma/macsyma.tokens");
      expect(generated).toContain("export const TOKEN_GRAMMAR");
      expect(generated).toContain("COLONEQ");
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  it("keeps the CLI entrypoint wired through cli-builder", async () => {
    const source = await readFile("program/index.ts", "utf8");
    const spec = await readFile("program/grammar-tools.cli.json", "utf8");

    expect(source).toContain("@coding-adventures/cli-builder");
    expect(source).toContain("new Parser");
    expect(spec).toContain("\"compile\"");
    expect(spec).toContain("\"compile-tokens\"");
    expect(spec).toContain("\"compile-grammar\"");
  });
});
