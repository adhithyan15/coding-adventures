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
  // Latin then became the first track to realize any of them. Chapter 37 authors eight
  // core-verb lessons — sum, habeō, eō, veniō, dīcō, videō, sciō, dō — one canonical
  // concept each:
  //
  //   tracksWithNoCoreVerb  22 -> 21      universallyMissing  40 -> 32
  //   meanCoveredPercent     0 ->  1      latin coveredPercent 0 -> 20
  //
  // `universallyMissing` drops by exactly the eight Latin realized, because no other
  // track covers any of them: the eight leave the "nobody teaches this" list and the
  // remaining thirty-two stay. `meanCoveredPercent` is 1 rather than 0 because one track
  // in twenty-two at 20% averages to 0.9%, which rounds to 1 — the first non-zero this
  // number has ever had.
  it("pins the core verb baseline: Latin realizes eight, the other 21 tracks none", () => {
    const { lessons, taxonomy } = loadEverything();
    const report = verbCoverage(lessons, taxonomy);

    expect(report.summary.coreVerbCount).toBe(40);

    expect(report.summary.tracksWithNoCoreVerb).toBe(21);
    expect(report.summary.universallyMissing).toHaveLength(32);
    expect(report.summary.meanCoveredPercent).toBe(1);

    // The one track that has joined the cross-language corpus, named explicitly so a
    // regression that silently unhooks these lessons cannot hide inside a total.
    const latin = report.tracks.find((track) => track.language === "latin")!;
    expect(latin.covered).toEqual([
      "VERB-BE",
      "VERB-HAVE",
      "VERB-GO",
      "VERB-COME",
      "VERB-SAY",
      "VERB-SEE",
      "VERB-KNOW",
      "VERB-GIVE",
    ]);
    expect(latin.coveredPercent).toBe(20);
    // None of the eight is in the universally-missing list any more, and every other
    // canonical verb still is.
    for (const verb of latin.covered) {
      expect(report.summary.universallyMissing).not.toContain(verb);
    }

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
