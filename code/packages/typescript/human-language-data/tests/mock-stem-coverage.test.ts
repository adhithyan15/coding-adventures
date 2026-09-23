// HL-C421 -- the reporter that replaces a hand pass that was wrong three times.
//
// The gates below are about ONE property above all: NOTHING IS SILENTLY
// DROPPED. Every hand pass failed by clearing a word on a morphological guess,
// so the tests that matter most are the ones proving a guess puts a word in a
// bucket that is still printed, beside the word it matched.

import { describe, expect, it } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  loadStemVocabulary,
  reportSpanishMockStemCoverage,
  parseMockPaper,
  reportStemCoverage,
  wordForms,
  type StemVocabulary,
} from "../src/mock-stem-coverage.js";
import { defaultCurriculumRoot } from "../src/loader.js";
import { runMockStemCoverageCli } from "../src/mock-stem-coverage-cli.js";
import { spanishMockAuditPath } from "../src/spanish-a1-mock-audit-cli.js";

const VOCAB: StemVocabulary = {
  functionWords: ["el", "la", "de", "que"],
  auxiliaryForms: ["es", "son"],
  examApparatus: ["opción"],
  properNouns: ["alberto"],
  derivation: {
    minStemLength: 3,
    verbEndings: ["ar", "er", "ir", "ado", "aba", "ó", "e", "a"],
    nominalSuffixes: ["s", "es", "o"],
    stemChanges: [["ie", "e"], ["ue", "o"]],
  },
};

describe("parseMockPaper", () => {
  it("reads a stem and its options", () => {
    const items = parseMockPaper(
      ["**7.** ¿Quién practica deporte?", "", "- a) el vecino.", "- b) la portera."].join("\n"),
    );
    expect(items).toEqual([
      { item: 7, stem: "¿Quién practica deporte?", options: ["el vecino.", "la portera."] },
    ]);
  });

  it("trims the capture rather than letting the pattern eat the whitespace", () => {
    // The `\s*` that used to sit before each `(.*)` capture is what CodeQL
    // flagged: both halves match a tab, so a failing match explores every split
    // point. Measured on the stem pattern, 40k tabs went from 743ms to 0ms.
    // Trimming afterwards has to give the same answer, which is what this pins.
    const items = parseMockPaper(
      ["**7.**\t  ¿Quién practica?  ", "- a)\t el vecino. ", "-   b)  la portera."].join("\n"),
    );
    expect(items).toEqual([
      { item: 7, stem: "¿Quién practica?", options: ["el vecino.", "la portera."] },
    ]);
  });

  it.each([
    ["a lone CR", "\r"],
    ["U+2028", "\u2028"],
    ["U+2029", "\u2029"],
  ])("keeps stem and options when lines are separated by %s", (_label, terminator) => {
    // `.` cannot match these, so once `\s*` left the pattern they made `$`
    // unreachable and the WHOLE ITEM was dropped -- number, stem and options
    // together. That is the failure this module exists to prevent, arriving
    // through the fix for a different one. They are split off as line
    // terminators now, which is what they are.
    //
    // The STEM is asserted, not just the item count. A first version of this
    // test put the terminator INSIDE the stem and checked only
    // `toHaveLength(1)` and the option, so it passed green against a stem of
    // "". It pinned a weaker property than the comment beside the code claimed,
    // which in this module is the whole failure mode.
    const items = parseMockPaper(`**7.** ¿Quién practica?${terminator}- a) el vecino`);
    expect(items).toEqual([
      { item: 7, stem: "¿Quién practica?", options: ["el vecino"] },
    ]);
  });

  it.each([
    ["a lone CR", "\r"],
    ["U+2028", "\u2028"],
    ["U+2029", "\u2029"],
  ])("splits a stem that %s cuts in half, and does not reassemble it", (_l, terminator) => {
    // THE HONEST LIMIT, pinned so nobody re-asserts otherwise. Splitting
    // rescues the item record; it does not glue the tail back on.
    // `¿Quién?` becomes a bare line that matches neither pattern and is
    // dropped -- the same way passage prose is dropped, because this reporter
    // reads stems and options, and a line that is neither has never been in
    // scope.
    //
    // Reassembling it would mean appending every unmatched line to the current
    // stem, which would sweep the whole audio passage into the report and
    // change every number in it. So the behaviour stays and the claim shrinks
    // to fit: the design rule is that a word the reporter READ is never
    // silently cleared, not that every word on the page is read.
    const items = parseMockPaper(`**7.**${terminator}¿Quién?\n- a) el vecino`);
    expect(items).toEqual([{ item: 7, stem: "", options: ["el vecino"] }]);
  });

  it("keeps an option whose marker is preceded by a non-breaking space", () => {
    // A paste from a PDF leaves U+00A0 behind. Narrowing `\s*` to `[ \t]*`
    // silently dropped the option; `[^\S\r\n]*` keeps it and is still
    // disjoint from `[a-z]`, so it cannot backtrack.
    const items = parseMockPaper("**7.** ¿Quién?\n-\u00a0a) el vecino\n- b) la portera");
    expect(items[0]?.options).toEqual(["el vecino", "la portera"]);
  });

  it("keeps an item that has no options of its own", () => {
    // Tarea 4 matches items against a shared Enunciados block, so an item
    // there legitimately carries none. Dropping it would hide its stem.
    expect(parseMockPaper("**44.** Afirma que el pedido llegó tarde.")[0]?.options).toEqual([]);
  });
});

describe("wordForms", () => {
  it("keeps accents, because the taught set does", () => {
    // `título` and `titulo` are different strings to the gate. Folding here
    // would credit one for the other and the report would go quiet.
    expect(wordForms("El título")).toEqual(["el", "título"]);
  });

  it("does not split an accented word, which /\\w+/ would", () => {
    expect(wordForms("mañana")).toEqual(["mañana"]);
  });
});

describe("reportStemCoverage", () => {
  const items = [{ item: 1, stem: "", options: [] }];
  const run = (text: string, taught: string[] = [], requires: string[] = []) =>
    reportStemCoverage(
      [{ ...items[0]!, stem: text }],
      new Set(taught),
      new Map([[1, new Set(requires)]]),
      VOCAB,
    );

  it("files a taught word as taught", () => {
    expect(run("perro", ["perro"])[0]).toMatchObject({ form: "perro", bucket: "taught" });
  });

  it.each([
    ["function word", "que", "function-word"],
    ["auxiliary form", "son", "function-word"],
    ["exam apparatus", "opción", "apparatus"],
    ["proper noun", "alberto", "proper-noun"],
  ])("files a declared %s out of the report", (_label, word, bucket) => {
    expect(run(word)[0]).toMatchObject({ bucket });
  });

  it("files a word with NO taught relative as unaccounted", () => {
    expect(run("multa")[0]).toMatchObject({ form: "multa", bucket: "unaccounted" });
  });

  it("files a morphological guess as DERIVABLE, never as cleared, and names the match", () => {
    // THE CENTRAL PROPERTY. A guess never removes a word from the report; it
    // moves it to a bucket that still prints, beside the word it matched, so a
    // reader can reject the match in a second.
    const [row] = run("hablado", ["hablar"]);
    expect(row).toMatchObject({ form: "hablado", bucket: "derivable", matched: "hablar" });
  });

  it("does not match espacio to esperar, the false clear that started this", () => {
    // The hand pass cleared `espacio` by a 3-CHARACTER PREFIX against the
    // taught `esperar`, and the word vanished until review found it in the stem
    // of an item that was passing. This reporter strips declared SUFFIXES
    // rather than comparing prefixes -- `espacio` reduces to `espaci`,
    // `esperar` to `esper` -- so the two never meet. Pinned so that adding a
    // prefix rule later has to break a test that says why there isn't one.
    expect(run("espacio", ["esperar"])[0]).toMatchObject({
      form: "espacio",
      bucket: "unaccounted",
    });
  });

  it("counts a word the row already claims, even in another form", () => {
    // Rows are written in citation forms and papers carry inflected ones. A
    // string comparison re-flagged `alumnos` in the very item whose row had
    // just been repaired with `alumno`.
    expect(run("alumnos", [], ["alumno"])[0]).toMatchObject({ bucket: "in-requires" });
  });

  it("records every place a form appears, so a stem hit is not hidden by an option hit", () => {
    const report = reportStemCoverage(
      [{ item: 22, stem: "Los alumnos mayores", options: ["vigilar a los alumnos"] }],
      new Set(),
      new Map(),
      VOCAB,
    );
    expect(report.find((row) => row.form === "alumnos")?.places).toEqual(["22 option", "22 stem"]);
  });
});

describe("the committed vocabulary", () => {
  it("loads, and declares every list the reporter indexes into", () => {
    const vocabulary = loadStemVocabulary(defaultCurriculumRoot());
    expect(vocabulary.functionWords.length).toBeGreaterThan(50);
    expect(vocabulary.examApparatus.length).toBeGreaterThan(20);
    expect(vocabulary.derivation.minStemLength).toBe(3);
  });

  it("does NOT list afirmar as apparatus", () => {
    // It is apparatus in the two instruction lines, but mock 1 item 41 is a
    // SCORED statement whose row already lists three other words from the same
    // stem. Calling it apparatus was the inconsistency review caught.
    expect(loadStemVocabulary(defaultCurriculumRoot()).examApparatus).not.toContain("afirmar");
  });

  it.each([
    ["functionWords is a string", { functionWords: "el" }],
    ["auxiliaryForms is a string", { auxiliaryForms: "es" }],
    ["properNouns is a string", { properNouns: "ana" }],
    ["verbEndings is a string", { derivation: { minStemLength: 3, verbEndings: "ar", nominalSuffixes: [], stemChanges: [] } }],
    ["nominalSuffixes is a string", { derivation: { minStemLength: 3, verbEndings: [], nominalSuffixes: "s", stemChanges: [] } }],
    ["derivation is missing", { derivation: undefined }],
    ["examApparatus holds a number", { examApparatus: [1] }],
    ["stemChanges is not pairs", { derivation: { minStemLength: 3, verbEndings: [], nominalSuffixes: [], stemChanges: ["ie"] } }],
    ["minStemLength is zero", { derivation: { minStemLength: 0, verbEndings: [], nominalSuffixes: [], stemChanges: [] } }],
  ])("refuses a malformed vocabulary: %s", (_label, override) => {
    // A STRING where a list belongs is the one that fails OPEN: `includes` on a
    // string is a SUBSTRING test, so "a" would match every function word and
    // the report would come back empty and look clean. `root-slug-splits.ts`
    // carries the same guard, learned the same way.
    const root = mkdtempSync(join(tmpdir(), "stem-vocab-"));
    mkdirSync(join(root, "core"), { recursive: true });
    const base = {
      functionWords: [],
      auxiliaryForms: [],
      examApparatus: [],
      properNouns: [],
      derivation: { minStemLength: 3, verbEndings: [], nominalSuffixes: [], stemChanges: [] },
    };
    writeFileSync(
      join(root, "core", "spanish-mock-stem-vocabulary.json"),
      `${JSON.stringify({ ...base, ...override }, null, 2)}\n`,
    );
    expect(() => loadStemVocabulary(root)).toThrow(/spanish-mock-stem-vocabulary/);
  });
});

describe("the level is a closed set, not a path fragment", () => {
  it("refuses a level that would escape the curriculum root", () => {
    // `reportSpanishMockStemCoverage` is exported from `index.ts`, and a
    // TypeScript union does not survive the package boundary. A first draft
    // built its directory as `spanish/mocks/${level.toLowerCase()}`, so a
    // JavaScript caller passing this read `/home/user/etc/mock-1-paper.md` --
    // outside `code/learning/human-languages` entirely. It now goes through
    // `spanishMockDir`, which is a lookup and cannot build a path out of what
    // it is handed. Found by security review.
    expect(() =>
      reportSpanishMockStemCoverage(defaultCurriculumRoot(), "../../../../../../etc" as never),
    ).toThrow(/unknown mock level/);
  });

  it.each(["__proto__", "constructor", "toString"])(
    "refuses the prototype-shaped level %s rather than reading through it",
    (level) => {
      // `AUDIT_DIR[level]` on a plain object would find an inherited member for
      // each of these and hand back something that is not a directory at all.
      // `Object.hasOwn` is what closes that, and it is cheaper than proving the
      // lookup table can never gain a prototype.
      expect(() =>
        reportSpanishMockStemCoverage(defaultCurriculumRoot(), level as never),
      ).toThrow(/unknown mock level/);
    },
  );
});

describe("runMockStemCoverageCli", () => {
  it("refuses an unknown --level rather than building a path from it", () => {
    // The CLI's own three-way string comparison is the last place a level is
    // checked by open-coded equality rather than by the lookup table, and it is
    // what keeps the traversal above unreachable from the command line.
    // Nothing pinned it until review asked.
    const lines: string[] = [];
    expect(runMockStemCoverageCli(["--level", "../../etc"], defaultCurriculumRoot(), (t) => lines.push(t))).toBe(2);
    expect(lines).toEqual([]);
  });

  it("refuses a --level flag with nothing after it", () => {
    expect(runMockStemCoverageCli(["--level"], defaultCurriculumRoot(), () => {})).toBe(2);
  });

  it("reports both mocks on a valid level", () => {
    const lines: string[] = [];
    expect(runMockStemCoverageCli([], defaultCurriculumRoot(), (t) => lines.push(t))).toBe(0);
    const text = lines.join("");
    expect(text).toContain("A2 mock 1");
    expect(text).toContain("A2 mock 2");
    expect(text).toContain("unaccounted");
  });
});

describe("spanishMockAuditPath", () => {
  it("goes through the closed set too, because its result reaches a writeFileSync", () => {
    expect(spanishMockAuditPath("A2")).toBe("spanish/mocks/a2/book-bounded-audit.json");
    expect(() => spanishMockAuditPath("../../../etc" as never)).toThrow(/unknown mock level/);
    expect(() => spanishMockAuditPath("__proto__" as never)).toThrow(/unknown mock level/);
  });
});
