import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Japanese A1 inventory", () => {
  const inventory = loadExamInventory("japanese", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const lessons = loadTrackLessons("japanese");
    const taught = trackIntroducedAtoms(lessons, "japanese");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    for (const id of dropped) expect(sourceIds.has(id), `notTransferred names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
  });

  it("names the orthography points with NO Japanese analogue, one reason each", () => {
    // The instruction this file was written under: say which of the proxy's
    // orthography points have no Japanese analogue AT ALL. Three do, and they
    // are dropped in two entries rather than one, because the reasons differ
    // and a reader checking the file needs to see which is which.
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    const dropped = proxy.notTransferred.flatMap((entry) => entry.spanishPoints).sort();
    expect(dropped).toEqual(["A1-O1-04", "A1-O1-05", "A1-O1-06"]);
    const caseEntry = proxy.notTransferred.find((e) => e.spanishPoints.includes("A1-O1-04"))!;
    // Caseless in ALL THREE scripts — and the reason romaji does not rescue the
    // demand the way pinyin does for Chinese, which is the comparison that
    // stops the two files looking inconsistent.
    expect(caseEntry.why).toMatch(/caseless in all three of its scripts/i);
    expect(caseEntry.why).toMatch(/IT DOES NOT SURVIVE IN ROMAJI/);
    expect(caseEntry.why).toMatch(/Chinese inventory keeps the same two points/);
    // And the superscript entry names the track that DERIVED it.
    const abbrev = proxy.notTransferred.find((e) => e.spanishPoints.includes("A1-O1-06"))!;
    expect(abbrev.why).toMatch(/RU-A1-L-09/);
    expect(proxy.note).toMatch(/A1-O1-01 becomes THREE points, one per script/);
  });

  it("splits the script into three columns, and measures each separately", () => {
    // The point of the whole exercise. One column would report a number that
    // describes nothing; three report that hiragana is two thirds done,
    // katakana has barely started, and kanji is three characters.
    const categories = new Set(inventory.points.map((point) => point.category));
    expect([...categories].filter((c) => c.startsWith("Hiragana")).length).toBe(1);
    expect([...categories].filter((c) => c.startsWith("Katakana")).length).toBe(1);
    expect([...categories].filter((c) => c.startsWith("Kanji")).length).toBe(1);
    // No script point may be probed with a lexis atom, and no romanized
    // headword may stand in for a sign — the mechanical half of JA-A1-HYO-06.
    const script = inventory.points.filter((point) =>
      /^(Hiragana|Katakana|Kanji)/.test(point.category),
    );
    for (const point of script) {
      for (const atom of point.probe ?? []) {
        expect(atom.startsWith("JA-LEX-"), `${point.id} probes a lexis atom`).toBe(false);
      }
    }
    // And the mixed-script fact exists as a point of its own, because it is the
    // thing a single column would have lost.
    const mixed = inventory.points.find((point) => point.id === "JA-A1-HYO-01");
    expect(mixed, "the three-scripts-on-one-line point must exist").toBeDefined();
    expect(mixed!.probe).not.toBeNull();
  });

  it("enumerates the production tasks the exam anchor cannot score", () => {
    // The caveat is the reason this category exists, so the file must quote
    // what it says rather than merely act on it.
    expect(inventory.about).toMatch(/JLPT does not test production \(speaking and writing\) or interaction/);
    expect(inventory.about).toMatch(/THE EXAM ANCHOR CANNOT SCORE HALF THE CEFR\s+CONSTRUCT/);
    const production = inventory.points.filter((point) => point.category.startsWith("Sanshutsu"));
    // Four companion tasks from assessment-spec.md#a1, plus interaction.
    expect(production).toHaveLength(5);
    // Exactly one of the four TASKS is reachable, and it is the role-play.
    const tasks = production.filter((point) => /^JA-A1-PROD-0[1-4]$/.test(point.id));
    expect(tasks.filter((point) => point.probe !== null).map((point) => point.id)).toEqual([
      "JA-A1-PROD-04",
    ]);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; japaneseSpecific?: boolean };
      expect(cast.japaneseSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // The particles, politeness, in-group/out-group, the mora, the two kanji
    // readings, the mixed-script fact, the ha/wa spelling, the romaji decision,
    // the three script censuses, the five repair points and the five
    // production/interaction points. None has a Spanish point to derive from.
    expect(specific.length).toBeGreaterThanOrEqual(25);
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/NOT A TRANSCRIPTION OF THE JLPT SYLLABUS/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO\s+THE JAPAN FOUNDATION/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    expect(inventory.about).toMatch(/NO SEARCH WAS RUN, BY INSTRUCTION/);
    // The number this file must never invent.
    expect(inventory.about).toMatch(/NO JLPT KANJI OR\s+VOCABULARY LIST IS CITED ANYWHERE IN THIS FILE/);
    // Its prose envelope IS real and was used, which is what makes the
    // production points bindable rather than invented.
    expect(inventory.source).toMatch(/EXAM ENVELOPE: PARTIAL AND DANGLING, BUT ITS PROSE HALF IS REAL AND WAS USED/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(new Set(anchors.map((anchor) => anchor.kind))).toEqual(
      new Set(["sourced-proxy", "external-framework", "project-owned", "editorial"]),
    );
    for (const anchor of anchors) expect(anchor.note.trim().length, anchor.id).toBeGreaterThan(0);
    const known = new Set(anchors.map((anchor) => anchor.id));
    for (const point of inventory.points) {
      const ids = (point as unknown as { anchorIds?: string[] }).anchorIds;
      expect(ids?.length, `${point.id} names no anchor`).toBeGreaterThan(0);
      for (const id of ids ?? []) expect(known.has(id), `${point.id} cites unknown anchor ${id}`).toBe(true);
    }
  });

  it("reports a FULL repair column, an empty joining column, and three scripts at three depths", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const lessons = loadTrackLessons("japanese");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(179);
    expect(coverage.covered).toBe(67);
    expect(coverage.unmapped).toBe(112);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE, and it is a strength rather than a gap. This is the only
    // track measured so far that holds the complete CEFR A1 repair kit:
    // sumimasen, wakarimasen, mou ichido onegai shimasu, mou sukoshi yukkuri
    // itte kudasai, koko, wakarimashita. Russian has two of five moves and no
    // word for sorry; Chinese has one and no word for sorry; Gujarati had none.
    expect(coverage.byCategory["Kaiwa no un-ei - managing the conversation, and repairing it"]!).toEqual({
      enumerated: 8,
      covered: 7,
    });
    // AND THE SAME EMPTY COLUMN AS EVERY OTHER TRACK. Eighth in a row, third
    // outside South Asia. `demo`, `kara`, `node` and the quotative `to` return
    // zero occurrences in kana and in romaji. The best-built track in the
    // corpus — zero gentle-ramp findings of any kind — has Gujarati's joining
    // column, which is what makes the hole structural rather than neglect.
    expect(coverage.byCategory["Setsuzoku - joining two clauses"]!).toEqual({
      enumerated: 8,
      covered: 0,
    });
    // The three scripts, at three depths, which is the reason they are three
    // columns. Averaging 31/46, 2/46 and 3 would report nothing true.
    expect(coverage.byCategory["Hiragana - the first syllabary"]!.covered).toBe(2);
    expect(coverage.byCategory["Katakana - the second syllabary"]!).toEqual({ enumerated: 2, covered: 1 });
    expect(coverage.byCategory["Kanji - the third script, which is not a syllabary at all"]!.covered).toBe(3);
    // Particles are to Japanese what particles are to Mandarin, and the same
    // sentence is true of both tracks: one particle atom exists and it is about
    // spelling, not about the particle's job.
    expect(coverage.byCategory["Joshi - the particles, which do the work case endings do elsewhere"]!).toEqual({
      enumerated: 8,
      covered: 1,
    });
    // Fourteen body words and nine family words, and no word for "I".
    expect(coverage.byCategory["Daimeishi - pronouns, and the fact that Japanese avoids them"]!.covered).toBe(0);
    expect(coverage.byCategory["Karada - the body"]!.covered).toBe(2);
    // UNCHANGED at 66 by HL-C360, and that is the finding rather than an
    // oversight: chapters 14-15 taught ten cardinals and moved NO total,
    // because JA-A1-NUM-01 and JA-A1-NG2-01 were already ticked -- one on a
    // single numeral inside a phrase, the other on the vague half of counting.
    // The tranche deepened two ticks instead of adding one. A coverage total
    // cannot see that, which is why the named pins below exist.
    //
    // 66 -> 67. The counters (chapters 16-18) close JA-A1-NUM-02, and this is
    // the first Japanese-specific point in the file to be closed at all -- it
    // has no Spanish column behind it, because Spanish has no classifier
    // system. ONE point again, and again the total is the least of what moved:
    // the track's SECOND count now exists, the reader can attach a number to a
    // noun for the first time in 157 lessons, and JA-A1-NUM-03 changes from
    // "blocked upstream" to ordinary vocabulary work.
    expect(formatExamCoverage(coverage)).toContain(
      "japanese A1 (partial inventory): 67/179 points covered (37%)",
    );
  }, 60_000);

  // The cardinal point, named rather than left to the aggregate, because the
  // aggregate did not move. Both halves were falsified before this was kept: a
  // fabricated id fails the "probes only atoms that EXIST" test above, and
  // nulling the probe drops the total to 65 and fails the assertion above.
  it("closes JA-A1-NUM-01 on all ten cardinals, not on one inside a phrase", () => {
    const lessons = loadTrackLessons("japanese");
    const taught = trackIntroducedAtoms(lessons, "japanese");
    const cardinals = inventory.points.find((point) => point.id === "JA-A1-NUM-01");
    expect(cardinals?.probe).toEqual([
      // The ten, in numerical order here and in NO other order in the book: the
      // chapters teach ichi, go, ni, san, yon, then nana, hachi, ku, roku, juu.
      "JA-LEX-ICHI",
      "JA-LEX-NI",
      "JA-LEX-SAN",
      "JA-LEX-YON",
      "JA-LEX-GO",
      "JA-LEX-ROKU",
      "JA-LEX-NANA",
      "JA-LEX-HACHI",
      "JA-LEX-KU",
      "JA-LEX-JUU",
      // What carries the point past ten: a numeral before juu multiplies it and
      // one after it is added, so ten words reach ninety-nine.
      "JA-GRAMMAR-JUU-COMPOUND",
      // The seam the counters will run along, opened at four and held at seven.
      "JA-GRAMMAR-KUN-IN-THE-COUNT",
    ]);
    for (const atom of cardinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // The numeral the old tick rested on is still taught, in the chapter-9
    // phrase the cardinal lesson takes apart.
    expect(taught.has("JA-LEX-ICHIDO")).toBe(true);
    // The ordinals are still absent, and that point is now ordinary vocabulary
    // work rather than a block: a Japanese ordinal is a counter with dai- in
    // front or -me behind, and the counters exist as of chapter 18.
    expect(inventory.points.find((point) => point.id === "JA-A1-NUM-03")?.probe).toBeNull();
  }, 60_000);

  // The counter point, named rather than left to the aggregate. Both halves
  // were falsified before this was kept: a fabricated id fails the "probes only
  // atoms that EXIST" test above AND the per-atom loop below, and nulling the
  // probe fails the coverage total and this file's own `toEqual`.
  it("closes JA-A1-NUM-02 on BOTH counting systems, not on a list of counters", () => {
    const lessons = loadTrackLessons("japanese");
    const taught = trackIntroducedAtoms(lessons, "japanese");
    const counters = inventory.points.find((point) => point.id === "JA-A1-NUM-02");
    expect(counters?.probe).toEqual([
      // The machinery, first, because it is what the point is about: a number
      // in Japanese takes a counter chosen by the kind of thing.
      "JA-GRAMMAR-COUNTER-01",
      // The NATIVE series, which IS the general counter and is the whole of one
      // of the two systems. Ten words for five signs, across chapters 16-17.
      "JA-LEX-HITOTSU",
      "JA-LEX-FUTATSU",
      "JA-LEX-MITTSU",
      "JA-LEX-YOTTSU",
      "JA-LEX-ITSUTSU",
      "JA-LEX-MUTTSU",
      "JA-LEX-NANATSU",
      "JA-LEX-YATTSU",
      "JA-LEX-KOKONOTSU",
      "JA-LEX-TOO",
      // Two counters that are NOT the general one, chosen because the corpus's
      // own nouns can exercise them: the nine family words, and ashi and kami.
      "JA-LEX-NIN",
      "JA-LEX-HITORI-FUTARI",
      "JA-LEX-HON",
      // And the fact that a counter is pushed by the sound in front of it.
      "JA-GRAMMAR-COUNTER-SOUND-CHANGE-01",
    ]);
    for (const atom of counters?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // THE SEAM, asserted rather than described: the atom chapter 14 introduced
    // on yon is the same one chapters 16-18 return to three more times, which is
    // why it is NOT re-declared here under a counter-specific id.
    expect(taught.has("JA-GRAMMAR-KUN-IN-THE-COUNT")).toBe(true);
    // The counter the reader already owned and nobody had named. Its lesson said
    // "do counts an occurrence" in chapter 9; chapter 18 is where that sentence
    // is cashed, and no new atom was invented for it.
    expect(taught.has("JA-LEX-ICHIDO")).toBe(true);
    for (const absent of [
      "JA-LEX-DO-COUNTER",
      "JA-LEX-MAI",
      "JA-LEX-HIKI",
      "JA-LEX-SATSU",
      "JA-LEX-DAI",
    ]) expect(taught.has(absent), absent).toBe(false);
  }, 60_000);
});

// ---------------------------------------------------------------------------
// The first inventory for a track with NO ALPHABET, and the first with a TONE
// column at all.
//
// Two decisions in this file are larger than any point in it, so both are
// asserted here rather than left to a commit message.
//
//   1. SPANISH'S "ALPHABET" POINT IS ANSWERED BY THE STROKE, not by the
//      character. A1-O1-01 asks for the closed set of units a reader learns
//      once and reuses forever. Translating that as "the characters" would ask
//      a beginner's track for tens of thousands and report every Mandarin
//      course that has ever existed as failing; the strokes are the set the
//      question was really about, and the corpus teaches them, opening on `yi`
//      — one horizontal that is both a stroke and a whole character.
//
//   2. PINYIN IS A PRONUNCIATION CLAIM, NOT A SCRIPT CLAIM. The argument and
//      its three pieces of internal evidence are written into the file at
//      ZH-A1-PY-06 so a reader who disagrees can refile those points without
//      re-deriving it. The mechanical consequence is what this block checks:
//      the character column counts characters, so the 53 words the reader
//      knows by ear cannot inflate it.
//
// And one column exists because `exam-levels.json` said so. The chinese caveat
// records that GF0025-2021 defines its levels "across listening, speaking,
// reading, writing, and translation" — a fifth skill the PCIC inventories are
// monolingual by construction and cannot enumerate. `Fanyi` is in the file for
// that reason and comes back 0 of 2, with zero of the 175 lesson files
// declaring `mediation` in `modes`.
// ---------------------------------------------------------------------------
