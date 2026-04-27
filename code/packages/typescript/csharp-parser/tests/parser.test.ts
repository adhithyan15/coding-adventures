import { describe, expect, it } from "vitest";
import type { ASTNode } from "@coding-adventures/parser";

import { createCSharpParser, parseCSharp } from "../src/parser.js";

function assertParsesCompilationUnit(
  source: string,
  version?: string
): ASTNode {
  const ast = parseCSharp(source, version);
  expect(ast.ruleName).toBe("compilation_unit");
  return ast;
}

describe("class declarations", () => {
  it("parses a simple class", () => {
    assertParsesCompilationUnit("class Hello {}");
  });

  it("parses a public class", () => {
    assertParsesCompilationUnit("public class Main {}");
  });

  it("parses a namespaced class", () => {
    assertParsesCompilationUnit("namespace MyApp { public class Greeter {} }");
  });

  it("parses a method inside a class", () => {
    assertParsesCompilationUnit("class Program { void Main() {} }");
  });
});

describe("createCSharpParser", () => {
  it("returns a parser with a parse method", () => {
    const parser = createCSharpParser("public class Foo {}");
    expect(typeof parser.parse).toBe("function");
  });

  it("produces the same root rule as parseCSharp", () => {
    const source = "public class Foo {}";
    const astDirect = parseCSharp(source);
    const astFactory = createCSharpParser(source).parse();

    expect(astDirect.ruleName).toBe(astFactory.ruleName);
    expect(astDirect.children.length).toBe(astFactory.children.length);
  });

  it("accepts a version string", () => {
    const ast = createCSharpParser("public class Foo {}", "8.0").parse();
    expect(ast.ruleName).toBe("compilation_unit");
  });

  it("throws for unknown version", () => {
    expect(() => createCSharpParser("public class Foo {}", "99.0")).toThrow(
      /Unknown C# version "99.0"/
    );
  });
});

describe("version-aware parsing", () => {
  const classCases = [
    "1.0",
    "2.0",
    "3.0",
    "4.0",
    "5.0",
    "6.0",
    "7.0",
    "8.0",
    "9.0",
    "10.0",
    "11.0",
    "12.0",
  ];

  for (const version of classCases) {
    it(`parses a class declaration in C# ${version}`, () => {
      assertParsesCompilationUnit("public class Foo {}", version);
    });
  }

  const topLevelCases = [
    { label: "9.0", version: "9.0" },
    { label: "10.0", version: "10.0" },
    { label: "11.0", version: "11.0" },
    { label: "12.0", version: "12.0" },
    { label: "default", version: undefined },
  ];

  for (const { label, version } of topLevelCases) {
    it(`parses top-level statements in C# ${label}`, () => {
      assertParsesCompilationUnit("int x = 1;", version);
    });
  }

  it("uses the default grammar when version is omitted", () => {
    assertParsesCompilationUnit("public class Foo {}");
  });

  it("uses the default grammar when version is empty", () => {
    assertParsesCompilationUnit("public class Foo {}", "");
  });

  it("throws for unknown C# version", () => {
    expect(() => parseCSharp("public class Foo {}", "99.0")).toThrow(
      /Unknown C# version "99.0"/
    );
  });
});
