// HL-C443, sequence strips: several letters' cited strips printed as one
// figure, a labelled group per letter, short letters sharing a shelf.
import { describe, expect, it } from "vitest";
import {
  letterNumbers,
  renderScriptSequenceFilmstripFigure,
  scriptSequenceFilmstripFigureSource,
  sequenceUnit,
  shelveLetters,
  type FilmstripEntry,
} from "../src/figure-filmstrip.js";
import { renderFigure, type ScriptFilmstripTarget } from "../src/figure.js";
import { assertKnownFigureTarget } from "../src/figure-cli.js";
import type { ParsedLesson } from "../src/parse.js";

/** A tiny two-frame letter, like the one-letter strip's fixture. */
function entry(glyph: string, overrides: Partial<FilmstripEntry> = {}): FilmstripEntry {
  return {
    script: "tamil",
    glyph,
    font: "_fonts/NotoSansTamil-Static.ttf",
    source: { citation: "A cited primer", url: "https://example.org/primer" },
    penLifts: 1,
    summary: "2 strokes",
    viewBox: { minX: -10, minY: -100, width: 200, height: 300 },
    frames: [
      { number: 1, label: `${glyph} first`, startsAfterLift: false, markup: '<path d="M0 0L10 10"/>' },
      { number: 2, label: `${glyph} second`, startsAfterLift: true, markup: '<circle cx="5" cy="5" r="3"/>' },
    ],
    ...overrides,
  };
}

/** Every frame panel's top-left corner, in document order. */
function panels(svg: string): Array<readonly [number, number]> {
  return [...svg.matchAll(/<svg x="(-?[\d.]+)" y="(-?[\d.]+)" width="150"/g)].map(
    (match) => [Number(match[1]), Number(match[2])] as const,
  );
}

/** The text of every `<text>` at one font size. */
function texts(svg: string, size: number): string[] {
  return [...svg.matchAll(new RegExp(`<text [^>]*font-size="${size}"[^>]*>([^<]*)</text>`, "g"))].map(
    (match) => match[1]!,
  );
}

describe("shelving letters", () => {
  it("lets consecutive short letters share a shelf, in order", () => {
    expect(shelveLetters([3, 2, 2, 2])).toEqual([[0, 1], [2, 3]]);
    expect(shelveLetters([1, 1, 1, 1, 1, 1, 1])).toEqual([[0, 1, 2, 3, 4, 5], [6]]);
  });

  it("gives a letter that fills a shelf a shelf of its own", () => {
    expect(shelveLetters([6, 3])).toEqual([[0], [1]]);
    expect(shelveLetters([2, 9, 1])).toEqual([[0], [1], [2]]);
    expect(shelveLetters([2, 2], 3)).toEqual([[0], [1]]);
  });
});

describe("naming letters by number", () => {
  it("reads as English", () => {
    expect(letterNumbers([2])).toBe("Letter 2");
    expect(letterNumbers([1, 3])).toBe("Letters 1 and 3");
    expect(letterNumbers([1, 2, 4])).toBe("Letters 1, 2 and 4");
    expect(letterNumbers([1, 3], "Part")).toBe("Parts 1 and 3");
  });
});

describe("a strip that holds a vowel sign", () => {
  // Tamil மேசை is drawn ே, ம, ை, ச: a sign is not a letter, and the groups are
  // in WRITTEN order, so they are called parts.
  const parts = [
    entry("ே", { source: { citation: "Pen traces", url: "https://example.org/traces" } }),
    entry("ம"),
    entry("ை", { source: { citation: "Pen traces", url: "https://example.org/traces" } }),
    entry("ச"),
  ];

  it("is a strip of parts once any group is a sign, and of letters otherwise", () => {
    expect(sequenceUnit(parts)).toBe("Part");
    expect(sequenceUnit([entry("வ"), entry("க")])).toBe("Letter");
  });

  it("labels, heads, credits and describes its groups as parts in written order", () => {
    const figure = renderScriptSequenceFilmstripFigure("TA-W26", "மேசை", parts);
    expect(texts(figure.svg, 15)).toEqual(["How it is written — 4 parts, one after another"]);
    expect(texts(figure.svg, 12)).toEqual([
      "Part 1 of 4 — 2 strokes",
      "Part 2 of 4 — 2 strokes",
      "Part 3 of 4 — 2 strokes",
      "Part 4 of 4 — 2 strokes",
    ]);
    expect(figure.svg).toContain("Parts 1 and 3: stroke order after Pen traces");
    expect(figure.svg).toContain("Parts 2 and 4: stroke order after A cited primer");
    expect(figure.svg).toContain('aria-label="How to write மேசை: 4 parts, ே, ம, ை, ச, one after another"');
    expect(figure.svg).toMatch(
      /<desc>4 parts written one after another: ே, ம, ை, ச \(tamil\)\. The parts are in the order the hand writes them.*a vowel sign written to the left of its consonant comes before it\. Each vowel sign is drawn on its own, without the consonant it attaches to\. Each part has its own group of frames/,
    );
    expect(figure.svg).not.toMatch(/Letter/);
  });
});

describe("the printed sequence strip", () => {
  it("prints each letter's own frames as a labelled group, side by side when they fit", () => {
    const figure = renderScriptSequenceFilmstripFigure("TA-W1", "வ, க", [entry("வ"), entry("க")]);
    // Two letters of two frames share one shelf: 16 + 310 + 26 + 310 + 16.
    expect(figure.svg).toMatch(/^<svg xmlns="http:\/\/www.w3.org\/2000\/svg" width="678" /);
    const at = panels(figure.svg);
    expect(at.map(([x]) => x)).toEqual([16, 176, 352, 512]);
    expect(new Set(at.map(([, y]) => y)).size).toBe(1);
    expect(texts(figure.svg, 15)).toEqual(["How it is written — 2 letters, one after another"]);
    expect(texts(figure.svg, 12)).toEqual(["Letter 1 of 2 — 2 strokes", "Letter 2 of 2 — 2 strokes"]);
    // The second label starts over the second group, not the first.
    expect(figure.svg).toContain('<text x="352" y="54" ');
    expect(figure.labels).toEqual(["வ first", "வ second", "க first", "க second"]);
  });

  it("names the letters only where no font is needed", () => {
    const svg = renderScriptSequenceFilmstripFigure("TA-W1", "வ, க", [entry("வ"), entry("க")]).svg;
    expect(svg).toContain('aria-label="How to write வ, க: 2 letters, வ, க, one after another"');
    expect(svg).toContain("<title>Writing வ, க</title>");
    expect(svg).toContain("does not show how large the letters are next to each other");
    for (const line of [...texts(svg, 15), ...texts(svg, 12)]) {
      expect(line).not.toMatch(/[வக]/);
    }
  });

  it("stacks shelves when the letters do not fit across, and wraps a long letter inside its own", () => {
    const long = Array.from({ length: 8 }, (_, index) => ({
      number: index + 1,
      label: `movement ${index + 1}`,
      startsAfterLift: false,
      markup: `<path d="M${index} 0"/>`,
    }));
    const svg = renderScriptSequenceFilmstripFigure("X", "வக", [entry("வ", { frames: long }), entry("க")]).svg;
    const at = panels(svg);
    expect(at).toHaveLength(10);
    // Letter 1's seventh frame wraps back to the first column of ITS shelf.
    expect(at[6]![0]).toBe(16);
    expect(at[6]![1]).toBeGreaterThan(at[0]![1]);
    // Letter 2 starts a new shelf below both of letter 1's rows.
    expect(at[8]![0]).toBe(16);
    expect(at[8]![1]).toBeGreaterThan(at[6]![1]);
    // Six columns wide: 16 + 6*150 + 5*10 + 16.
    expect(svg).toMatch(/^<svg [^>]*width="982" /);
  });

  it("prints one citation line when every letter shares a source", () => {
    const svg = renderScriptSequenceFilmstripFigure("X", "வ, க", [entry("வ"), entry("க")]).svg;
    expect(texts(svg, 10)).toEqual(["Stroke order after A cited primer"]);
  });

  it("credits each source once, naming the letters it covers", () => {
    const other = { citation: "Another primer", url: "https://example.org/other" };
    const svg = renderScriptSequenceFilmstripFigure("X", "வ, க, ச", [
      entry("வ"),
      entry("க", { source: other }),
      entry("ச"),
    ]).svg;
    const lines = texts(svg, 10).join(" ");
    expect(lines).toContain("Letters 1 and 3: stroke order after A cited primer");
    expect(lines).toContain("Letter 2: stroke order after Another primer");
    expect(svg).toContain("க: Another primer &lt;https://example.org/other&gt;");
    expect(lines).not.toContain("attested, not standardised");
  });

  it("says once that an order is only attested, and keeps each note in the description", () => {
    const varied = {
      citation: "A cited primer",
      url: "https://example.org/primer",
      variation: "Schools differ on the loop.",
    };
    const svg = renderScriptSequenceFilmstripFigure("X", "வ, க", [entry("வ", { source: varied }), entry("க")]).svg;
    expect(texts(svg, 10).filter((line) => line.includes("attested"))).toHaveLength(1);
    expect(svg).toContain("Source notes on variation: வ: Schools differ on the loop.");
  });

  it("refuses fewer than two letters, two scripts, or any letter a single strip would refuse", () => {
    expect(() => renderScriptSequenceFilmstripFigure("X", "வ", [entry("வ")])).toThrow(/at least two letters/);
    expect(() =>
      renderScriptSequenceFilmstripFigure("X", "வક", [entry("வ"), entry("ક", { script: "gujarati" })]),
    ).toThrow(/one script/);
    expect(() =>
      renderScriptSequenceFilmstripFigure("X", "வ, க", [
        entry("வ"),
        entry("க", { source: { citation: " ", url: "https://example.org" } }),
      ]),
    ).toThrow(/uncited stroke order/);
    expect(() =>
      renderScriptSequenceFilmstripFigure("X", "வ, க", [
        entry("வ"),
        entry("க", {
          frames: [{ number: 1, label: "x", startsAfterLift: false, markup: "<script>alert(1)</script>" }],
        }),
      ]),
    ).toThrow(/disallowed tag 'script'/);
  });

  it("is a pure function of its letters, its headword and its lesson", () => {
    const letters = [entry("வ"), entry("க")];
    const first = renderScriptSequenceFilmstripFigure("X", "வ, க", letters);
    expect(renderScriptSequenceFilmstripFigure("X", "வ, க", letters)).toEqual(first);
    expect(renderScriptSequenceFilmstripFigure("X", "வ க", letters).sourceHash).not.toBe(first.sourceHash);
    expect(renderScriptSequenceFilmstripFigure("Y", "வ, க", letters).sourceHash).not.toBe(first.sourceHash);
    expect(JSON.parse(scriptSequenceFilmstripFigureSource("X", "வ, க", letters)).kind).toBe(
      "script-filmstrip-sequence",
    );
  });
});

describe("rendering a sequence target", () => {
  const lesson = { realization: { lessonId: "TA-W1" } } as unknown as ParsedLesson;
  const target: ScriptFilmstripTarget = {
    kind: "script-filmstrip",
    lessonId: "TA-W1",
    script: "tamil",
    glyph: "வ, க",
    letters: ["வ", "க"],
    output: "tamil/book/figures/TA-W1-filmstrip.svg",
  };

  it("draws every letter from the ledger", () => {
    const filmstrips = new Map([
      ["tamil:வ", entry("வ")],
      ["tamil:க", entry("க")],
    ]);
    const figure = renderFigure(target, lesson, { filmstrips });
    expect(figure.svg).toContain("<title>Writing வ, க</title>");
    expect(figure.labels).toHaveLength(4);
  });

  it("names the letter the ledger is missing", () => {
    const filmstrips = new Map([["tamil:வ", entry("வ")]]);
    expect(() => renderFigure(target, lesson, { filmstrips })).toThrow(/no filmstrip geometry for tamil:க/);
  });

  it("checks a declared sequence names two or more real letters", () => {
    expect(() => assertKnownFigureTarget(target)).not.toThrow();
    for (const letters of [["வ"], ["வ", ""], "வக" as unknown as string[], [1, 2] as unknown as string[]]) {
      expect(() => assertKnownFigureTarget({ ...target, letters }), JSON.stringify(letters)).toThrow(
        /two or more letters/,
      );
    }
  });
});
