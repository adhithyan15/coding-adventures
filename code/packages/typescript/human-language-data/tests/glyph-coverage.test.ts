import { describe, expect, it } from "vitest";
import {
  measureGlyphCoverage,
  renderGlyphCoverage,
  scriptWrappers,
  mappedCharacters,
  type BookFonts,
} from "../src/glyph-coverage.js";
import { loadBookFonts, loadMainFontCharset } from "../src/loader.js";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
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
    //
    // THE BOUND IS 2s, NOT 500ms, AND THE GAP IS THE POINT. Measured on this
    // input, the fixed scanner takes 1ms median and 3ms worst of seven runs --
    // so 500ms was never measuring the scanner, it was measuring whether the
    // process got descheduled. It duly failed at 530ms locally and 578ms on a
    // macOS runner, on changes that touched only lesson markdown.
    //
    // 2s still separates the two cases decisively: healthy is ~1ms, the
    // ambiguous-regex regression is ~2,530ms, and a machine loaded enough to
    // stretch 1ms past 2s would have stretched 2,530ms far past it too. Do not
    // tighten this back toward the healthy time -- the headroom is protecting
    // against scheduler noise, not slack in the scanner.
    const started = Date.now();
    scriptWrappers("\\newfontfamily\\af" + " ".repeat(64_000) + "{A.ttf}");
    expect(Date.now() - started).toBeLessThan(2_000);
  });

  it("is linear on many UNTERMINATED declaration heads", () => {
    // CodeQL js/polynomial-redos, and a DIFFERENT ambiguity from the one above:
    // `[^\]]*` inside `\[...\]` rescanned to end-of-input from every
    // unterminated `[`, so N heads cost O(N^2). Fixing the whitespace split left
    // this standing. Both shapes CodeQL reported are covered.
    //
    // 2s for the same reason as the test above, but this one was the tighter of
    // the two and nobody had noticed: measured, the fixed scanner takes 196ms
    // and 207ms on these two inputs, so a 500ms bound left barely 2.4x of head-
    // room. The sibling test flaked first only because process starvation hits
    // a 1ms call more visibly than a 200ms one; this bound was closer to the
    // edge all along.
    for (const unit of ["\\newfontfamily\\a[", "\\newfontfamily\\a{{"]) {
      const started = Date.now();
      scriptWrappers(unit.repeat(32_000));
      expect(Date.now() - started).toBeLessThan(2_000);
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

// ---------------------------------------------------------------------------
// The charset file itself, which nothing checked.
//
// `loadMainFontCharset` reads one field, `char`, and throws only when the array
// is empty. Everything else about the file -- whether `cp` names the character
// beside it, whether an entry is duplicated, whether a codepoint that cannot
// render is listed -- was unchecked, on a file that is the sole authority for
// whether a character reaches the reader.
//
// It was also CURATED, and a curated allow-list fails in one direction only:
// silently, against the author. It held 123 of the 530 characters Latin Modern
// actually sets. `1.º` was unwritable in every book until the ordinal tranche
// added U+00AA and U+00BA by hand, and german/CHANGELOG.md records capital Ü
// being avoided while the font has had it all along. The file is regenerated
// from the font's cmap now, so the class is closed rather than the two instances.
//
// These tests cannot re-read the font: Latin Modern ships with TeX Live and is
// not resolvable from a plain checkout, which is why the answer is committed at
// all. What they CAN do is refuse a file that contradicts itself, and refuse the
// three kinds of entry the regeneration deliberately excluded -- which is where
// a hand-edit would land.
// ---------------------------------------------------------------------------
describe("the committed main-font charset", () => {
  const charset = loadMainFontCharset();

  interface CharsetEntry {
    cp: string;
    char: string;
    name: string;
  }
  function entries(): CharsetEntry[] {
    // Read as data rather than through the loader, which returns only `char`.
    const root = new URL("../../../../learning/human-languages/core/main-font-charset.json", import.meta.url);
    return (JSON.parse(readFileSync(root, "utf8")) as { characters: CharsetEntry[] }).characters;
  }

  it("says the same thing twice about every character, so a hand-edit cannot drift", () => {
    // `cp` and `char` are two spellings of one fact, and only `char` is read.
    // A wrong `cp` is therefore invisible at runtime and misleading to a reader,
    // which is the worst combination a ledger can have.
    for (const entry of entries()) {
      const codepoint = entry.char.codePointAt(0);
      expect([...entry.char], `${entry.cp} is more than one character`).toHaveLength(1);
      expect(`U+${codepoint!.toString(16).toUpperCase().padStart(4, "0")}`).toBe(entry.cp);
      expect(entry.name.trim().length, `${entry.cp} has no name`).toBeGreaterThan(0);
    }
  });

  it("is sorted and free of duplicates, so a regeneration is a readable diff", () => {
    const codepoints = entries().map((entry) => entry.char.codePointAt(0)!);
    expect(new Set(codepoints).size).toBe(codepoints.length);
    expect(codepoints).toEqual([...codepoints].sort((a, b) => a - b));
    expect(charset.size).toBe(codepoints.length);
  });

  it("lists NOTHING invisible, ASCII or private-use, which is where a hand-edit lands", () => {
    // The exclusions the regeneration makes, restated as a gate. Each is a way of
    // blessing something that is not a rendering claim: a Private Use codepoint
    // is a font-internal glyph slot rather than a character (Unicode calls the
    // range Co, so the category check below covers it); a C1 control is mapped
    // only for legacy TeX encodings; and U+00A0's glyph has no outline at all, so
    // "the cmap has it" and "it sets" come apart there.
    // By CATEGORY and not by range. The first draft of this test listed the four
    // things the regeneration actually excluded -- ASCII, the Private Use Area,
    // the C1 controls, U+00A0 and U+00AD -- and the security review of this
    // change pointed out that it therefore closed four instances rather than the
    // class: a hand-edit adding U+202E RIGHT-TO-LEFT OVERRIDE, U+200B, U+2028 or
    // U+FEFF would have walked straight through a gate whose stated purpose is
    // catching exactly that. `\p{C}` and `\p{Z}` are the property the exclusion
    // was always about -- renders nothing -- so they are what is asserted.
    const invisible = /^[\p{C}\p{Z}]$/u;
    const offenders: string[] = [];
    for (const entry of entries()) {
      const codepoint = entry.char.codePointAt(0)!;
      if (codepoint <= 0x7f) offenders.push(`${entry.cp} ASCII`);
      if (invisible.test(entry.char)) offenders.push(`${entry.cp} renders nothing`);
    }
    expect(offenders).toEqual([]);
    // The control, so the regex is known to discriminate rather than to be
    // trivially satisfied by every character in the file.
    for (const hostile of ["\u00A0", "\u00AD", "\u202E", "\u200B", "\u2028", "\uFEFF", "\uE000"]) {
      expect(invisible.test(hostile), `${hostile.codePointAt(0)!.toString(16)} must be caught`).toBe(true);
    }
    expect(invisible.test("\u00AA")).toBe(false);
  });

  it("keeps the six small-caps casualties INSIDE the list, and says why", () => {
    // lmromancaps10 carries 821 codepoints rather than 828: a small-caps face has
    // no lowercase long s and no lowercase f-ligatures. No book uses small caps --
    // `\textsc` and `\scshape` appear zero times across every book/ directory --
    // so the six are covered by the faces that actually set, and the file records
    // the difference rather than losing it. If small caps ever appear, this is
    // where the reader finds out the answer became face-dependent.
    const root = new URL("../../../../learning/human-languages/core/main-font-charset.json", import.meta.url);
    const parsed = JSON.parse(readFileSync(root, "utf8")) as {
      smallCapsGap: { note: string; characters: { cp: string; char: string }[] };
    };
    expect(parsed.smallCapsGap.characters.map((entry) => entry.cp)).toEqual([
      "U+017F", "U+FB00", "U+FB01", "U+FB02", "U+FB03", "U+FB04",
    ]);
    expect(parsed.smallCapsGap.note).toMatch(/NO BOOK IN THIS CORPUS USES SMALL CAPS/);
    for (const entry of parsed.smallCapsGap.characters) {
      expect(charset.has(entry.char), `${entry.cp} named as a gap but not listed`).toBe(true);
    }
  });

  it("HOLDS THE CHARACTERS THE OLD CURATION DROPPED, named rather than counted", () => {
    // The instances this file was regenerated for. A count would pass on any 530
    // characters; these are the ones a book could not write while the font could
    // set them. U+00AA and U+00BA made `1.º` impossible corpus-wide; U+00DC is
    // the capital Ü german/CHANGELOG.md records working around; U+2010 is the
    // real HYPHEN, distinct from the ASCII one; U+20AC is the euro sign.
    for (const char of ["ª", "º", "Ü", "Ä", "Ö", "Å", "Ø", "ø", "ð", "Þ", "€", "±", "½", "‐", "†"]) {
      expect(charset.has(char), `${char} is set by the font and must be listed`).toBe(true);
    }
    // And the control: three characters Latin Modern genuinely does NOT have
    // stay out. U+0254 is the open o that failed CI in Bengali, U+01E3 the
    // ae-with-macron that failed it in Latin, U+1E9E the capital eszett German
    // wanted. A regeneration that swept in the whole of Unicode would list these.
    for (const char of ["ɔ", "ǣ", "ẞ"]) {
      expect(charset.has(char), `${char} is NOT in the font and must not be listed`).toBe(false);
    }
  });
});
