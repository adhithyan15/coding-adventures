/**
 * TE04's portable corpus is deliberately outside this package.  Reading it at
 * test time keeps one language-neutral oracle while the package continues to
 * publish only parser code.  The loop below is data-driven: adding a valid
 * corpus case automatically adds a native parse-and-render assertion.
 */
import { readFileSync } from "node:fs";
import type { DocumentNode } from "@coding-adventures/document-ast";
import { toHtml } from "@coding-adventures/document-ast-to-html";
import { describe, expect, it } from "vitest";
import { parse } from "../src/index.js";

interface PortableCase {
  readonly id: string;
  readonly source: string;
  readonly expected_ast: DocumentNode;
  readonly expected_html: string;
}

interface PortableCorpus {
  readonly schema_version: number;
  readonly profile: string;
  readonly cases: readonly PortableCase[];
}

const corpusUrl = new URL(
  "../../../../specs/fixtures/gfm-extension-v1/cases.json",
  import.meta.url,
);

function loadCorpus(): PortableCorpus {
  return JSON.parse(readFileSync(corpusUrl, "utf8")) as PortableCorpus;
}

/** Exercise only the two public APIs named by the adoption contract. */
function assertPortableCase(testCase: PortableCase): void {
  const actualAst = parse(testCase.source);
  expect(actualAst, `${testCase.id}: complete Document AST`).toEqual(
    testCase.expected_ast,
  );
  expect(toHtml(actualAst), `${testCase.id}: exact HTML`).toBe(
    testCase.expected_html,
  );
}

const corpus = loadCorpus();

describe("TE04 portable GFM extension conformance", () => {
  it("loads the closed v1 corpus without a package-local case filter", () => {
    expect(corpus.schema_version).toBe(1);
    expect(corpus.profile).toBe("gfm-extension-v1");
    expect(corpus.cases).toHaveLength(16);
  });

  for (const testCase of corpus.cases) {
    it(testCase.id, () => assertPortableCase(testCase));
  }

  it("detects independent source corruption", () => {
    const testCase = corpus.cases[0]!;
    expect(() =>
      assertPortableCase({
        ...testCase,
        source: `${testCase.source}\nportable corruption witness\n`,
      }),
    ).toThrow();
  });

  it("detects independent expected-AST corruption", () => {
    const testCase = corpus.cases[0]!;
    expect(() =>
      assertPortableCase({
        ...testCase,
        expected_ast: { type: "document", children: [] },
      }),
    ).toThrow();
  });

  it("detects independent expected-HTML corruption", () => {
    const testCase = corpus.cases[0]!;
    expect(() =>
      assertPortableCase({
        ...testCase,
        expected_html: `${testCase.expected_html}<!-- corruption witness -->`,
      }),
    ).toThrow();
  });
});
