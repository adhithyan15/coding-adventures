// ---------------------------------------------------------------------------
// filmstrip-ledger.test.ts — generator AND gate for the book's filmstrip data
// ---------------------------------------------------------------------------
//
// This one file both WRITES `data/ductus/filmstrip-geometry.d/` and CHECKS
// that the committed copy still matches what the current pen paths and fonts
// produce. Which of the two it does is decided by Vite's mode:
//
//     npm run generate:filmstrip-ledger   ->  vitest --mode write   (writes)
//     npm run check:filmstrip-ledger      ->  vitest               (compares)
//     npm test / the BUILD                ->  vitest               (compares)
//
// It lives in the test suite rather than in a `bin/` script for a blunt
// reason: this package cannot run under plain Node. `scriptdata.ts` reads the
// canonical Japanese/Perso-Arabic/Tamil/Urdu inventories through a Vite virtual
// module, so every entry point into `DUCTUS` needs a Vite process. Vitest is
// the Vite process this package already has, and putting the gate inside the
// suite means the BUILD enforces it without any new wiring — a stroke edited
// in `src/strokes/` and not regenerated fails this package's own tests.
//
// WHICH letters get an entry is decided by the book, not by this package: the
// generator reads the curriculum's `core/figure-generation.json` and emits an
// entry for every letter a `script-filmstrip` target declared there draws, and
// for every letter of the derived lesson candidates (HL-C443). That keeps the
// generated file the size of what is actually printed instead of all 352
// authored glyphs, and it means adding a filmstrip to a lesson is one target
// plus one regeneration rather than an edit here.
// ---------------------------------------------------------------------------

import { beforeAll, describe, expect, it } from "vitest";
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { SCRIPTS } from "../src/scriptdata.ts";
import { loadLessons } from "@coding-adventures/human-language-data/src/loader.ts";
import {
  filmstripCandidates,
  MAX_PHRASE_WORDS,
  MAX_SEQUENCE_PIECES,
} from "@coding-adventures/human-language-data/src/figure-targets.ts";
import { renderScriptSequenceFilmstripFigure } from "@coding-adventures/human-language-data/src/figure-filmstrip.ts";
import { ductusFor, boundsOf, composeHeadlinePhrase, composeHeadlineWord, parseFont } from "../src/index.ts";
import type { LetterDuctus } from "../src/strokes.ts";
import type { GlyphOutline } from "../src/ductusview.ts";
import {
  buildFilmstripEntry,
  buildFilmstripLedger,
  captionSizeFor,
  MIN_TINY_PEN_SCALE,
  penSizeFor,
  serialiseFilmstripLedger,
  TINY_STROKE_EXTENT,
  FILMSTRIP_LEDGER_PATH,
  FILMSTRIP_LEDGER_DIRECTORY,
  type FilmstripEntry,
} from "../src/filmstrip-ledger.ts";

const CURRICULUM_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../../../learning/human-languages",
);

/** The book's own list of figures; only `script-filmstrip` targets matter here. */
interface FigureTarget {
  kind: string;
  script?: string;
  glyph?: string;
  /** A sequence target's letters, in writing order; `glyph` is then the headword. */
  letters?: string[];
}

// Loading every lesson is the slow part of building the ledger (seconds, not
// milliseconds), and the candidate list cannot change during one test run, so
// it is read once per file rather than once per generation.
let candidateCache: ReturnType<typeof filmstripCandidates> | undefined;
function letterLessonCandidates(): ReturnType<typeof filmstripCandidates> {
  candidateCache ??= filmstripCandidates(loadLessons(CURRICULUM_ROOT));
  return candidateCache;
}

/**
 * What the ledger draws for one key: a cited letter, or a composed word whose
 * ductus and outline `composeHeadlineWord` built from cited letters.
 */
type Wanted =
  | { script: string; glyph: string }
  | { script: string; glyph: string; word: { ductus: LetterDuctus; outline: GlyphOutline } };

function filmstripTargets(): Wanted[] {
  const config = JSON.parse(
    readFileSync(join(CURRICULUM_ROOT, "core", "figure-generation.json"), "utf8"),
  ) as { targets?: FigureTarget[] };
  const wanted = new Map<string, Wanted>();
  for (const target of config.targets ?? []) {
    if (target.kind !== "script-filmstrip") continue;
    if (typeof target.script !== "string" || typeof target.glyph !== "string") {
      throw new Error("script-filmstrip targets need a script and a glyph");
    }
    // Two lessons may legitimately print the same letter; the ledger holds it
    // once, and `buildFilmstripLedger` rejects an accidental second copy. A
    // sequence target asks for each of its letters, never for its headword.
    for (const glyph of target.letters ?? [target.glyph]) {
      wanted.set(`${target.script}:${glyph}`, { script: target.script, glyph });
    }
  }
  // HL-C443: single-letter writing lessons on switched-on tracks are candidates
  // too. A DECLARED target without a cited ductus is an authoring error and
  // throws below; a derived candidate without one is simply not drawn, because
  // the candidate list is every letter lesson, cited or not.
  //
  // A SEQUENCE candidate (a list of letters, or a word whose letters stand
  // apart) contributes its letters only when every one of them is cited — the
  // same all-or-nothing rule `withDerivedFilmstrips` applies, so the ledger
  // never carries a letter for a strip the book will not print.
  //
  // A SHARED-HEADLINE candidate (a Devanagari word: its letters' bodies, then
  // one headline) is composed here, from the cited letters and the font. A
  // word the composer refuses (an uncited letter, a broken printed headline)
  // gets no entry, so the book does not print it — the same rule as an
  // uncited letter. `headlineWordRefusals` below pins which corpus words that
  // is, and why.
  //
  // A shared-headline PHRASE (words separated by single spaces) is composed
  // word by word, each with its own headline, and contributes one entry per
  // word, but only when EVERY word composes: a phrase never prints with a
  // word missing. A one-letter word is that letter's own entry.
  for (const candidate of letterLessonCandidates()) {
    if (candidate.composition === "shared-headline" && candidate.letters !== undefined) {
      const composed = composeHeadlinePhrase(candidate.glyph, candidate.script, fontFor(candidate.script).parsed);
      if (composed.ok) {
        for (const word of composed.words) {
          wanted.set(`${candidate.script}:${word.glyph}`, {
            script: candidate.script,
            glyph: word.glyph,
            word: { ductus: word.ductus, outline: word.outline },
          });
        }
      }
      continue;
    }
    if (candidate.composition === "shared-headline") {
      const composed = composeHeadlineWord(candidate.glyph, candidate.script, fontFor(candidate.script).parsed);
      if (composed.ok) {
        wanted.set(`${candidate.script}:${candidate.glyph}`, {
          script: candidate.script,
          glyph: candidate.glyph,
          word: { ductus: composed.ductus, outline: composed.outline },
        });
      }
      continue;
    }
    const letters = candidate.letters ?? [candidate.glyph];
    if (letters.some((glyph) => ductusFor(glyph, candidate.script) === undefined)) continue;
    for (const glyph of letters) {
      wanted.set(`${candidate.script}:${glyph}`, { script: candidate.script, glyph });
    }
  }
  return [...wanted.values()];
}

const fonts = new Map<string, ReturnType<typeof parseFont>>();

/**
 * The letter's real outline, out of the font the curriculum says this script is
 * rendered in. The font is never named here: it is read from the script's own
 * canonical inventory, so a figure cannot be drawn from a font the lessons do
 * not use.
 */
function fontFor(script: string): { parsed: ReturnType<typeof parseFont>; font: string } {
  const inventory = SCRIPTS.find((candidate) => candidate.script === script);
  if (inventory === undefined) throw new Error(`no ${script} inventory`);
  const font = inventory.font;
  let parsed = fonts.get(font);
  if (parsed === undefined) {
    const bytes = readFileSync(resolve(CURRICULUM_ROOT, font));
    const buffer = bytes.buffer.slice(
      bytes.byteOffset,
      bytes.byteOffset + bytes.byteLength,
    ) as ArrayBuffer;
    parsed = parseFont(buffer);
    fonts.set(font, parsed);
  }
  return { parsed, font };
}

function outlineFor(script: string, glyph: string): { outline: GlyphOutline; font: string } {
  const { parsed, font } = fontFor(script);
  const drawing = parsed.glyphFor(glyph);
  if (drawing === undefined) {
    throw new Error(`${font} has no outline for ${script} ${glyph}`);
  }
  return {
    outline: { path: drawing.path, bounds: boundsOf(drawing.contours) },
    font,
  };
}

function currentLedgerBytes(): string {
  const entries = filmstripTargets().map((target) => {
    const { script, glyph } = target;
    if ("word" in target) {
      return buildFilmstripEntry(target.word.ductus, target.word.outline, fontFor(script).font);
    }
    const letter = ductusFor(glyph, script);
    if (letter === undefined) {
      throw new Error(
        `no cited ductus for ${script} ${glyph} — a filmstrip may never be ` +
          `drawn from an invented stroke order`,
      );
    }
    const { outline, font } = outlineFor(script, glyph);
    return buildFilmstripEntry(letter, outline, font);
  });
  return serialiseFilmstripLedger(buildFilmstripLedger(entries));
}

function currentLedgerOwnerBytes(): Map<string, string> {
  const ledger = JSON.parse(currentLedgerBytes()) as ReturnType<typeof buildFilmstripLedger>;
  const outputs = new Map<string, string>([
    ["_meta.json", `${JSON.stringify({ version: ledger.version, generator: ledger.generator }, null, 2)}\n`],
  ]);
  const scripts = [...new Set(ledger.entries.map((entry) => entry.script))].sort();
  for (const script of scripts) {
    outputs.set(
      `${script}.json`,
      `${JSON.stringify({ script, entries: ledger.entries.filter((entry) => entry.script === script) }, null, 2)}\n`,
    );
  }
  return outputs;
}

// These tests assert BYTES, not speed. Building the ledger renders every printed
// letter (21 since HL-C443 derived Tamil's letter lessons) and the first build
// also loads the whole curriculum to find those lessons; on a shared CI runner
// that is several times slower than a laptop, and it broke vitest's default 5 s
// per-test budget on the first CI run. So the curriculum load is done once in
// `beforeAll` with its own budget, and each test is given a budget sized to the
// work it does rather than to a wall-clock guess that measures runner load.
const LEDGER_BUILD_TIMEOUT_MS = 60_000;

describe("the printed filmstrip ledger", { timeout: LEDGER_BUILD_TIMEOUT_MS }, () => {
  const directory = join(CURRICULUM_ROOT, FILMSTRIP_LEDGER_DIRECTORY);

  beforeAll(() => {
    letterLessonCandidates();
  }, 120_000);

  it("matches the pen paths and fonts it was generated from", () => {
    const expected = currentLedgerOwnerBytes();
    expect(
      existsSync(join(CURRICULUM_ROOT, FILMSTRIP_LEDGER_PATH)),
      `${FILMSTRIP_LEDGER_PATH} is a forbidden generated monolith`,
    ).toBe(false);

    if (import.meta.env.MODE === "write") {
      mkdirSync(directory, { recursive: true });
      for (const [name, bytes] of expected) writeFileSync(join(directory, name), bytes, "utf8");
    }

    let names: string[];
    try {
      const stat = lstatSync(directory);
      if (stat.isSymbolicLink() || !stat.isDirectory()) throw new Error("not a real directory");
      names = readdirSync(directory).sort();
    } catch {
      throw new Error(
        `${FILMSTRIP_LEDGER_DIRECTORY} is missing — run ` +
          `\`npm run generate:filmstrip-ledger\` in script-ductus`,
      );
    }
    if (import.meta.env.MODE === "write") {
      for (const name of names) {
        if (!expected.has(name) && /^[a-z_][a-z0-9_-]*\.json$/.test(name)) {
          const stale = join(directory, name);
          const stat = lstatSync(stale);
          if (stat.isFile() && !stat.isSymbolicLink()) unlinkSync(stale);
        }
      }
      names = readdirSync(directory).sort();
    }
    expect(names, `${FILMSTRIP_LEDGER_DIRECTORY} owner set is stale`).toEqual([...expected.keys()]);
    for (const [name, bytes] of expected) {
      const path = join(directory, name);
      const stat = lstatSync(path);
      expect(stat.isFile() && !stat.isSymbolicLink(), `${path} must be a real file`).toBe(true);
      expect(
        readFileSync(path, "utf8"),
        `${FILMSTRIP_LEDGER_DIRECTORY}/${name} is stale — run ` +
          `\`npm run generate:filmstrip-ledger\` in script-ductus`,
      ).toBe(bytes);
    }
  });

  it("is byte-identical when generated twice", () => {
    // The book's `check:figures` gate compares bytes, so a filmstrip that
    // rendered differently on two runs would turn a green build red at random.
    expect(currentLedgerBytes()).toBe(currentLedgerBytes());
  });

  it("draws every frame from a cited stroke order", () => {
    const ledger = JSON.parse(currentLedgerBytes()) as {
      entries: Array<{ source: { url: string; citation: string }; frames: unknown[] }>;
    };
    expect(ledger.entries.length).toBeGreaterThan(0);
    for (const entry of ledger.entries) {
      expect(entry.source.citation).not.toBe("");
      expect(entry.source.url).toMatch(/^https?:\/\//);
      expect(entry.frames.length).toBeGreaterThan(0);
    }
  });
});

describe("Devanagari words in the real corpus", { timeout: LEDGER_BUILD_TIMEOUT_MS }, () => {
  beforeAll(() => {
    letterLessonCandidates();
  }, 120_000);

  it("composes every shared-headline candidate that fits, and names why the rest do not", () => {
    // Each Devanagari writing lesson whose headword is one word of bare
    // letters is a shared-headline candidate (figure-targets.ts). The ledger
    // holds the ones `composeHeadlineWord` could fit to the printed word; the
    // book prints exactly those. A word that fails would print nothing, so
    // the outcome of every corpus candidate is pinned here.
    const outcomes = Object.fromEntries(
      letterLessonCandidates()
        .filter((candidate) => candidate.composition === "shared-headline")
        .map((candidate) => {
          const font = fontFor(candidate.script).parsed;
          const composed =
            candidate.letters === undefined
              ? composeHeadlineWord(candidate.glyph, candidate.script, font)
              : composeHeadlinePhrase(candidate.glyph, candidate.script, font);
          return [candidate.lessonId, `${candidate.glyph}: ${composed.ok ? "composed" : composed.reason}`];
        }),
    );
    // नाम and सा joined when ā gained a cited ductus and a cited place in a
    // word (after its consonant's body, before the headline); मम नाम joined
    // with them, as the first phrase, composed word by word.
    expect(outcomes).toEqual({
      "HI-A1F01-name-label": "नाम: composed",
      "HI-W12-schwa-drop": "नाम: composed",
      "MW-W01-saa": "सा: composed",
      "SA-W03-mama-delayed-copy": "मम: composed",
      "SA-W03-mama-dictation": "मम: composed",
      "SA-W03-mama-guided-copy": "मम: composed",
      "SA-W03-mama-nama-delayed-copy": "मम नाम: composed",
      "SA-W03-mama-nama-dictation": "मम नाम: composed",
      "SA-W03-mama-nama-guided-copy": "मम नाम: composed",
    });
  });
});

describe("the longest Devanagari phrase the book takes", { timeout: LEDGER_BUILD_TIMEOUT_MS }, () => {
  it("prints no taller than the tallest strip already printed", () => {
    // A phrase strip grows one band per word. The cited letters with the most
    // movements in the narrowest words wrap each word to three rows of
    // frames; three such words is the book's cap (MAX_PHRASE_WORDS), and a
    // fourth would print 2,048 units tall. The line is the tallest strip in
    // print (GU-R13-doorway-nine-r3, 1,801.14 units). Composing a word runs
    // the ink checks, so each word is composed once and the strips are built
    // from the same entries.
    const worst = ["औइ", "औझ", "धऋ", "औब"];
    expect(MAX_PHRASE_WORDS).toBe(3);
    expect([...worst.slice(0, 3).join("")].length).toBeLessThanOrEqual(MAX_SEQUENCE_PIECES);
    const { parsed, font } = fontFor("devanagari");
    const entries = worst.map((word) => {
      const composed = composeHeadlineWord(word, "devanagari", parsed);
      if (!composed.ok) throw new Error(`${word}: ${composed.reason}`);
      return buildFilmstripEntry(composed.ductus, composed.outline, font);
    });
    const height = (count: number) => {
      const words = worst.slice(0, count);
      const svg = renderScriptSequenceFilmstripFigure("X", words.join(" "), entries.slice(0, count), "Word").svg;
      return Number(/height="([\d.]+)"/.exec(svg)![1]);
    };
    expect(height(3)).toBe(1571.14);
    expect(height(3)).toBeLessThanOrEqual(1801.14);
    expect(height(4)).toBeGreaterThan(1801.14);
  });
});

describe("building one entry", () => {
  const letter = ductusFor("\u0b85", "tamil")!;
  const tamil = outlineFor("tamil", "\u0b85");

  it("carries the authored labels through untouched", () => {
    const built = buildFilmstripEntry(letter, tamil.outline, tamil.font);
    expect(built.frames.map((frame) => frame.label)).toEqual(
      letter.strokes.flatMap((stroke) => stroke.segments.map((segment) => segment.label)),
    );
    expect(built.font).toBe(tamil.font);
    expect(built.source).toEqual(letter.source);
  });

  it("emits only drawing tags, never a script or a handler", () => {
    for (const frame of buildFilmstripEntry(letter, tamil.outline, tamil.font).frames) {
      expect(frame.markup).not.toMatch(/<(?!\/?(?:g|path|circle|text|tspan)[\s/>])/);
      expect(frame.markup).not.toMatch(/\son[a-z]+=/i);
      expect(frame.markup.startsWith("<g ")).toBe(true);
    }
  });

  it("sizes the caption from the letter's own box", () => {
    const size = captionSizeFor(letter, tamil.outline, {});
    expect(size).toBeGreaterThan(0);
    // An explicit caption size wins over the automatic one.
    const forced = buildFilmstripEntry(letter, tamil.outline, tamil.font, {
      highlightSegment: true,
      captionSize: size * 2,
    });
    expect(forced.viewBox.height).toBeGreaterThan(
      buildFilmstripEntry(letter, tamil.outline, tamil.font).viewBox.height,
    );
  });

  it("refuses a letter with nothing to draw", () => {
    expect(() =>
      buildFilmstripEntry({ ...letter, strokes: [] }, tamil.outline, tamil.font),
    ).toThrow(/no filmstrip frames/);
  });
});

describe("the pen and its tip on a tiny mark", () => {
  // A dot-sized mark's whole pen path spans tens of font units, and its frame
  // zooms in on it, so the default 26-unit line and 34-unit tip covered the
  // movement they were meant to show. `penSizeFor` scales both down below
  // TINY_STROKE_EXTENT and leaves every other letter alone.
  const tamilLetter = ductusFor("\u0b85", "tamil")!;
  const tamil = outlineFor("tamil", "\u0b85");
  const nukta = ductusFor("\u093c", "devanagari")!;
  const nuktaOutline = outlineFor("devanagari", "\u093c");
  const anusvara = ductusFor("\u0a82", "gujarati")!;
  const anusvaraOutline = outlineFor("gujarati", "\u0a82");
  const tips = (entry: FilmstripEntry) =>
    entry.frames.map((frame) => /class="ductus__tip"[^>]* r="([\d.]+)"/.exec(frame.markup)?.[1]);
  const penWidths = (entry: FilmstripEntry) =>
    entry.frames.map((frame) => /class="ductus__pen"[^>]* stroke-width="([\d.]+)"/.exec(frame.markup)?.[1]);

  /** A one-stroke letter whose pen path is a straight line `extent` units long. */
  const line = (extent: number) => ({
    ...tamilLetter,
    strokes: [{ segments: [{ label: "across", path: [{ x: 0, y: 0 }, { x: extent, y: 0 }] }] }],
  });

  it("leaves a letter of ordinary size exactly as it was", () => {
    expect(penSizeFor(tamilLetter, {})).toEqual({});
    const built = buildFilmstripEntry(tamilLetter, tamil.outline, tamil.font);
    expect(new Set(tips(built))).toEqual(new Set(["34"]));
    expect(penSizeFor(line(TINY_STROKE_EXTENT), {})).toEqual({});
  });

  it("scales the pen down in proportion below the threshold, with no jump", () => {
    expect(penSizeFor(line(75), {})).toEqual({ penWidth: 13, tipRadius: 17 });
    const justUnder = penSizeFor(line(TINY_STROKE_EXTENT - 1), {});
    expect(justUnder.tipRadius).toBeCloseTo(34 * (149 / 150), 1);
    expect(justUnder.penWidth).toBeCloseTo(26 * (149 / 150), 1);
  });

  it("never goes below the minimum, so the pen stays visible", () => {
    expect(MIN_TINY_PEN_SCALE).toBe(0.3);
    expect(penSizeFor(line(10), {})).toEqual({ penWidth: 7.8, tipRadius: 10.2 });
    expect(penSizeFor(line(0), {})).toEqual({ penWidth: 7.8, tipRadius: 10.2 });
  });

  it("draws the nukta and the Gujarati anusvara with a pen their own size", () => {
    // ़ spans 44 units (clamped to the minimum); ં spans 60 (scale 0.4).
    const dab = buildFilmstripEntry(nukta, nuktaOutline.outline, nuktaOutline.font);
    expect(tips(dab)).toEqual(["10.2"]);
    expect(penWidths(dab)).toEqual(["7.8"]);
    const dot = buildFilmstripEntry(anusvara, anusvaraOutline.outline, anusvaraOutline.font);
    expect(new Set(tips(dot))).toEqual(new Set(["13.6"]));
    expect(new Set(penWidths(dot))).toEqual(new Set(["10.4"]));
    // The box does not depend on the pen, so the panels are the same size.
    const fullPen = buildFilmstripEntry(anusvara, anusvaraOutline.outline, anusvaraOutline.font, {
      highlightSegment: true,
      penWidth: 26,
      tipRadius: 34,
    });
    expect(fullPen.viewBox).toEqual(dot.viewBox);
  });

  it("lets an explicit pen width or tip radius win", () => {
    expect(penSizeFor(nukta, { tipRadius: 20 })).toEqual({ penWidth: 7.8 });
    expect(penSizeFor(nukta, { penWidth: 5 })).toEqual({ tipRadius: 10.2 });
    expect(penSizeFor(nukta, { penWidth: 5, tipRadius: 20 })).toEqual({});
    const forced = buildFilmstripEntry(nukta, nuktaOutline.outline, nuktaOutline.font, {
      highlightSegment: true,
      tipRadius: 20,
    });
    expect(tips(forced)).toEqual(["20"]);
    expect(penWidths(forced)).toEqual(["7.8"]);
  });

  it("has nothing to scale on a letter with no pen path", () => {
    expect(penSizeFor({ ...tamilLetter, strokes: [] }, {})).toEqual({});
  });
});

describe("assembling the ledger", () => {
  const sample = (script: string, glyph: string): FilmstripEntry => ({
    script,
    glyph,
    font: "_fonts/X.ttf",
    source: { citation: "c", url: "https://example.org/c" },
    penLifts: 0,
    summary: "one unbroken stroke \u00b7 1 movement",
    viewBox: { minX: 0, minY: 0, width: 1, height: 1 },
    frames: [{ number: 1, label: "l", startsAfterLift: false, markup: "<path/>" }],
  });

  it("puts the entries in one order that depends only on their content", () => {
    const forward = buildFilmstripLedger([
      sample("tamil", "\u0b86"),
      sample("devanagari", "\u0906"),
      sample("tamil", "\u0b85"),
    ]);
    const shuffled = buildFilmstripLedger([
      sample("tamil", "\u0b85"),
      sample("tamil", "\u0b86"),
      sample("devanagari", "\u0906"),
    ]);
    expect(serialiseFilmstripLedger(forward)).toBe(serialiseFilmstripLedger(shuffled));
    expect(forward.entries.map((entry) => entry.script)).toEqual([
      "devanagari",
      "tamil",
      "tamil",
    ]);
  });

  it("refuses to hold the same letter twice", () => {
    expect(() =>
      buildFilmstripLedger([sample("tamil", "\u0b85"), sample("tamil", "\u0b85")]),
    ).toThrow(/duplicate entry/);
  });

  it("ends in exactly one newline, so the file is diffable", () => {
    const text = serialiseFilmstripLedger(buildFilmstripLedger([sample("tamil", "\u0b85")]));
    expect(text.endsWith("}\n")).toBe(true);
    expect(text.endsWith("\n\n")).toBe(false);
  });
});
