/**
 * script-closure.test.ts — HL11: was the reader ever taught the letters?
 *
 * Glyphs are built from code points rather than typed, so a maintainer who
 * cannot read Tamil can still see what each fixture contains — and so a fixture
 * cannot drift into a lookalike from another script, which would silently change
 * what the test measures.
 */

import { describe, it, expect } from "vitest";
import { parseLesson } from "../src/parse.js";
import { measureScriptClosure } from "../src/script-closure.js";
import { loadEverything } from "../src/loader.js";

// TAMIL LETTER KA, MA, NA; TAMIL SIGN VIRAMA.
const KA = "க";
const MA = "ம";
const NA = "ந";
const VIRAMA = "்";

interface Options {
  type?: string;
  delivery?: string;
  headword?: string;
  romanization?: string;
  body?: string;
}

function lesson(id: string, sequence: number, o: Options = {}) {
  const front = [
    "---",
    "schema_version: 2",
    `id: ${id}`,
    `sequence: ${sequence}`,
    "chapter: 1",
    `type: ${o.type ?? "word"}`,
    `headword: "${o.headword ?? "x"}"`,
    'gloss: x',
    "concept_tag: GREETING-HELLO",
    ...(o.romanization === undefined ? [] : [`romanization: "${o.romanization}"`]),
    ...(o.delivery === undefined ? [] : [`delivery: ${o.delivery}`]),
    "---",
  ].join("\n");
  return parseLesson(
    `${front}\n\n# ${id}\n\n## Warm-up\n\n${o.body ?? "Say it."}\n`,
    "tamil",
  );
}

describe("closure", () => {
  it("flags a lesson whose body shows a glyph nothing taught", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { body: `Read this: ${KA}${MA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.glyphs).toBe([KA, MA].sort().join(""));
  });

  it("does not flag the same glyph once a script lesson has taught it", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `This letter: ${KA}` }),
      lesson("TA-2", 20, { body: `Read this: ${KA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
  });

  it("respects reading order, not file order", () => {
    // The teaching lesson comes LATER in sequence, so the word lesson is still
    // in debt even though the corpus contains the letter somewhere. A ramp is a
    // claim about order; measuring it out of order measures nothing.
    const report = measureScriptClosure([
      lesson("TA-W1", 90, { type: "writing", body: `This letter: ${KA}` }),
      lesson("TA-2", 20, { body: `Read this: ${KA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.lessonId).toBe("TA-2");
  });

  it("accepts `delivery: script` as teaching, not just `type: writing`", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { delivery: "script", body: `This letter: ${KA}` }),
      lesson("TA-2", 20, { body: `Read this: ${KA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.tracks[0]?.scriptLessons).toBe(1);
  });

  it("never puts a script lesson in debt to itself", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `Brand new: ${KA}${MA}${NA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.tracks[0]?.taughtGlyphs).toBe(3);
  });

  it("orders violations steepest first, as a work queue", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { body: `${KA}` }),
      lesson("TA-2", 20, { body: `${KA}${MA}${NA}${VIRAMA}` }),
    ]);
    expect(report.violations.map((v) => v.lessonId)).toEqual(["TA-2", "TA-1"]);
  });
});

describe("the exposure rule", () => {
  it("exempts a headword that carries a romanization", () => {
    // The reader is being SHOWN the word, not asked to read it -- the
    // romanization is the promise that they can use it without decoding.
    const report = measureScriptClosure([
      lesson("TA-1", 10, {
        headword: `${KA}${MA}`, romanization: "kama", body: `${KA}${MA}`,
      }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.tracks[0]?.exposureOnly).toBe(1);
  });

  it("does NOT exempt the same headword without one", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}${MA}`, body: `${KA}${MA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.tracks[0]?.headwordsWithoutRomanization).toBe(1);
  });

  it("does not let an exempt headword launder the rest of the lesson", () => {
    // This is the loophole worth guarding: the exemption covers the headword's
    // glyphs, not every glyph in a lesson that happens to have a headword.
    const report = measureScriptClosure([
      lesson("TA-1", 10, {
        headword: `${KA}`, romanization: "ka",
        body: `${KA} and now decode this: ${MA}${NA}`,
      }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.glyphs).toBe([MA, NA].sort().join(""));
  });

  it("counts an empty romanization as no romanization", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}`, romanization: "   ", body: `${KA}` }),
    ]);
    expect(report.summary.violations).toBe(1);
  });

  it("reports where the exposure rule is doing the work", () => {
    // The exposure count sits beside the violation count so the rule cannot
    // quietly become the reason the number looks good.
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}`, romanization: "ka", body: `${KA}` }),
      lesson("TA-2", 20, { headword: `${MA}`, romanization: "ma", body: `${MA}` }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.summary.exposureOnly).toBe(2);
  });
});

describe("track rollups", () => {
  it("counts glyphs shown, taught, and never taught", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `${KA}` }),
      lesson("TA-2", 20, { body: `${KA}${MA}` }),
    ]);
    const track = report.tracks[0]!;
    expect(track.taughtGlyphs).toBe(1);
    expect(track.shownGlyphs).toBe(2);
    expect(track.neverTaughtGlyphs).toBe(1);
  });

  it("names a track that teaches no letters at all", () => {
    const report = measureScriptClosure([lesson("TA-1", 10, { body: `${KA}` })]);
    expect(report.summary.tracksTeachingNothing).toBe(1);
    expect(report.tracks[0]?.scriptLessons).toBe(0);
  });

  it("skips Latin-script tracks entirely", () => {
    // Their reader arrives already knowing the alphabet, which is the whole
    // reason this measurement exists for the others.
    const spanish = parseLesson(
      "---\nschema_version: 2\nid: ES-1\nsequence: 10\nchapter: 1\ntype: word\n" +
        "headword: hola\ngloss: hello\nconcept_tag: GREETING-HELLO\n---\n\n# ES-1\n\nhola\n",
      "spanish",
    );
    const report = measureScriptClosure([spanish]);
    expect(report.tracks).toEqual([]);
    expect(report.summary.violations).toBe(0);
  });
});

describe("the real corpus", () => {
  const { lessons } = loadEverything();
  const report = measureScriptClosure(lessons);

  it("measures every non-Latin track and no Latin one", () => {
    expect(report.summary.tracksWithScript).toBeGreaterThanOrEqual(10);
    expect(report.tracks.map((t) => t.language)).not.toContain("spanish");
    expect(report.tracks.map((t) => t.language)).toContain("tamil");
  });

  it("finds far more closure debt than the pace budget ever could", () => {
    // The point of the whole module. HL08's glyph budget flags tens of lessons
    // for arriving too fast; closure flags hundreds for arriving untaught, and
    // a track can satisfy the budget perfectly while teaching nothing.
    expect(report.summary.violations).toBeGreaterThan(500);
    // Was `toBeGreaterThan(5)`, asserting the debt was large. It has stopped being
    // a fact about the corpus and started being a fact about how much of it has
    // been fixed: the Chinese, Japanese and Gujarati script tranches each removed
    // a track, 8 -> 5, and the floor failed. Debt assertions belong the other way
    // up, so this is now a CEILING on the same footing as the forward-reference
    // one — it may fall, never grow, and whoever raises it writes down why.
    // `violations` above still carries this test's stated point on its own.
    expect(report.summary.tracksTeachingNothing).toBeLessThanOrEqual(5); // 8 -> 5: chinese (HL-C209), japanese (HL-C211), gujarati (HL-C215)
  });

  it("every violation names real glyphs and a real lesson", () => {
    const ids = new Set(lessons.map((l) => l.realization.lessonId));
    for (const v of report.violations) {
      expect(ids.has(v.lessonId), v.lessonId).toBe(true);
      expect(v.count).toBe([...v.glyphs].length);
      expect(v.count).toBeGreaterThan(0);
    }
  });

  it("every Indic track now teaches letters, and Tamil still teaches the most", () => {
    // This test used to assert `scriptLessons === 0` for Telugu, Kannada and
    // Malayalam, which was true and was the problem: four of the six Indic
    // tracks taught no letter at all. HL12's recognition segments ended that, so
    // the old assertion is not re-pinned -- it is replaced by the claim it was
    // standing in for. Tamil leads because it leads on sourcing: its letters
    // have a cited stroke order and can be taught to the hand, where these three
    // scripts have none and are taught to the eye first.
    const tamil = report.tracks.find((t) => t.language === "tamil")!;
    for (const language of ["telugu", "kannada", "malayalam", "sanskrit"]) {
      const other = report.tracks.find((t) => t.language === language);
      if (!other) continue;
      expect(other.scriptLessons, language).toBeGreaterThan(0);
      expect(tamil.scriptLessons, language).toBeGreaterThan(other.scriptLessons);
      expect(tamil.neverTaughtGlyphs, language).toBeLessThan(other.neverTaughtGlyphs);
    }
  });
});

// --- Regressions from security review ---------------------------------------

describe("a headword's debt is not silently dropped", () => {
  it("counts an untaught headword whose glyphs are NOT in the body", () => {
    // The load-bearing set was built from the body alone, so a lesson whose
    // headword glyphs do not also appear verbatim in its body had its headword
    // debt vanish -- and then, worse, was counted as clean BECAUSE of an
    // exemption it had never claimed.
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}${MA}`, body: "All romanized here." }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.glyphs).toBe([KA, MA].sort().join(""));
    expect(report.tracks[0]?.exposureOnly).toBe(0);
  });

  it("never credits the exposure rule to a lesson with no romanization", () => {
    const report = measureScriptClosure([
      lesson("TA-1", 10, { headword: `${KA}`, body: "nothing in script" }),
    ]);
    expect(report.tracks[0]?.exposureOnly).toBe(0);
    expect(report.tracks[0]?.headwordsWithoutRomanization).toBe(1);
  });
});

describe("the exemption's real size is reported, not just its lesson count", () => {
  it("counts glyphs exempted even from a lesson that still violates", () => {
    // `exposureOnly` only counts lessons the rule FLIPPED to clean. A lesson
    // reporting five untaught glyphs while fifteen more were exempted is not a
    // lesson with five problems, and per-lesson counting cannot see that.
    const report = measureScriptClosure([
      lesson("TA-1", 10, {
        headword: `${KA}${MA}`, romanization: "kama",
        body: `${KA}${MA} then decode ${NA}`,
      }),
    ]);
    expect(report.summary.violations).toBe(1);
    expect(report.violations[0]?.glyphs).toBe(NA);
    // The two exempted glyphs are visible even though the lesson still violates.
    expect(report.summary.exposureExemptedGlyphs).toBe(2);
    expect(report.summary.exposureOnly).toBe(0);
  });

  it("counts a headword glyph that appears nowhere else in the lesson", () => {
    // The mirror of the bug above. A non-exempt headword is ADDED to the
    // load-bearing set, so what the exemption removes is the whole untaught
    // headword -- including glyphs the body never repeats. Guarding on the body
    // would suppress exactly the case whose omission was the first finding.
    const report = measureScriptClosure([
      lesson("TA-1", 10, {
        headword: `${KA}${MA}`, romanization: "kama", body: "all romanized",
      }),
    ]);
    expect(report.summary.violations).toBe(0);
    expect(report.summary.exposureExemptedGlyphs).toBe(2);
  });

  it("does not count a glyph as exempted once it has been taught", () => {
    const report = measureScriptClosure([
      lesson("TA-W1", 10, { type: "writing", body: `${KA}` }),
      lesson("TA-2", 20, { headword: `${KA}`, romanization: "ka", body: `${KA}` }),
    ]);
    expect(report.summary.exposureExemptedGlyphs).toBe(0);
  });
});

describe("an unknown script is unmeasured, not clean", () => {
  it("names the track instead of silently dropping it", () => {
    // Both "genuinely Latin" and "we do not recognise this" used to `continue`,
    // so a mistyped script made a whole track vanish from the report with
    // nothing anywhere saying so. That is the silent zero this module exists to
    // prevent, reached through the module itself.
    const odd = parseLesson(
      "---\nschema_version: 2\nid: XX-1\nsequence: 10\nchapter: 1\ntype: word\n" +
        "headword: x\ngloss: x\nconcept_tag: GREETING-HELLO\n---\n\n# XX-1\n\nx\n",
      "not-a-real-track",
    );
    // `parseLesson` falls back to "latin" for an unregistered language, so drive
    // the unknown-script path directly by overriding the resolved script.
    const forged = { ...odd, script: "klingon" } as typeof odd;
    const report = measureScriptClosure([forged]);
    expect(report.unknownScriptTracks).toEqual(["not-a-real-track"]);
    expect(report.summary.tracksWithUnknownScript).toBe(1);
    expect(report.tracks).toEqual([]);
  });

  it("reports zero unknown scripts for the real corpus", () => {
    const { lessons } = loadEverything();
    expect(measureScriptClosure(lessons).unknownScriptTracks).toEqual([]);
  });
});

describe("shared Indic marks are not all attributed to Devanagari", () => {
  it("counts a mark whose Script_Extensions include the track's script", () => {
    // U+0951 (DEVANAGARI STRESS SIGN UDATTA) has Script_Extensions covering
    // Bengali, Kannada, Malayalam, Tamil, Telugu and more. Asking "what script
    // is this glyph" returns Devanagari because that is where the map starts;
    // asking "does it belong to Tamil" is the question the caller means.
    const report = measureScriptClosure([
      lesson("TA-1", 10, { body: `${KA}॑` }),
    ]);
    expect(report.violations[0]?.count).toBe(2);
    expect(report.violations[0]?.glyphs).toContain("॑");
  });
});

describe("SCRIPT_SYSTEMS is frozen", () => {
  it("cannot be mutated into disagreeing with the matchers derived from it", async () => {
    // The matchers are built once at module load. A consumer adding a script
    // afterwards would pass membership tests while `belongsToAny` never learned
    // it, so the track would report ZERO debt while appearing measured.
    const { SCRIPT_SYSTEMS } = await import("../src/ramp.js");
    expect(Object.isFrozen(SCRIPT_SYSTEMS)).toBe(true);
    expect(Object.isFrozen(SCRIPT_SYSTEMS["tamil"])).toBe(true);
  });
});
