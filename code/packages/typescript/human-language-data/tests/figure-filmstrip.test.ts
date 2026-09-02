import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { defaultCurriculumRoot } from "../src/loader.js";
import {
  assertFilmstripEntry,
  assertFiniteViewBox,
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

    // Each frame is a NESTED VIEWPORT, so it clips to its panel whatever its own
    // transforms say. 150 output units for a 200-unit box makes the panel
    // 150 x 225; the first sits at the margin and the second one gap further on.
    expect(figure.svg).toContain(
      '<svg x="16" y="42" width="150" height="225" ' +
        'viewBox="-10 -100 200 300" preserveAspectRatio="xMidYMid meet" overflow="hidden">',
    );
    expect(figure.svg).toContain('<svg x="176" y="42" width="150" height="225" ');

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
    const panels = [...svg.matchAll(/<svg x="(-?[\d.]+)" y="(-?[\d.]+)" width="150"/g)].map(
      (match) => [Number(match[1]), Number(match[2])] as const,
    );
    expect(panels).toHaveLength(9);
    expect(panels[6][0]).toBe(panels[0][0]);
    expect(panels[6][1]).toBeGreaterThan(panels[0][1]);
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

  it("refuses a viewBox that is not four finite numbers", () => {
    // These four are the only ledger values that reach an attribute without
    // going through `escapeXml`, because they are supposed to be numbers. The
    // ledger is JSON, so "supposed to be" has to be checked: a string here
    // would close `viewBox="..."` and open whatever came next.
    for (const hostile of [
      '-41" onload="alert(document.domain)" data-x="',
      '-41"/><script>alert(1)</script><svg viewBox="-41',
      '-41"/><text>Stroke order after A. Ttacker</text><svg viewBox="-41',
      Number.NaN,
      Number.POSITIVE_INFINITY,
      null,
      undefined,
    ]) {
      const hostileEntry = entry();
      (hostileEntry.viewBox as unknown as Record<string, unknown>).minX = hostile;
      expect(() => renderScriptFilmstripFigure("X", hostileEntry)).toThrow(
        /viewBox minX is not a finite number/,
      );
    }
    expect(assertFiniteViewBox({ minX: -1, minY: -2, width: 3, height: 4 }, "f")).toEqual({
      minX: -1,
      minY: -2,
      width: 3,
      height: 4,
    });
  });

  it("proves an entry has the shape its type claims", () => {
    // `readLedgerFile<T>` parses JSON and casts; the cast is a promise to the
    // compiler, not a check at runtime.
    expect(() => assertFilmstripEntry(entry(), "f")).not.toThrow();
    const cases: Array<[string, (value: Record<string, unknown>) => void]> = [
      ["glyph is not a string", (e) => (e.glyph = 7)],
      ["source.url is not a string", (e) => (e.source = { citation: "c", url: 7 })],
      ["penLifts is not a whole number", (e) => (e.penLifts = "one")],
      ["frames is not a list", (e) => (e.frames = "none")],
      [
        "frame 1 markup is not a string",
        (e) => ((e.frames as Array<Record<string, unknown>>)[0].markup = 7),
      ],
      [
        "frame 1 lift flag is not a boolean",
        (e) => ((e.frames as Array<Record<string, unknown>>)[0].startsAfterLift = "yes"),
      ],
    ];
    for (const [message, break_] of cases) {
      const broken = entry() as unknown as Record<string, unknown>;
      break_(broken);
      expect(() => assertFilmstripEntry(broken as unknown as FilmstripEntry, "f")).toThrow(
        message,
      );
    }
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
      "<script>alert(1)</script>",
      '<image href="x"/>',
      '<g onload="alert(1)"><path d="M0 0"/></g>',
      "<foreignObject><div/></foreignObject>",
      '<path d="M0 0"/> < not a tag',
      "<!-- a comment -->",
      "<![CDATA[<script>alert(1)</script>]]>",
      "<?xml-stylesheet href='x'?>",
      "<path d='M0 0'/>",
      "<path d=M00/>",
    ]) {
      expect(() => assertSafeFilmstripMarkup(hostile, "fixture")).toThrow();
    }
  });

  it("refuses an attribute the ductus renderer does not emit", () => {
    // Inert today only because the five allowed tags are inert. The allowlist is
    // what keeps that true the day somebody adds `use` or `image`.
    for (const hostile of [
      '<g xlink:href="javascript:alert(1)"/>',
      '<text style="background-image:url(http://evil.example/x)">hi</text>',
      '<circle clip-path="url(http://evil.example/#c)"/>',
      '<path filter="url(http://evil.example/x#f)" d="M0 0"/>',
    ]) {
      expect(() => assertSafeFilmstripMarkup(hostile, "fixture")).toThrow(
        /disallowed attribute/,
      );
    }
  });

  it("refuses a fragment that would escape the panel it is placed in", () => {
    // Each fragment is embedded inside a `<g transform="...">` that positions
    // it. A fragment beginning `</g>` closes that wrapper, and everything after
    // it — allowlisted tags and all — is drawn loose on the figure, including a
    // `<text>` indistinguishable from the real citation line.
    expect(() =>
      assertSafeFilmstripMarkup(
        '</g><text x="0" y="0">Stroke order after a source nobody cited</text><g>',
        "fixture",
      ),
    ).toThrow(/closes 'g' that is not open/);
    expect(() => assertSafeFilmstripMarkup("<g><g></g>", "fixture")).toThrow(
      /leaves 'g' open/,
    );
    expect(() => assertSafeFilmstripMarkup("<g></path></g>", "fixture")).toThrow(
      /closes 'path' that is not open/,
    );
    expect(() => assertSafeFilmstripMarkup('</g class="x">', "fixture")).toThrow(
      /malformed closing/,
    );
  });

  it("refuses an attribute VALUE no serialiser could have written", () => {
    // A NUL or a bare `&` inside `d="..."` is not an injection — it fails closed
    // later, in `rsvg-convert`, with a message that names neither the letter nor
    // the frame. Naming it here is the whole point.
    expect(() => assertSafeFilmstripMarkup('<path d="M0 &#48;"/>', "f")).not.toThrow();
    expect(() => assertSafeFilmstripMarkup('<path d="M0 &0"/>', "f")).toThrow(
      /'d' has an unknown entity/,
    );
    expect(() => assertSafeFilmstripMarkup('<path d="M0\u00000"/>', "f")).toThrow(
      /'d' has a control character/,
    );
    // Paint is the one value shape that can name an external resource.
    expect(() => assertSafeFilmstripMarkup('<path fill="#e0ddd6" d="M0 0"/>', "f")).not.toThrow();
    expect(() => assertSafeFilmstripMarkup('<path fill="none" d="M0 0"/>', "f")).not.toThrow();
    expect(() =>
      assertSafeFilmstripMarkup('<path fill="url(http://evil.example/x#p)" d="M0 0"/>', "f"),
    ).toThrow(/'fill' is not a plain colour/);
    expect(() =>
      assertSafeFilmstripMarkup('<circle stroke="url(file:///etc/passwd#p)" r="1"/>', "f"),
    ).toThrow(/'stroke' is not a plain colour/);
  });

  it("holds XML's own case sensitivity", () => {
    // `<G></g>` is a mismatch every renderer rejects; catching it here names the
    // frame instead of failing later in the book's SVG-to-PDF step.
    expect(() => assertSafeFilmstripMarkup('<G class="x"></g>', "f")).toThrow(
      /closes 'g' that is not open/,
    );
    expect(() => assertSafeFilmstripMarkup("<SCRIPT></SCRIPT>", "f")).toThrow(
      /disallowed tag 'script'/,
    );
  });

  it("refuses text no serialiser could have written", () => {
    // `escapeXml` turns `&` into `&amp;`, so a bare or unknown reference means
    // the fragment did not come from it — and would break the book's own
    // SVG-to-PDF step later, where the message would name nothing useful.
    expect(() => assertSafeFilmstripMarkup("<text>Bell &amp; Co</text>", "fixture")).not.toThrow();
    expect(() => assertSafeFilmstripMarkup("<text>Bell &#38; Co</text>", "fixture")).not.toThrow();
    expect(() => assertSafeFilmstripMarkup("<text>Bell & Co</text>", "fixture")).toThrow(
      /unescaped or unknown entity/,
    );
    expect(() => assertSafeFilmstripMarkup("<text>&xxe;</text>", "fixture")).toThrow(
      /unescaped or unknown entity/,
    );
    expect(() =>
      assertSafeFilmstripMarkup("<text>a\u0000b</text>", "fixture"),
    ).toThrow(/control character/);
  });

  it("scans a large hostile fragment in linear time", () => {
    // A quadratic checker here would be a denial of service on the build, since
    // the input is a file on disk. 3 MB of unterminated attribute must not stall.
    const started = Date.now();
    expect(() =>
      assertSafeFilmstripMarkup(`<g ${'class="'.repeat(200_000)}`, "fixture"),
    ).toThrow();
    expect(Date.now() - started).toBeLessThan(5_000);
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
