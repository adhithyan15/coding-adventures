import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { defaultCurriculumRoot } from "../src/loader.js";
import {
  assertSafeFilmstripMarkup,
  indexFilmstripLedger,
  renderScriptFilmstripFigure,
  wrapFigureText,
  FILMSTRIP_LEDGER_PATH,
  type FilmstripEntry,
  type FilmstripLedger,
} from "../src/figure-filmstrip.js";

/**
 * A deliberately tiny letter: two frames, one stroke each, so the arithmetic
 * being checked is visible in the fixture rather than buried in a real glyph's
 * several-hundred-point outline.
 */
function entry(overrides: Partial<FilmstripEntry> = {}): FilmstripEntry {
  return {
    script: "tamil",
    glyph: "அ",
    font: "_fonts/NotoSansTamil-Static.ttf",
    source: {
      citation: "Sankaran Radhakrishnan, Tamil Script Learners Manual, p. 192",
      url: "https://example.org/tamil-manual",
    },
    penLifts: 1,
    summary: "2 strokes · 1 pen lift · 2 movements",
    viewBox: { minX: -10, minY: -100, width: 200, height: 300 },
    frames: [
      {
        number: 1,
        label: "curl around the upper loop",
        startsAfterLift: false,
        markup: '<g transform="scale(1,-1)"><path class="ductus__pen" d="M0 0L10 10"/></g>',
      },
      {
        number: 2,
        label: "draw the right upright down",
        startsAfterLift: true,
        markup: '<g transform="scale(1,-1)"><circle cx="5" cy="5" r="3"/></g>',
      },
    ],
    ...overrides,
  };
}

describe("the printed filmstrip", () => {
  it("lays every frame out in the letter's one shared box", () => {
    const figure = renderScriptFilmstripFigure("TA-S119-letter-a", entry());

    // Two panels, each carrying the frame it was given.
    expect(figure.svg.match(/<rect [^>]*rx="6"/g)).toHaveLength(2);
    expect(figure.svg).toContain('d="M0 0L10 10"');
    expect(figure.svg).toContain('<circle cx="5" cy="5" r="3"/>');

    // 150 output units wide for a 200-unit box is a scale of 0.75, and the box
    // starts at (-10, -100), so the first panel sits at x = 16 (the margin) and
    // its contents shift by 16 - 0.75 * -10 = 23.5 and 42 - 0.75 * -100 = 117.
    expect(figure.svg).toContain('transform="translate(23.5 117) scale(0.75)"');
    // The second panel is one frame plus one gap further right: 23.5 + 160.
    expect(figure.svg).toContain('transform="translate(183.5 117) scale(0.75)"');

    expect(figure.labels).toEqual([
      "curl around the upper loop",
      "draw the right upright down",
    ]);
    expect(figure.sourceHash).toMatch(/^fnv1a64:/);
    expect(figure.svgHash).toMatch(/^fnv1a64:/);
  });

  it("wraps a long letter onto a second row instead of off the page", () => {
    const frames = Array.from({ length: 9 }, (_, index) => ({
      number: index + 1,
      label: `movement ${index + 1}`,
      startsAfterLift: false,
      markup: `<path d="M${index} 0"/>`,
    }));
    const svg = renderScriptFilmstripFigure("X", entry({ frames })).svg;
    const width = Number(/ width="(\d+(?:\.\d+)?)"/.exec(svg)?.[1]);
    // Six columns, not nine: 16 + 6*150 + 5*10 + 16.
    expect(width).toBe(982);
    // The seventh frame starts a new row, so its x returns to the first column.
    const shifts = [...svg.matchAll(/translate\((-?[\d.]+) (-?[\d.]+)\) scale/g)].map(
      (match) => [Number(match[1]), Number(match[2])] as const,
    );
    expect(shifts[6][0]).toBe(shifts[0][0]);
    expect(shifts[6][1]).toBeGreaterThan(shifts[0][1]);
  });

  it("prints the citation and says plainly when the order is only attested", () => {
    const plain = renderScriptFilmstripFigure("X", entry()).svg;
    expect(plain).toContain("Stroke order after Sankaran Radhakrishnan");
    expect(plain).not.toContain("attested, not standardised");

    const varied = renderScriptFilmstripFigure(
      "X",
      entry({
        source: {
          citation: "A cited primer",
          url: "https://example.org/primer",
          variation: "Schools differ on whether the loop precedes the bar.",
        },
      }),
    ).svg;
    expect(varied).toContain("attested, not standardised");
    // The long note itself stays in the description rather than under the art.
    expect(varied).toContain("Schools differ on whether the loop precedes the bar.");
    expect(varied.indexOf("Schools differ")).toBeLessThan(varied.indexOf("<rect"));
  });

  it("refuses to draw a letter whose stroke order is not cited", () => {
    expect(() =>
      renderScriptFilmstripFigure(
        "X",
        entry({ source: { citation: "  ", url: "https://example.org/x" } }),
      ),
    ).toThrow(/uncited stroke order/);
    expect(() => renderScriptFilmstripFigure("X", entry({ frames: [] }))).toThrow(
      /no frames/,
    );
    expect(() =>
      renderScriptFilmstripFigure("X", entry({ viewBox: { minX: 0, minY: 0, width: 0, height: 4 } })),
    ).toThrow(/empty viewBox/);
  });

  it("is a pure function of the ledger entry", () => {
    const first = renderScriptFilmstripFigure("TA-S119-letter-a", entry());
    const second = renderScriptFilmstripFigure("TA-S119-letter-a", entry());
    expect(first.svg).toBe(second.svg);
    expect(first.svgHash).toBe(second.svgHash);
    // A different lesson is a different figure even for the same letter.
    expect(renderScriptFilmstripFigure("OTHER", entry()).sourceHash).not.toBe(
      first.sourceHash,
    );
  });
});

describe("the fragment allowlist", () => {
  it("accepts what the ductus renderer actually emits", () => {
    expect(() =>
      assertSafeFilmstripMarkup(
        '<g transform="scale(1,-1)"><path class="a" d="M0 0"/><circle r="2"/></g>' +
          '<text x="1" y="2">1. curl &amp; sweep <tspan x="1">on</tspan></text>',
        "fixture",
      ),
    ).not.toThrow();
  });

  it("refuses a tag, an attribute or a bracket it was not promised", () => {
    for (const hostile of [
      '<script>alert(1)</script>',
      '<image href="x"/>',
      '<g onload="alert(1)"><path d="M0 0"/></g>',
      '<foreignObject><div/></foreignObject>',
      '<path d="M0 0"/> < not a tag',
    ]) {
      expect(() => assertSafeFilmstripMarkup(hostile, "fixture")).toThrow();
    }
  });
});

describe("the committed ledger", () => {
  const ledger = JSON.parse(
    readFileSync(join(defaultCurriculumRoot(), FILMSTRIP_LEDGER_PATH), "utf8"),
  ) as FilmstripLedger;

  it("is a version-1 file with unique, citable entries", () => {
    const index = indexFilmstripLedger(ledger);
    expect(index.size).toBe(ledger.entries.length);
    expect(index.size).toBeGreaterThan(0);
    for (const value of index.values()) {
      expect(value.source.url).toMatch(/^https?:\/\//);
      expect(value.font).toMatch(/^_fonts\//);
      expect(value.frames.length).toBeGreaterThan(0);
    }
  });

  it("renders every committed entry without tripping the allowlist", () => {
    for (const value of ledger.entries) {
      const figure = renderScriptFilmstripFigure(`${value.script}:${value.glyph}`, value);
      expect(figure.svg.startsWith("<svg ")).toBe(true);
      expect(figure.svg.endsWith("</svg>\n")).toBe(true);
      expect(figure.labels).toHaveLength(value.frames.length);
    }
  });

  it("rejects a ledger that holds one letter twice", () => {
    expect(() =>
      indexFilmstripLedger({ ...ledger, entries: [entry(), entry()] }),
    ).toThrow(/duplicate entry/);
    expect(() =>
      indexFilmstripLedger({ ...ledger, version: 2 as unknown as 1 }),
    ).toThrow(/version 1/);
  });
});

describe("figure text wrapping", () => {
  it("breaks on words and never drops one", () => {
    const lines = wrapFigureText("a bb ccc dddd eeeee ffffff", 60, 10);
    expect(lines.join(" ")).toBe("a bb ccc dddd eeeee ffffff");
    expect(lines.length).toBeGreaterThan(1);
  });

  it("gives an over-long word its own line rather than cutting it", () => {
    expect(wrapFigureText("supercalifragilisticexpialidocious", 40, 10)).toEqual([
      "supercalifragilisticexpialidocious",
    ]);
  });

  it("answers with one empty line for empty text", () => {
    expect(wrapFigureText("   ", 100, 10)).toEqual([""]);
  });
});
