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
  parseMockPaper,
  reportStemCoverage,
  wordForms,
  type StemVocabulary,
} from "../src/mock-stem-coverage.js";
import { defaultCurriculumRoot } from "../src/loader.js";

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
