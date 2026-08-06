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
  // The burn-down. Every number here should go UP; none may go down.
  //
  // It started at 0 of 40 in all 22 tracks — not because the tracks taught no verbs
  // (Spanish teaches nineteen) but because every existing verb tag was NAMESPACED and
  // therefore joined nothing. Adding the canonical concepts was the enabling step;
  // realizing them is the authoring work these numbers track.
  //
  // HL-C42 is the first realization. Russian's Chapter 3 covers six of the forty —
  // VERB-BE, VERB-GO, VERB-SPEAK, VERB-KNOW, VERB-SEE, VERB-LIVE — so:
  //
  //   tracksWithNoCoreVerb  22 -> 21     universallyMissing  40 -> 34
  //   meanCoveredPercent     0 ->  1     (Russian is at 15%; 15/22 rounds to 1)
  //
  // `meanCoveredPercent` moving by a single point off six real lessons is the number
  // doing its job: it is a mean over 22 tracks, and one track's chapter is genuinely
  // one twenty-second of the problem.
  it("pins core verb coverage: Russian is the first track to realize any of it", () => {
    const { lessons, taxonomy } = loadEverything();
    const report = verbCoverage(lessons, taxonomy);

    expect(report.summary.coreVerbCount).toBe(40);

    expect(report.summary.tracksWithNoCoreVerb).toBe(21);
    expect(report.summary.universallyMissing).toHaveLength(34);
    expect(report.summary.meanCoveredPercent).toBe(1);

    // Taxonomy declaration order, not lesson order — `coreVerbConcepts` reads the
    // authored taxonomy so the authoring list is stable across tracks.
    const russian = report.tracks.find((track) => track.language === "russian")!;
    expect(russian.covered).toEqual([
      "VERB-BE",
      "VERB-GO",
      "VERB-SPEAK",
      "VERB-SEE",
      "VERB-KNOW",
      "VERB-LIVE",
    ]);
    expect(russian.coveredPercent).toBe(15);

    // The existing namespaced verbs are still counted, so the work already done is
    // visible rather than erased.
    const spanish = report.tracks.find((track) => track.language === "spanish")!;
    expect(spanish.extras.length).toBe(19);
    expect(spanish.covered).toEqual([]);
  });

  it("gives every track an explicit authoring list", () => {
    const { lessons, taxonomy } = loadEverything();
    const report = verbCoverage(lessons, taxonomy);
    for (const track of report.tracks) {
      expect(track.covered.length + track.missing.length).toBe(40);
    }
  });
});
