import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  FIGURE_HASH_MANIFEST_PATH,
  generatedFigureOutputs,
  runFigureGeneration,
} from "../src/figure-cli.js";

const roots: string[] = [];

function fixture(output = "spanish/book/figures/ES-C06-cafe-etymology.svg"): string {
  const root = mkdtempSync(join(tmpdir(), "human-language-figure-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "spanish", "lessons"), { recursive: true });
  writeFileSync(join(root, "core", "figure-generation.json"), `${JSON.stringify({
    version: 1,
    targets: [{ kind: "etymology-route", lessonId: "ES-C06-cafe", output }],
  })}\n`);
  writeFileSync(join(root, "spanish", "lessons", "cafe.md"), `---
schema_version: 2
id: ES-C06-cafe
spine_node: TEST
sequence: 10
chapter: 6
type: word
headword: café
gloss: coffee
concept_tag: ES-WORD-CAFE
roots: [qahwah-arabic, kahve-turkish, caffè-italian]
duration:
  max_seconds: 120
requires:
  knowledge: []
introduces:
  knowledge: [ES-LEX-CAFE]
practises:
  knowledge: [ES-LEX-CAFE]
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# café

## The word, taken apart

Follow the route.
`);
  return root;
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("figure generator filesystem shell", () => {
  it("writes and byte-checks the SVG plus its two hashes", () => {
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runFigureGeneration(["--write"], root)).toBe(0);
    const svg = join(root, "spanish", "book", "figures", "ES-C06-cafe-etymology.svg");
    expect(existsSync(svg)).toBe(true);
    expect(readFileSync(svg, "utf8")).toContain("qahwah");
    expect(readFileSync(join(root, FIGURE_HASH_MANIFEST_PATH), "utf8")).toContain(
      '"svgHash": "fnv1a64:',
    );
    expect(runFigureGeneration(["--check"], root)).toBe(0);
    writeFileSync(svg, "stale\n");
    expect(runFigureGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "spanish/book/figures/ES-C06-cafe-etymology.svg: generated output is missing or stale\n",
    );
  });

  it("rejects traversal and figures outside the lesson's track", () => {
    expect(() => generatedFigureOutputs(fixture("../../escape.svg"))).toThrow(/unsafe/);
    expect(() => generatedFigureOutputs(fixture("french/book/figures/cafe.svg"))).toThrow(
      /lesson's track/,
    );
  });

  it("returns usage status for an invalid mode", () => {
    const root = fixture();
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runFigureGeneration([], root)).toBe(2);
  });
});
