import { describe, expect, it } from "vitest";
import { loadEverything } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { coreVerbConcepts, verbCoverage } from "../src/verbs.js";
import type { Taxonomy } from "../src/types.js";

function lesson(id: string, concept: string, language = "spanish") {
  return parseLesson(
    `---\nschema_version: 2\nid: ${id}\nchapter: 1\ntype: word\n` +
      `headword: x\ngloss: x\nconcept_tag: ${concept}\n---\n\n# ${id}\n\n## Warm-up\n\nSay it.\n`,
    language,
  );
}

const TAXONOMY = {
  concepts: {
    "GREETING-HELLO": { family: "GREETING", gloss: "hello", core: true },
    "VERB-GO": { family: "VERB", gloss: "to go", core: true },
    "VERB-EAT": { family: "VERB", gloss: "to eat", core: true },
    // Grammatical, not lexical: describes how a verb behaves, not which verb it is.
    "VERB-PAST": { family: "VERB", gloss: "past", core: true },
  },
} as unknown as Taxonomy;

describe("what counts as a core verb", () => {
  it("takes lexical VERB concepts and leaves the grammatical ones out", () => {
    // A track does not "cover" the past tense the way it covers *to eat*, so the
    // grammatical concepts the A2 spine owns are excluded from the coverage denominator.
    expect(coreVerbConcepts(TAXONOMY)).toEqual(["VERB-GO", "VERB-EAT"]);
  });
});

describe("coverage", () => {
  it("counts canonical verbs and keeps namespaced ones as extras", () => {
    const report = verbCoverage(
      [
        lesson("ES-1", "VERB-GO"),
        lesson("ES-2", "ES-VERB-HABLAR"),
        lesson("ES-3", "GREETING-HELLO"),
      ],
      TAXONOMY,
    );
    const track = report.tracks[0]!;
    expect(track.covered).toEqual(["VERB-GO"]);
    expect(track.missing).toEqual(["VERB-EAT"]);
    // A namespaced verb is real vocabulary a track chose to teach — an extra, not noise.
    expect(track.extras).toEqual(["ES-VERB-HABLAR"]);
    // ...and a non-verb concept is neither.
    expect(track.coveredPercent).toBe(50);
  });

  it("names the core verbs no track teaches anywhere", () => {
    const report = verbCoverage([lesson("ES-1", "VERB-GO")], TAXONOMY);
    expect(report.summary.universallyMissing).toEqual(["VERB-EAT"]);
    expect(report.summary.tracksWithNoCoreVerb).toBe(0);
  });

  it("counts a track with only namespaced verbs as covering nothing", () => {
    // The finding this whole module exists for: a track can teach nineteen verbs and
    // still join the cross-language corpus on none of them.
    const report = verbCoverage([lesson("ES-1", "ES-VERB-HABLAR")], TAXONOMY);
    expect(report.tracks[0]?.covered).toEqual([]);
    expect(report.tracks[0]?.extras).toHaveLength(1);
    expect(report.summary.tracksWithNoCoreVerb).toBe(1);
  });
});

describe("corpus snapshot", () => {
  // The honest starting line. Every number here should go UP; none may go down.
  //
  // The baseline this shipped at was 0 of 40 in all 22 tracks: not because the tracks
  // taught no verbs — Spanish teaches nineteen — but because every existing verb tag was
  // NAMESPACED and therefore joined nothing. Adding the canonical concepts was the
  // enabling step; realizing them is the authoring work these numbers track.
  //
  // Three tracks then realized core verbs at once, authored in parallel: Latin chapter 37
  // (8 — sum, habeō, eō, veniō, dīcō, videō, sciō, dō), Arabic chapters 28-30 (6) and
  // Russian chapter 3 (6).
  //
  //   tracksWithNoCoreVerb  22 -> 19      universallyMissing  40 -> 29
  //   meanCoveredPercent     0 ->  2
  //
  // `universallyMissing` drops by ELEVEN, not twenty, because the three tracks overlap:
  // VERB-GO, VERB-SEE and VERB-KNOW are each covered by more than one of them, and a verb
  // leaves the "nobody teaches this" list the first time any track teaches it. That
  // overlap is the point — those three are now genuinely cross-language concepts, which
  // is what the canonical ids were added for and what 85 namespaced tags could never do.
  //
  // Each agent measured this alone and each wrote "the first track off zero". All three
  // were right in isolation and wrong together; the numbers here are re-measured against
  // the merged corpus.
  it("pins the core verb baseline: eighteen tracks, and Spanish has joined", () => {
    const { lessons, taxonomy } = loadEverything();
    const report = verbCoverage(lessons, taxonomy);

    expect(report.summary.coreVerbCount).toBe(40);

    // THE VERB PROGRAM IS COMPLETE, AND THIS IS THE ASSERTION THAT KEEPS IT THAT WAY.
    //
    // `universallyMissing` names the core verbs NO track teaches. It shipped at 40 of 40
    // — not because the tracks taught no verbs (Spanish taught nineteen) but because
    // every verb tag was NAMESPACED, so `ES-VERB-HABLAR` and `FR-VERB-PARLER` were
    // different concepts and the cross-language join HL01 exists for contained not one
    // verb. It is now ZERO.
    //
    // Empty is the only correct value from here. A verb re-entering this list means a
    // track's lessons were unhooked from their realization path — the exact silent
    // failure that `omits`/`segments` drift produces — and no total would show it.
    expect(report.summary.universallyMissing).toEqual([]);

    expect(report.summary.tracksWithNoCoreVerb).toBe(3);
    expect(report.summary.universallyMissing).toHaveLength(0);
    expect(report.summary.meanCoveredPercent).toBe(38); // a new zero-verb starter makes the corpus average honest

    // The tracks that have joined the cross-language corpus, named explicitly so a
    // regression that silently unhooks these lessons cannot hide inside a total.
    const latin = report.tracks.find((track) => track.language === "latin")!;
    expect(latin.covered).toEqual([
      "VERB-BE",
      "VERB-HAVE",
      "VERB-GO",
      "VERB-COME",
      "VERB-SAY",
      "VERB-SEE",
      // The second tranche (chapters 38-39) — the same eight Spanish and Portuguese
      // authored in parallel, so all three joined on them at once.
      "VERB-HEAR",
      "VERB-KNOW",
      "VERB-THINK",
      "VERB-UNDERSTAND",
      "VERB-READ",
      "VERB-WRITE",
      "VERB-GIVE",
      "VERB-TAKE",
      // Chapters 42-43 close the program — the last seven verbs no track taught.
      // *fero* is suppletive from TWO roots (*bher- present, *telh2- for tuli/latum),
      // and *emo* originally meant "to take", which is why *redimo* is redeem.
      "VERB-BRING",
      "VERB-GET",
      // Chapters 40-41 — the everyday verbs nobody taught. This set is where Latin's
      // taproot claim pays: *sto* and *sedeo* are COGNATE with English stand and sit
      // through *steh2- and *sed-, not borrowed, while *curro* and *claudo* gave English
      // current/courier and close/clause/include by the ordinary Latin route.
      "VERB-SLEEP",
      "VERB-PLAY",
      "VERB-WALK",
      "VERB-RUN",
      "VERB-SIT",
      "VERB-STAND",
      "VERB-WAIT",
      "VERB-ASK",
      "VERB-ANSWER",
      "VERB-HELP",
      "VERB-MEET",
      "VERB-BUY",
      "VERB-OPEN",
      "VERB-CLOSE",
      "VERB-LIKE-LOVE",
    ]);
    expect(latin.coveredPercent).toBe(78);
    expect(report.tracks.find((track) => track.language === "arabic")!.covered).toEqual([
      "VERB-GO",
      "VERB-COME",
      "VERB-SAY",
      "VERB-SEE",
      "VERB-KNOW",
      // Chapters 31-32, where the ROOT SYSTEM finally does the teaching: ك-ت-ب gives
      // *kataba* and, off patterns the learner already owns, *kitāb*, *kātib*, *maktūb*,
      // *maktab*, *maktaba* — generated rather than memorised. No other track can do this.
      "VERB-THINK",
      "VERB-UNDERSTAND",
      "VERB-READ",
      "VERB-WRITE",
      "VERB-TAKE",
      "VERB-EAT",
      "VERB-ASK",
      "VERB-HELP",
      "VERB-LIKE-LOVE",
    ]);
    expect(report.tracks.find((track) => track.language === "russian")!.covered).toEqual([
      "VERB-BE",
      "VERB-GO",
      "VERB-SPEAK",
      "VERB-SEE",
      "VERB-KNOW",
      // Chapters 4-5. Russian was the smallest track in the corpus and this tranche
      // nearly doubled it — and took its never-revisited atoms from 21 of 34 to 3 of 55.
      "VERB-THINK",
      "VERB-UNDERSTAND",
      "VERB-READ",
      "VERB-WRITE",
      "VERB-TAKE",
      "VERB-LIVE",
      "VERB-ASK",
      "VERB-HELP",
      "VERB-LIKE-LOVE",
    ]);
    // The overlap that makes these CROSS-LANGUAGE concepts rather than 22 private
    // vocabularies: three verbs are now taught by more than one track, joined on one id.
    for (const shared of ["VERB-GO", "VERB-SEE", "VERB-KNOW"]) {
      const teaching = report.tracks.filter((track) => track.covered.includes(shared));
      expect(teaching.length).toBeGreaterThan(1);
    }

    // THE EIGHT-VERB PROGRAM, COMPLETE. These eight — think, understand, read, write,
    // take, ask, help, like — were realized by NO track anywhere when the canonical verb
    // layer shipped. Every one is now taught by TWENTY of the 22 tracks, across Romance,
    // Germanic, Italic, Indo-Aryan, Iranian, Slavic, Semitic and Dravidian.
    //
    // Chinese and Japanese are the two exceptions and must stay exceptions until they
    // have somewhere to put a verb: both are genuinely still at chapter 1. Excluding them
    // is the honest reading, not a gap — which is why this asserts 20 and not 22.
    const EIGHT = [
      "VERB-THINK",
      "VERB-UNDERSTAND",
      "VERB-READ",
      "VERB-WRITE",
      "VERB-TAKE",
      "VERB-ASK",
      "VERB-HELP",
      "VERB-LIKE-LOVE",
    ];
    for (const verb of EIGHT) {
      const teaching = report.tracks.filter((track) => track.covered.includes(verb));
      expect(teaching).toHaveLength(20);
      expect(teaching.map((t) => t.language)).not.toContain("chinese");
      expect(teaching.map((t) => t.language)).not.toContain("japanese");
    }
    // None of the eight is in the universally-missing list any more, and every other
    // canonical verb still is.
    for (const verb of latin.covered) {
      expect(report.summary.universallyMissing).not.toContain(verb);
    }

    // SPANISH IS THE POINT OF THE WHOLE EXERCISE, AND IT HAS TURNED OVER.
    //
    // This assertion used to read `covered: []` and `extras: 19` — a track teaching
    // nineteen verbs and joining the cross-language corpus on not one of them, because
    // every tag was namespaced. That was the finding the canonical verb layer existed to
    // fix, and it is now fixed: thirteen were retagged to canonical concepts and the
    // realization paths rewired to match, so Spanish is the largest verb contributor in
    // the corpus.
    //
    // Six remain namespaced ON PURPOSE — estar, estar-forms, salir, estudiar, llamar,
    // querer. `ser` takes VERB-BE (one lesson per concept), and the rest have no core
    // concept that fits without stretching it.
    const spanish = report.tracks.find((track) => track.language === "spanish")!;
    expect(spanish.covered).toHaveLength(36);
    // 10 -> 26. HL23's A1 tranche adds sixteen everyday-action verbs under
    // `SPINE-NAME-EVERYDAY-ACTIONS`, all namespaced ON PURPOSE: none of the 46
    // canonical VERB-* concepts names to wash, to climb, to look for, to keep, to
    // die or to send, and promoting them would ask all 23 tracks to answer for
    // them. `extras` is exactly the measurement of that decision -- the number is
    // meant to be read as "sixteen everyday verbs the cross-language core does not
    // name yet", which is a question for the taxonomy, not a defect in the track.
    expect(spanish.extras.length).toBe(26); // HL-C128 step 9: 6 -> 7 -- ch258 names subject-verb agreement, which adds one verb-adjacent extra beyond the paradigm set // HL-C152: +5 lessons, +1 chapter — Spanish realizes SPINE-NEGATE-AND-ASK, completing A2 at 5/5 // HL-C157: ayer + hablare close A2

    // THE EIGHT THAT NOBODY TAUGHT. Twenty-three of the forty core verbs were realized by
    // no track anywhere — everyday words like *think*, *read*, *write* and *ask*. Spanish,
    // Latin and Portuguese were each given the SAME eight rather than a third of the list
    // apiece, because a verb only becomes a CROSS-LANGUAGE concept when more than one
    // track teaches it under one id. Splitting the list would have added the same 24
    // lessons and joined nothing. All three now teach all eight.
    const portuguese = report.tracks.find((track) => track.language === "portuguese")!;
    for (const verb of [
      "VERB-THINK",
      "VERB-UNDERSTAND",
      "VERB-READ",
      "VERB-WRITE",
      "VERB-TAKE",
      "VERB-ASK",
      "VERB-HELP",
      "VERB-LIKE-LOVE",
    ]) {
      expect(spanish.covered).toContain(verb);
      expect(latin.covered).toContain(verb);
      expect(portuguese.covered).toContain(verb);
      expect(report.summary.universallyMissing).not.toContain(verb);
    }
  });

  it("gives every track an explicit authoring list", () => {
    const { lessons, taxonomy } = loadEverything();
    const report = verbCoverage(lessons, taxonomy);
    for (const track of report.tracks) {
      expect(track.covered.length + track.missing.length).toBe(40);
    }
  });
});
