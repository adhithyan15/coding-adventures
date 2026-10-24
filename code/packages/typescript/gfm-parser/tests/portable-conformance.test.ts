/**
 * TE04's portable corpus is deliberately outside this package.  Reading it at
 * test time keeps one language-neutral oracle while the package continues to
 * publish only parser code.  The loop below is data-driven: adding a valid
 * corpus case automatically adds a native parse-and-render assertion.
 */
import { readFileSync } from "node:fs";
import { TextDecoder } from "node:util";
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
const MAX_CORPUS_BYTES = 131_072;

function parseCorpus(raw: Buffer): PortableCorpus {
  if (raw.byteLength > MAX_CORPUS_BYTES) {
    throw new Error("portable GFM corpus exceeds 131072 bytes");
  }
  if (raw.subarray(0, 3).equals(Buffer.from([0xef, 0xbb, 0xbf]))) {
    throw new Error("portable GFM corpus must not start with a byte-order mark");
  }
  const text = new TextDecoder("utf-8", { fatal: true }).decode(raw);
  return JSON.parse(text) as PortableCorpus;
}

function loadCorpus(): PortableCorpus {
  return parseCorpus(readFileSync(corpusUrl));
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
const corruptionCase = corpus.cases[0];
if (corruptionCase === undefined) {
  throw new Error("portable GFM corpus must contain a corruption witness case");
}

describe("TE04 portable GFM extension conformance", () => {
  it("loads the closed v1 corpus without a package-local case filter", () => {
    expect(corpus.schema_version).toBe(1);
    expect(corpus.profile).toBe("gfm-extension-v1");
    expect(corpus.cases).toHaveLength(16);
  });

  it("bounds and strictly decodes the native fixture read", () => {
    expect(() => parseCorpus(Buffer.alloc(MAX_CORPUS_BYTES + 1))).toThrow(
      /exceeds 131072 bytes/,
    );
    expect(() => parseCorpus(Buffer.from([0xef, 0xbb, 0xbf, 0x7b, 0x7d]))).toThrow(
      /byte-order mark/,
    );
    expect(() => parseCorpus(Buffer.from([0xff]))).toThrow();
  });

  for (const testCase of corpus.cases) {
    it(testCase.id, () => assertPortableCase(testCase));
  }

  it("detects independent source corruption", () => {
    expect(() =>
      assertPortableCase({
        ...corruptionCase,
        source: `${corruptionCase.source}\nportable corruption witness\n`,
      }),
    ).toThrow();
  });

  it("detects independent expected-AST corruption", () => {
    expect(() =>
      assertPortableCase({
        ...corruptionCase,
        expected_ast: { type: "document", children: [] },
      }),
    ).toThrow();
  });

  it("detects independent expected-HTML corruption", () => {
    expect(() =>
      assertPortableCase({
        ...corruptionCase,
        expected_html: `${corruptionCase.expected_html}<!-- corruption witness -->`,
      }),
    ).toThrow();
  });
});
