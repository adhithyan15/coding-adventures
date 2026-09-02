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
  assertKnownFigureTarget,
  FIGURE_HASH_MANIFEST_PATH,
  generatedFigureOutputs,
  runFigureGeneration,
} from "../src/figure-cli.js";
import { FILMSTRIP_LEDGER_PATH } from "../src/figure-filmstrip.js";
import type { FigureTarget } from "../src/figure.js";

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

// ---------------------------------------------------------------------------
// script-filmstrip targets
// ---------------------------------------------------------------------------

/**
 * A curriculum root with one writing lesson and one filmstrip target. The
 * ledger is written by hand here — the real one is generated and byte-checked
 * by `script-ductus` — so this exercises the SHELL: does the CLI find the
 * ledger, match the entry, and put the file in the right track's book?
 */
function filmstripFixture(options: { ledger?: unknown } = {}): string {
  const root = mkdtempSync(join(tmpdir(), "human-language-filmstrip-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "data", "ductus"), { recursive: true });
  mkdirSync(join(root, "tamil", "lessons"), { recursive: true });
  writeFileSync(join(root, "core", "figure-generation.json"), `${JSON.stringify({
    version: 1,
    targets: [
      {
        kind: "script-filmstrip",
        lessonId: "TA-S119-letter-a",
        script: "tamil",
        glyph: "\u0b85",
        output: "tamil/book/figures/TA-S119-letter-a-filmstrip.svg",
      },
    ],
  })}\n`);
  writeFileSync(
    join(root, FILMSTRIP_LEDGER_PATH),
    `${JSON.stringify(
      options.ledger ?? {
        version: 1,
        generator: "test",
        entries: [
          {
            script: "tamil",
            glyph: "\u0b85",
            font: "_fonts/NotoSansTamil-Static.ttf",
            source: { citation: "A cited manual", url: "https://example.org/manual" },
            penLifts: 0,
            summary: "one unbroken stroke \u00b7 1 movement",
            viewBox: { minX: 0, minY: 0, width: 100, height: 200 },
            frames: [
              {
                number: 1,
                label: "curl around the upper loop",
                startsAfterLift: false,
                markup: '<path d="M0 0L1 1"/>',
              },
            ],
          },
        ],
      },
    )}\n`,
  );
  writeFileSync(join(root, "tamil", "lessons", "letter-a.md"), `---
schema_version: 2
id: TA-S119-letter-a
spine_node: TEST
sequence: 10
chapter: 15
type: writing
headword: "\u0b85"
gloss: the single character
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: []
introduces:
  knowledge: [TA-SCRIPT-RECOG-119]
practises:
  knowledge: [TA-SCRIPT-RECOG-119]
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# \u0b85

## The letter, taken apart

Follow the hand.
`);
  return root;
}

describe("script-filmstrip targets", () => {
  it("writes the strip into the lesson's own track book", () => {
    const root = filmstripFixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    expect(runFigureGeneration(["--write"], root)).toBe(0);
    const svg = join(root, "tamil", "book", "figures", "TA-S119-letter-a-filmstrip.svg");
    expect(existsSync(svg)).toBe(true);
    const written = readFileSync(svg, "utf8");
    expect(written).toContain('<path d="M0 0L1 1"/>');
    expect(written).toContain("Stroke order after A cited");
    expect(runFigureGeneration(["--check"], root)).toBe(0);
  });

  it("names the regeneration step when the ledger has no such letter", () => {
    const root = filmstripFixture({
      ledger: { version: 1, generator: "test", entries: [] },
    });
    expect(() => generatedFigureOutputs(root)).toThrow(
      /generate:filmstrip-ledger/,
    );
  });

  it("refuses a target that does not say which letter it draws", () => {
    expect(() =>
      assertKnownFigureTarget({
        kind: "script-filmstrip",
        lessonId: "TA-S119-letter-a",
        script: "tamil",
        glyph: "",
        output: "tamil/book/figures/x.svg",
      }),
    ).toThrow(/needs a glyph/);
    expect(() =>
      assertKnownFigureTarget({
        kind: "script-filmstrip",
        lessonId: "TA-S119-letter-a",
        script: "Tamil Script",
        glyph: "\u0b85",
        output: "tamil/book/figures/x.svg",
      }),
    ).toThrow(/canonical script id/);
    expect(() =>
      assertKnownFigureTarget({
        kind: "cave-painting",
        lessonId: "TA-S119-letter-a",
        output: "tamil/book/figures/x.svg",
      } as unknown as FigureTarget),
    ).toThrow(/unknown figure kind/);
    expect(() =>
      assertKnownFigureTarget({
        kind: "etymology-route",
        lessonId: "",
        output: "spanish/book/figures/x.svg",
      }),
    ).toThrow(/needs a lessonId/);
  });
});
