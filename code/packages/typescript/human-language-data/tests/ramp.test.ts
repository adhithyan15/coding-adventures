// HL08's gentle-ramp budgets, measured. See src/ramp.ts for why this did not exist.

import { describe, expect, it } from "vitest";
import { loadChapterPolicy, loadEverything } from "../src/loader.js";
import { measureRamp, measureScriptRamp } from "../src/ramp.js";
import { parseLesson } from "../src/parse.js";
import type { ChapterPolicy } from "../src/types.js";

const POLICY = {
  version: 1,
  payoffRepresentativeness: 0.5,
  maxNewAtomsPerLesson: 3,
  maxNewAtomsPerChapter: 12,
  maxLinearisableTableColumns: 3,
  maxNewGlyphsPerLesson: 3,
  maxNewScriptSystemsPerLesson: 1,
} as ChapterPolicy;

/** A lesson whose BODY carries `text`, for measuring what the reader must decode. */
function scriptLesson(
  id: string,
  language: string,
  chapter: number,
  sequence: number,
  text: string,
) {
  return parseLesson(
    `---\nschema_version: 2\nid: ${id}\nchapter: ${chapter}\nsequence: ${sequence}\n` +
      `type: word\nheadword: x\ngloss: x\nconcept_tag: GREETING-HELLO\n---\n\n# ${id}\n\n` +
      `## Warm-up\n\n${text}\n`,
    language,
  );
}

function lesson(id: string, chapter: number, atoms: string[]) {
  const directive = `<!-- hl-knowledge: introduces=[${atoms.join(", ")}]; assesses=[] -->\n\n`;
  return parseLesson(
    `---\nschema_version: 2\nid: ${id}\nchapter: ${chapter}\ntype: word\n` +
      `headword: x\ngloss: x\nconcept_tag: GREETING-HELLO\n---\n\n# ${id}\n\n` +
      `## Warm-up\n\n${directive}Say it.\n`,
    "spanish",
  );
}

describe("the lesson budget", () => {
  it("flags a lesson above the budget and leaves one at it alone", () => {
    const report = measureRamp(
      [lesson("ES-1", 1, ["A", "B", "C", "D"]), lesson("ES-2", 1, ["E", "F", "G"])],
      POLICY,
    );
    expect(report.lessons.map((v) => v.lessonId)).toEqual(["ES-1"]);
    // Exactly at the budget is compliant — it is a maximum, not a target to stay under.
    expect(report.summary.lessonViolations).toBe(1);
  });

  it("orders violations steepest first, so the list is a work queue", () => {
    const report = measureRamp(
      [lesson("ES-1", 1, ["A", "B", "C", "D"]), lesson("ES-2", 2, ["E", "F", "G", "H", "I"])],
      POLICY,
    );
    expect(report.lessons.map((v) => v.atoms)).toEqual([5, 4]);
    expect(report.summary.steepestLesson?.lessonId).toBe("ES-2");
  });
});

describe("the chapter budget", () => {
  it("catches a chapter that splitting alone would have hidden", () => {
    // Four compliant 3-atom lessons still put 12 new atoms in one chapter; a fifth
    // breaks the chapter budget even though no single lesson does. That is the whole
    // point of having both numbers — splitting must not be able to game the rule.
    const atoms = (n: number) => Array.from({ length: 3 }, (_, i) => `A${n}${i}`);
    const five = [1, 2, 3, 4, 5].map((n) => lesson(`ES-${n}`, 1, atoms(n)));
    const report = measureRamp(five, POLICY);
    expect(report.summary.lessonViolations).toBe(0);
    expect(report.summary.chapterViolations).toBe(1);
    expect(report.chapters[0]).toMatchObject({ chapter: 1, atoms: 15, lessonCount: 5 });
  });
});

describe("what the measurement cannot see", () => {
  it("reports atom-less lessons as unmeasurable, never as compliant", () => {
    // A schema-v1 lesson declares no atoms. Counting it as 0-and-therefore-fine would
    // let an unmigrated track look perfectly gentle.
    const report = measureRamp([lesson("ES-1", 1, []), lesson("ES-2", 1, ["A", "B", "C", "D"])], POLICY);
    expect(report.summary.unmeasurableLessons).toBe(1);
    expect(report.summary.measurablePercent).toBe(50);
    expect(report.summary.lessonViolations).toBe(1);
  });
});

describe("corpus snapshot", () => {
  // The first reproducible measurement of the gentle ramp. The quoted "52 over-budget
  // lessons" was an ad-hoc count no test reproduced, and it could not be reproduced
  // because the answer depends on how much of the corpus is schema-v2 that day.
  it("pins the ramp, and the size of its blind spot", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy());

    expect(report.policy).toEqual({ maxNewAtomsPerLesson: 3, maxNewAtomsPerChapter: 12 });
    expect(report.summary.lessonViolations).toBe(46); // HL-C128 step 8: +2 -- ES-C65-ahora-hoy and ES-C65-vi-di each introduce 4 atoms against a budget of 3. In both cases TWO of the four are etymons, which the level gate already waives from reinforcement as read-once rather than drilled, so the load on the reader is two new forms apiece. Recorded rather than hidden; if the budget should count etymons separately that is a change to the budget, not to these lessons. // HL-C128 step 10: +1 -- ES-C67-primero introduces 4 atoms against a budget of 3, one of which is an etymon the level gate already waives as read-once
    // 24 -> 21. HL-C94 splits Spanish's four over-budget opening chapters into
    // twelve: ch3 27 atoms, ch4 31, ch5 17, ch6 19 become twelve chapters of which
    // exactly one (ch7, 13) is still over. That is the owner's "the first chapters
    // are heavy and ramp up quickly", measured and paid down.
    //
    // 25 -> 24. Tamil chapter 1 used to hold all eleven script lessons and blew the
    // per-chapter atom budget (24 atoms against a budget of 12). Chapters 1-3 are now
    // pure speech and the script strand runs one lesson at a time from chapter 4, so
    // no Tamil chapter exceeds the budget. This is the number that most directly
    // measures "do not throw many things at the reader at once".
    expect(report.summary.chapterViolations).toBe(21); // -1: HL-C96 splits ch7, the last Spanish chapter over budget -- Spanish is now clean // +1: vocabulary wave 6

    // HALF THE CORPUS IS INVISIBLE HERE. 572 lessons declare no atoms, so they are
    // neither compliant nor violating — they are unmigrated. A track with few violations
    // and many unmeasurable lessons has not proved it is gentle. Ratchet this DOWN as
    // schema-v2 migration lands; the violation count will rise as it does, and that is
    // the measurement improving rather than the corpus worsening.
    // Five Chapter-7 teaching lessons now declare atoms; its practice lesson remains
    // correctly atom-free, so the blind spot falls by five rather than six.
    // Four Chapter-8 teaching lessons now declare atoms; Chapter 9 migrates four more.
    // Chapter 10 migrates three teaching lessons; Chapter 11 migrates four more;
    // Chapters 12 and 13 each migrate three teaching lessons.
    // Their terminal practices remain correctly atom-free, so only teaching lessons
    // leave the blind spot.
    // Chapter 14 migrates two teaching lessons; its terminal practice correctly
    // remains atom-free, so two more lessons leave the blind spot. Chapter 15
    // replaces two legacy teaching lessons with five measurable steps; its terminal
    // practice remains atom-free, so the blind spot falls by two more net of the
    // three added lessons.
    // Chapter 16 replaces two legacy teaching lessons with seven measurable
    // singular steps; its terminal checkpoint remains correctly atom-free.
    // Chapter 17 replaces three legacy teaching lessons with seven measurable
    // singular steps; its terminal checkpoint remains correctly atom-free.
    // Chapter 18 replaces nine legacy teaching lessons with eight measurable
    // singular steps; its terminal checkpoint remains correctly atom-free.
    // HL-C88 slice 8 moves this UP by two, which is the one direction this
    // ratchet is not supposed to travel, so it needs saying: the two lessons are
    // chapter 64's review and chapter 65's synthesis, and a review or a synthesis
    // introduces no atoms BY DESIGN -- it re-practises what the teaching chapters
    // introduced. They are terminal lessons of exactly the kind every note above
    // calls "correctly atom-free". The blind spot they add is real but it is not
    // unmigrated corpus, and migrating them would mean inventing atoms for lessons
    // whose whole job is to revisit.
    //
    // HL-C113 adds one more for the same reason: ES-C55-sintesis-condicion
    // closes the B1 si-condition rung and declares no atoms, because a
    // synthesis re-practises what the two teaching chapters introduced. A
    // ratchet that only moves down should not accept a cross-reference in
    // place of its own justification, so it is written out here.
    expect(report.summary.unmeasurableLessons).toBe(600); // +2: HL-C88 slice 8 review + synthesis, correctly atom-free // +1: B1 si-condition synthesis, same reason // +2: the preterite review and synthesis, both correctly atom-free -- a review introduces nothing by definition // HL-C113 step 7: +2 -- the reported-speech review and synthesis, correctly atom-free for the same reason as every other pair on this line // +1: HL-C128 step 2 -- ch225, the demonstrative review, introduces nothing by design, which is the same reason as every other pair on this line rather than a cross-reference to them // HL-C128 step 8: +1 -- ch256, the review, introduces nothing by design; the same reason as every other pair on this line // HL-C128 step 9: +1 -- ch261, the review, introduces nothing by design // HL-C128 step 10: +1 -- ch266, the closing review, introduces nothing by design
    // 65 -> 66: vocabulary wave 4 added 52 schema-v2 lessons. Chapter 10's three
    // newly measurable teaching lessons carry the corpus across one point, and the
    // five-step Chapter-15 migration carries it across the next. Chapter 18's eight
    // measurable teaching lessons carry the current corpus across another point.
    expect(report.summary.measurablePercent).toBe(74); // +1: HL-C98 // +1: vocabulary wave 5 added 40 schema-v2 lessons, all measurable // +1: vocabulary wave 6 added 54 schema-v2 lessons, all measurable // HL12: +30 recognition segments (telugu/kannada/malayalam 8 each, sanskrit 6) -- all schema-v2 and all atom-bearing, so the measurable share rises // HL-C137 wave II: +36 adjective lessons, +6 chapters, all six Indic tracks // HL-C156: letter ledgers replicated to all six — 85 one-character segments // HL-C166: +11 -- Sanskrit chapters 19 and 20
  });

  it("names the steepest lesson, which is where a burn-down starts", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy());
    expect(report.summary.steepestLesson).toMatchObject({ atoms: 6, budget: 3 });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// THE SCRIPT RAMP
//
// The second curve. Every test below exists because the atom budget could not
// see the thing it measures: `HI-W01-shirorekha-na-ma` declares ONE atom and
// shows TWELVE new Devanagari glyphs, and passed cleanly for a whole release.

describe("the script budget", () => {
  it("counts new target-script glyphs, which the atom budget cannot see", () => {
    // One atom, four glyphs — the exact shape of the bug this was built for.
    const report = measureScriptRamp(
      [scriptLesson("HI-1", "hindi", 1, 10, "क ख ग घ")],
      POLICY,
    );
    expect(report.summary.lessonViolations).toBe(1);
    expect(report.lessons[0]).toMatchObject({ glyphs: 4, budget: 3, systems: ["Devanagari"] });
  });

  it("charges a glyph once, to the lesson that first shows it", () => {
    // A ramp measurement, not a density measurement: what Chapter 1 taught is free
    // in Chapter 30, or every later lesson would be punished for revision.
    const report = measureScriptRamp(
      [
        scriptLesson("HI-1", "hindi", 1, 10, "क ख ग घ"),
        scriptLesson("HI-2", "hindi", 30, 10, "क ख ग घ"),
      ],
      POLICY,
    );
    expect(report.lessons.map((v) => v.lessonId)).toEqual(["HI-1"]);
  });

  it("walks lessons in reading order, not file order", () => {
    // Passed in back to front. Chapter 1 must still be the one charged.
    const report = measureScriptRamp(
      [
        scriptLesson("HI-LATER", "hindi", 30, 10, "क ख ग घ"),
        scriptLesson("HI-FIRST", "hindi", 1, 10, "क ख ग घ"),
      ],
      POLICY,
    );
    expect(report.lessons.map((v) => v.lessonId)).toEqual(["HI-FIRST"]);
  });

  it("ignores romanization, which exists to make the script approachable", () => {
    // `namaskāram` is Latin. Counting ā as a glyph to learn would drown the signal.
    const report = measureScriptRamp(
      [scriptLesson("HI-1", "hindi", 1, 10, "namaskāram, dhanyavād — ā ē ī ō ū ṣ ṭ ḻ")],
      POLICY,
    );
    expect(report.summary.lessonViolations).toBe(0);
  });

  it("counts combining marks, because an abugida is mostly marks", () => {
    // ा ि ी ु are mātrās — shapes the reader must decode. Dropping them would
    // undercount every Indic track in the corpus.
    const report = measureScriptRamp([scriptLesson("HI-1", "hindi", 1, 10, "का कि की कु")], POLICY);
    expect(report.lessons[0]?.glyphs).toBe(5); // क + four mātrās
  });

  it("counts script digits, which nobody born to ASCII can already read", () => {
    const report = measureScriptRamp([scriptLesson("HI-1", "hindi", 1, 10, "१ २ ३ ४ ५")], POLICY);
    expect(report.lessons[0]?.glyphs).toBe(5);
  });

  it("catches the prolonged-sound mark that Script= alone would miss", () => {
    // ー is Script=Common, Script_Extensions={Hira,Kana}. Matching the narrow
    // property undercounted コーヒー by the very mark that makes it a long vowel.
    const report = measureScriptRamp(
      [scriptLesson("JA-1", "japanese", 1, 10, "コーヒー アイスカフェ")],
      POLICY,
    );
    expect(report.lessons[0]?.sample).toContain("ー");
    // And it is only ever charged once, like any other glyph.
    expect(report.lessons[0]?.glyphs).toBe(new Set("コーヒーアイスカフェ").size);
  });
});

describe("the writing-system budget", () => {
  it("flags a lesson opening two systems at once", () => {
    // The owner's rule: sometimes you cannot introduce more than one script at a time.
    const report = measureScriptRamp(
      [scriptLesson("JA-1", "japanese", 1, 10, "こんにちは 今日")],
      POLICY,
    );
    expect(report.systems[0]).toMatchObject({ lessonId: "JA-1", systems: ["Han", "Hiragana"] });
  });

  it("leaves a single-system lesson alone however many glyphs it shows", () => {
    const report = measureScriptRamp(
      [scriptLesson("JA-1", "japanese", 1, 10, "あいうえおかきくけこ")],
      POLICY,
    );
    expect(report.summary.systemViolations).toBe(0);
  });
});

describe("the cousin layer", () => {
  it("counts foreign glyphs separately and never charges them to the budget", () => {
    // A Kannada Chapter-1 lesson showing the same word in four sister scripts is
    // context for a reader who knows one of them, not a reading obligation. Charging
    // both to one budget made KA-C01-dhanyavada look like a 34-glyph cliff; its real
    // Kannada load is 7.
    const report = measureScriptRamp(
      [scriptLesson("KA-1", "kannada", 1, 10, "ಧನ್ಯವಾದ — धन्यवाद நன்றி ధన్యవాదములు")],
      POLICY,
    );
    expect(report.summary.lessonsWithForeignScript).toBe(1);
    expect(report.summary.maxForeignGlyphsInALesson).toBeGreaterThan(3);
    // Kannada's own glyphs are what the budget judges.
    expect(report.lessons[0]?.systems ?? []).toEqual(["Kannada"]);
  });

  it("treats a Latin-script track as carrying no decoding burden", () => {
    const report = measureScriptRamp(
      [scriptLesson("ES-1", "spanish", 1, 10, "hola qué tal ñ")],
      POLICY,
    );
    expect(report.summary.lessonViolations).toBe(0);
    expect(report.tracks[0]).toMatchObject({ language: "spanish", latinScript: true, totalGlyphs: 0 });
  });
});

describe("the script ramp against the real corpus", () => {
  it("pins the script ramp, which no gate had ever measured", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy()).script;

    expect(report.policy).toEqual({ maxNewGlyphsPerLesson: 3, maxNewScriptSystemsPerLesson: 1 });

    // 61 lessons put more than three new shapes on the page at once. This is the
    // HL-C18C burn-down list. It is REPORT-ONLY: the debt predates the measurement,
    // so it is made visible rather than turned into a build failure.
    // 61 -> 60, and the net hides two moves in opposite directions. Two Tamil ch1
    // lessons left the list when chapter 1 gained a declared reading order. One
    // JOINED it: TA-C01-vanakkam. Removing its script block stopped it counting as a
    // script lesson, so the five glyphs of வணக்கம் itself now register against the
    // three-glyph budget — which is honest. A five-letter word IS five new shapes on
    // page one; the old structure hid that behind a "letters in this word" heading.
    // 60 -> 61, which is a net of three separate moves, not one.
    // OUT: TA-W03-pulli-vanakkam, at 9 glyphs the steepest Tamil lesson in the corpus.
    //      It moved to chapter 7, by which point its glyphs are no longer first
    //      appearances, so it stopped counting.
    // IN:  TA-C01-practice (5 glyphs, பயுேோ) and TA-C01-nandri (4, நனறி).
    // Both additions are SPEAKING lessons, and that is the trade being made knowingly:
    // Tamil now shows the script from page one and does not teach a letter until
    // chapter 4, so early spoken lessons display glyphs the writing strand has not
    // reached. This gate counts a glyph the first time it APPEARS, which is the right
    // rule for a track that teaches script alongside speech and the wrong one for a
    // track that deliberately shows before it teaches. The exposure is intended; the
    // count is honest; what it measures is no longer quite what Tamil is doing.
    expect(report.summary.lessonViolations).toBe(62); // HL-C134: the hand-written prose carried back into the lessons is now visible to this measurement — the words were always on the page, only the markdown had not seen them

    // All five are Japanese Chapter 1, which opens kanji beside hiragana in its very
    // first lesson and adds katakana in its fifth.
    expect(report.summary.systemViolations).toBe(5);
    expect(new Set(report.systems.map((v) => v.language))).toEqual(new Set(["japanese"]));

    // The cousin layer's footprint. Not a violation — a reason to keep that layer
    // visually skippable, so a reader who knows no sister language can pass it by.
    // 170 -> 173: vocabulary wave 4's Sanskrit lessons cite Devanagari daughter-language
    // cousin forms the same way earlier waves did.
    // 173 -> 184: vocabulary wave 5's telugu/malayalam lessons cite Dravidian cousin
    // forms in cousin scripts the same way.
    expect(report.summary.lessonsWithForeignScript).toBe(196); // HL-C134: the hand-written prose carried back into the lessons is now visible to this measurement — the words were always on the page, only the markdown had not seen them
    expect(report.summary.maxForeignGlyphsInALesson).toBe(27); // HL11: +1. Reordering moved which lesson is the FIRST to show a given cousin-script glyph, so one cousin table now carries one more first-sighting. Cousin glyphs are counted and never charged to the budget (HL08)
  });

  it("names the steepest lesson: one atom, twelve glyphs", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy()).script;
    expect(report.summary.steepestLesson).toMatchObject({
      lessonId: "MR-C01-dhanyavad", // HL11: Hindi lost this title by having its order fixed. HI-W01 still shows twelve glyphs, but Hindi's WORDS now come before it, so it is no longer the first place those glyphs appear. Marathi inherits the record with the same twelve -- and Marathi still has no declared order, which is why
      glyphs: 12,
      budget: 3,
    });
  });

  it("resolves every non-Latin track to a real script", () => {
    // Gujarati had no LANGUAGE_SCRIPT entry and silently resolved to `latin`, so its
    // 39 lessons read as having no script to learn — and `romanization` fell back to
    // the Gujarati headword, handing a voice assistant Gujarati in a Latin field.
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy()).script;
    const gujarati = report.tracks.find((t) => t.language === "gujarati");
    expect(gujarati).toMatchObject({ script: "gujarati", latinScript: false });
    expect(gujarati!.totalGlyphs).toBeGreaterThan(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// SECURITY REGRESSIONS
//
// All three came out of the pre-push security review of HL-C18C. Each was a way
// for a measurement to stop measuring without anyone noticing — which is the
// exact failure this module was written to end.

describe("a hostile or fat-fingered script id", () => {
  it("does not let a prototype key crash the whole report", () => {
    // `track.json` may declare ANY string as its script. `SCRIPT_SYSTEMS["__proto__"]`
    // returns Object.prototype, which is not nullish — so `?? ["Latin"]` never fires and
    // `new Set(Object.prototype)` throws. measureScriptRamp is called unconditionally by
    // measureRamp, so this took down the entire gap report, not just this section.
    for (const key of ["__proto__", "constructor", "toString", "valueOf", "hasOwnProperty"]) {
      const lesson = parseLesson(
        `---\nschema_version: 2\nid: X-1\nchapter: 1\nsequence: 10\ntype: word\n` +
          `headword: x\ngloss: x\nconcept_tag: GREETING-HELLO\n---\n\n# X\n\n## Warm-up\n\nक ख ग घ\n`,
        "hostile",
        key,
      );
      expect(() => measureScriptRamp([lesson], POLICY)).not.toThrow();
      // Unknown script falls back to Latin, i.e. "no decoding burden we can name" —
      // never a crash, and never a silent violation against the wrong inventory.
      const report = measureScriptRamp([lesson], POLICY);
      expect(report.tracks[0]).toMatchObject({ latinScript: true });
    }
  });
});

describe("chapter-policy.json validation", () => {
  it("refuses a budget that would silently disable the gate", async () => {
    // JSON.parse turns 1e999 into Infinity, and `size > Infinity` is false for every
    // lesson alive — so one typo publishes "0 violations", which reads in the report
    // exactly like "measured, found none".
    const { mkdtempSync, mkdirSync, writeFileSync } = await import("node:fs");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");
    const { loadChapterPolicy } = await import("../src/loader.js");

    const write = (policy: Record<string, unknown>) => {
      const root = mkdtempSync(join(tmpdir(), "hl-policy-"));
      mkdirSync(join(root, "core"), { recursive: true });
      writeFileSync(join(root, "core", "chapter-policy.json"), JSON.stringify(policy));
      return root;
    };
    const valid = {
      version: 1,
      payoffRepresentativeness: 0.5,
      maxNewAtomsPerLesson: 3,
      maxNewAtomsPerChapter: 12,
    };

    expect(() => loadChapterPolicy(write(valid))).not.toThrow();
    expect(() => loadChapterPolicy(write({ ...valid, maxNewGlyphsPerLesson: 1e999 }))).toThrow(
      /maxNewGlyphsPerLesson must be a non-negative integer/,
    );
    expect(() => loadChapterPolicy(write({ ...valid, maxNewGlyphsPerLesson: "banana" }))).toThrow();
    expect(() => loadChapterPolicy(write({ ...valid, maxNewGlyphsPerLesson: {} }))).toThrow();
    expect(() => loadChapterPolicy(write({ ...valid, maxNewGlyphsPerLesson: -1 }))).toThrow();
    expect(() => loadChapterPolicy(write({ ...valid, maxNewGlyphsPerLesson: 2.5 }))).toThrow();

    // The atom budgets are REQUIRED: measureRamp reads them with no default, so a policy
    // file missing them yields undefined and the same silent zero.
    const { maxNewAtomsPerLesson: _drop, ...missing } = valid;
    expect(() => loadChapterPolicy(write(missing))).toThrow(
      /maxNewAtomsPerLesson is required and missing/,
    );
  });
});

describe("the script-system map", () => {
  // These regexes are built at MODULE LOAD, and index.ts re-exports this file, so a bad
  // name takes down every CLI in the package — book generation, narration, modality,
  // validate. Importing this test file at all proves the current map compiles; what is
  // worth asserting is that each real track's matcher actually MATCHES its own script,
  // since a name can be valid Unicode and still be the wrong script for the track.
  it("gives every non-Latin track a matcher that finds its own glyphs", () => {
    const { lessons } = loadEverything();
    const report = measureRamp(lessons, loadChapterPolicy()).script;
    const nonLatin = report.tracks.filter((t) => !t.latinScript);

    expect(nonLatin.length).toBe(16);
    for (const track of nonLatin) {
      // A track whose matcher resolved to the wrong script would silently measure zero
      // glyphs across its whole corpus — the failure that hid Gujarati for a release.
      expect(track.totalGlyphs).toBeGreaterThan(0);
    }
  });

  it("rejects a plausible-looking name that is not a Unicode script", () => {
    // The names a future editor would reach for are exactly the ones that throw.
    for (const wrong of ["Kanji", "Nastaliq", "Devangari"]) {
      expect(() => new RegExp(`\\p{Script_Extensions=${wrong}}`, "u")).toThrow();
    }
    for (const right of ["Han", "Arabic", "Devanagari", "Hiragana", "Katakana"]) {
      expect(() => new RegExp(`\\p{Script_Extensions=${right}}`, "u")).not.toThrow();
    }
  });
});
