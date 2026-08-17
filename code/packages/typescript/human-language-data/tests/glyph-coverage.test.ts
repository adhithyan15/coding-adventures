import { describe, expect, it } from "vitest";
import {
  measureGlyphCoverage,
  renderGlyphCoverage,
  scriptWrappers,
  mappedCharacters,
  type BookFonts,
} from "../src/glyph-coverage.js";
import { loadBookFonts, loadMainFontCharset } from "../src/loader.js";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const CHARSET = new Set(["ā", "ī", "ô"]);

function book(over: Partial<BookFonts> = {}): BookFonts {
  return {
    language: "test",
    preamble: [
      "\\setmainfont{Latin Modern Roman}",
      "\\newfontfamily\\bengalifont[Path=../]{NotoSansBengali-Static.ttf}",
      "\\newcommand{\\bn}[1]{{\\bengalifont #1}}",
    ].join("\n"),
    files: [],
    scriptFonts: { "NotoSansBengali-Static.ttf": new Set(["ন".codePointAt(0)!, "ক".codePointAt(0)!]) },
    ...over,
  };
}

describe("glyph coverage", () => {
  it("reads BOTH preamble wrapper forms", () => {
    // Reading only the brace form is what made Sanskrit's `\sk` invisible to an
    // earlier probe, which then reported 14,563 missing characters on a corpus
    // that compiles clean.
    const preamble = [
      "\\newfontfamily\\afont{A.ttf}",
      "\\newfontfamily\\bfont{B.ttf}",
      "\\newcommand{\\aa}[1]{{\\afont #1}}",
      "\\newcommand{\\bb}[1]{\\textb{#1}}",
    ].join("\n");
    expect([...scriptWrappers(preamble)]).toEqual([
      ["aa", "A.ttf"],
      ["bb", "B.ttf"],
    ]);
  });

  it("reads the newunicodechar escape hatch", () => {
    expect([...mappedCharacters("\\newunicodechar{ṉ}{\\b{n}}\n\\newunicodechar{ṁ}{\\.{m}}")]).toEqual(["ṉ", "ṁ"]);
  });

  it("catches HL-C214: ae-with-macron in main-font text", () => {
    const report = measureGlyphCoverage(
      [book({ files: [{ path: "latin/book/chapters/ch47.tex", text: "Old English had \\emph{ǣg}." }] })],
      CHARSET,
    );
    expect(report.gaps.map((g) => `${g.codepoint} ${g.layer}`)).toEqual(["U+01E3 main"]);
  });

  it("catches HL-C223: open-o in a romanization", () => {
    const report = measureGlyphCoverage(
      [book({ files: [{ path: "bengali/book/chapters/ch16.tex", text: "It is \\textbf{nɔ}, not na." }] })],
      CHARSET,
    );
    expect(report.gaps.map((g) => g.codepoint)).toEqual(["U+0254"]);
  });

  it("catches a character in the WRONG script font", () => {
    // The layer no earlier probe could see: Devanagari inside a Bengali wrapper
    // renders as tofu, and the main-font check would never look at it.
    const report = measureGlyphCoverage(
      [book({ files: [{ path: "x.tex", text: "a \\bn{क} b" }] })],
      CHARSET,
    );
    expect(report.gaps[0]).toMatchObject({ layer: "script", codepoint: "U+0915", font: "NotoSansBengali-Static.ttf" });
  });

  it("accepts a character the preamble re-renders with newunicodechar", () => {
    const report = measureGlyphCoverage(
      [
        book({
          preamble: "\\newunicodechar{ṉ}{\\b{n}}",
          files: [{ path: "x.tex", text: "the alveolar ṉ" }],
        }),
      ],
      CHARSET,
    );
    expect(report.gaps).toEqual([]);
  });

  it("does not carry one book's mapping into another", () => {
    // A `\newunicodechar` in Tamil's preamble does nothing for Bengali's book.
    const report = measureGlyphCoverage(
      [
        book({ language: "tamil", preamble: "\\newunicodechar{ṉ}{\\b{n}}", files: [{ path: "t.tex", text: "ṉ" }] }),
        book({ language: "bengali", preamble: "", files: [{ path: "b.tex", text: "ṉ" }] }),
      ],
      CHARSET,
    );
    expect(report.gaps.map((g) => g.language)).toEqual(["bengali"]);
  });

  it("strips a wrapper nested inside another command", () => {
    const report = measureGlyphCoverage(
      [book({ files: [{ path: "x.tex", text: "\\textbf{\\bn{ন}} and \\bn{\\textbf{ক}}" }] })],
      CHARSET,
    );
    expect(report.gaps).toEqual([]);
  });

  it("treats an unresolvable script font as unmeasured, never clean", () => {
    // Skipping quietly is how a gate reports success for work it did not do —
    // but reporting a gap would be worse, so the contract is: no font, no claim.
    const report = measureGlyphCoverage(
      [book({ scriptFonts: {}, files: [{ path: "x.tex", text: "\\bn{ক}" }] })],
      CHARSET,
    );
    expect(report.gaps).toEqual([]);
    expect(report.summary.scriptCharacters).toBe(0);
  });

  it("stays silent on characters that are covered", () => {
    const report = measureGlyphCoverage(
      [book({ files: [{ path: "x.tex", text: "nām, \\bn{ন}, ô" }] })],
      CHARSET,
    );
    expect(report.gaps).toEqual([]);
  });

  it("renders a clean run as a positive statement", () => {
    const text = renderGlyphCoverage(measureGlyphCoverage([book()], CHARSET)).join("\n");
    expect(text).toContain("every character renders");
  });

  it("THE GATE: every character in every generated book renders", () => {
    const books = loadBookFonts();
    const charset = loadMainFontCharset();
    expect(books.length).toBeGreaterThan(0);
    expect(charset.size).toBeGreaterThan(0);
    const report = measureGlyphCoverage(books, charset);
    // Named, not counted: a bare number tells whoever breaks this nothing about
    // which character to change.
    expect(report.gaps.map((g) => `${g.file} ${g.codepoint} '${g.char}' (${g.layer})`)).toEqual([]);
    // Guard the vacuous case in both directions.
    expect(report.summary.filesScanned).toBeGreaterThan(100);
    expect(report.summary.mainCharacters).toBeGreaterThan(50);
    expect(report.summary.scriptCharacters).toBeGreaterThan(100);
  }, 60_000);

  it("proves the corpus gate is not vacuous", () => {
    // Same measurement, same corpus, one planted character — the real failure
    // from HL-C223, in the track it actually happened in.
    const books = loadBookFonts();
    const planted = books.map((b) =>
      b.language === "bengali" ? { ...b, files: [...b.files, { path: "bengali/planted.tex", text: "nɔ" }] } : b,
    );
    const report = measureGlyphCoverage(planted, loadMainFontCharset());
    expect(report.gaps.map((g) => g.codepoint)).toEqual(["U+0254"]);
  }, 60_000);
});

describe("the font parser, against malformed input", () => {
  // `readFontCoverage` is module-private but reachable through `loadBookFonts`,
  // and `report-cli` takes a `--root`, so it is not limited to vendored fonts.
  function font(build: (b: Buffer, table: number) => void, size = 160): Buffer {
    const b = Buffer.alloc(size);
    b.writeUInt32BE(0x00010000, 0);
    b.writeUInt16BE(1, 4);
    b.write("cmap", 12, "ascii");
    b.writeUInt32BE(32, 20);
    b.writeUInt16BE(0, 32);
    b.writeUInt16BE(1, 34);
    b.writeUInt16BE(3, 36);
    b.writeUInt16BE(1, 38);
    b.writeUInt32BE(16, 40);
    build(b, 48);
    return b;
  }

  function coverage(buffer: Buffer): Set<number> {
    const dir = mkdtempSync(join(tmpdir(), "hl-font-"));
    try {
      mkdirSync(join(dir, "_fonts"), { recursive: true });
      mkdirSync(join(dir, "t", "book"), { recursive: true });
      writeFileSync(join(dir, "_fonts", "Evil.ttf"), buffer);
      writeFileSync(
        join(dir, "t", "book", "preamble.tex"),
        "\\newfontfamily\\tfont{Evil.ttf}\n\\newcommand{\\tt}[1]{{\\tfont #1}}",
      );
      writeFileSync(join(dir, "t", "book", "ch01.tex"), "x");
      const book = loadBookFonts(dir)[0]!;
      return (book.scriptFonts["Evil.ttf"] as Set<number> | undefined) ?? new Set();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }

  it("clamps a format-12 group to the real Unicode ceiling", () => {
    // Unclamped, one 12-byte record claiming [0, 0xFFFFFFFF] drove 4.29 BILLION
    // Set.add calls: 548MB RSS and a hard V8 abort under a 256MB heap, from a
    // 68-byte file.
    const started = Date.now();
    const cover = coverage(
      font((b, t) => {
        b.writeUInt16BE(12, t);
        b.writeUInt32BE(1, t + 12);
        b.writeUInt32BE(0, t + 16);
        b.writeUInt32BE(0xffffffff, t + 20);
      }),
    );
    expect(cover.size).toBeLessThanOrEqual(0x110000);
    expect(Date.now() - started).toBeLessThan(5_000);
  });

  it("bounds format 4 by WORK DONE, not by set size", () => {
    // Budgeting on `out.size` looks equivalent and is not: format 4 caps at
    // 65,536 codepoints, so thousands of overlapping full-range segments never
    // grow the set past that ceiling and loop anyway — measured 5,430ms.
    const segs = 4000;
    const segX2 = segs * 2;
    const size = 48 + 16 + segX2 * 3 + 16;
    const buffer = font((b, t) => {
      b.writeUInt16BE(4, t);
      b.writeUInt16BE(segX2, t + 6);
      for (let i = 0; i < segs; i += 1) {
        b.writeUInt16BE(0xfffe, t + 14 + i * 2);
        b.writeUInt16BE(0x0000, t + 14 + segX2 + 2 + i * 2);
      }
    }, size);
    const started = Date.now();
    coverage(buffer);
    expect(Date.now() - started).toBeLessThan(2_000);
  });

  it("survives every truncated or out-of-range offset without throwing", () => {
    const cases: Buffer[] = [
      Buffer.alloc(0),
      Buffer.alloc(5),
      font((b) => b.writeUInt32BE(0xffffff, 20)),
      font((b) => b.writeUInt32BE(0xffffff, 40)),
      font((b) => b.writeUInt16BE(0xffff, 34)),
    ];
    for (const buffer of cases) expect(() => coverage(buffer)).not.toThrow();
  });

  it("leaves an unparseable font OUT of the map, so it is unmeasured not clean", () => {
    // An empty cmap is truthy as a Set. Putting it in the map would report every
    // character in that script as a gap; leaving it out is honest.
    expect(coverage(Buffer.alloc(160)).size).toBe(0);
  });
});

describe("hostile preamble input", () => {
  it("does not resolve a font name through the prototype chain", () => {
    // `run.font` comes from an unrestricted capture. A font named `constructor`
    // resolved to `Object` — truthy — sailing past the unmeasured guard and
    // throwing on `.has`.
    for (const name of ["constructor", "__proto__", "toString", "hasOwnProperty"]) {
      const book: BookFonts = {
        language: "x",
        preamble: `\\newfontfamily\\xfont{${name}}\n\\newcommand{\\xx}[1]{{\\xfont #1}}`,
        files: [{ path: "a.tex", text: "\\xx{\u0995}" }],
        scriptFonts: Object.create(null) as Record<string, Set<number>>,
      };
      expect(() => measureGlyphCoverage([book], new Set())).not.toThrow();
    }
  });

  it("matches a font family declaration in linear time", () => {
    // Two nullable `\s*` around an optional group split whitespace ambiguously:
    // 2,530ms at 64k spaces.
    const started = Date.now();
    scriptWrappers("\\newfontfamily\\af" + " ".repeat(64_000) + "{A.ttf}");
    expect(Date.now() - started).toBeLessThan(500);
  });

  it("is linear on many UNTERMINATED declaration heads", () => {
    // CodeQL js/polynomial-redos, and a DIFFERENT ambiguity from the one above:
    // `[^\]]*` inside `\[...\]` rescanned to end-of-input from every
    // unterminated `[`, so N heads cost O(N^2). Fixing the whitespace split left
    // this standing. Both shapes CodeQL reported are covered.
    for (const unit of ["\\newfontfamily\\a[", "\\newfontfamily\\a{{"]) {
      const started = Date.now();
      scriptWrappers(unit.repeat(32_000));
      expect(Date.now() - started).toBeLessThan(500);
    }
  });

  it("still reads a real multi-script preamble", () => {
    // A scan replaced the regex, so pin that it gives the same answer on the most
    // complex preamble in the corpus, not only on fixtures.
    const kannada = loadBookFonts().find((book) => book.language === "kannada")!;
    expect([...scriptWrappers(kannada.preamble)]).toEqual([
      ["kn", "NotoSansKannada-Static.ttf"],
      ["ta", "NotoSansTamil-Static.ttf"],
      ["te", "NotoSansTelugu-Static.ttf"],
      ["ml", "NotoSansMalayalam-Static.ttf"],
      ["dv", "NotoSansDevanagari-Static.ttf"],
      ["ar", "NotoNaskhArabic-Static.ttf"],
    ]);
  }, 60_000);

  it("caps rewrite passes on deeply nested wrappers", () => {
    const depth = 8_000;
    const book: BookFonts = {
      language: "x",
      preamble: "\\newfontfamily\\xfont{A.ttf}\n\\newcommand{\\xx}[1]{{\\xfont #1}}",
      files: [{ path: "n.tex", text: "\\xx{".repeat(depth) + "x" + "}".repeat(depth) }],
      scriptFonts: {},
    };
    const started = Date.now();
    measureGlyphCoverage([book], new Set());
    expect(Date.now() - started).toBeLessThan(2_000);
  });
});
