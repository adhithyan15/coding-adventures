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
  // The burn-down. Every number here should move in the direction of MORE coverage;
  // none may go backwards.
  //
  // The starting line was 0 of 40 in all 22 tracks — not because the tracks taught no
  // verbs (Spanish teaches nineteen) but because every existing verb tag was NAMESPACED
  // and therefore joined nothing. Adding the canonical concepts was the enabling step.
  //
  // Arabic is the first track to actually realize any of them: six core verbs, taught
  // one word per lesson across three new chapters — dhahaba (VERB-GO), jāʾa (VERB-COME),
  // qāla (VERB-SAY), raʾā (VERB-SEE), ʿarafa (VERB-KNOW), akala (VERB-EAT). So:
  //
  //   tracksWithNoCoreVerb   22 -> 21   (Arabic left the zero column)
  //   universallyMissing     40 -> 34   (the six Arabic now teaches are no longer
  //                                      missing EVERYWHERE — they are still missing
  //                                      from the other 21 tracks)
  //   meanCoveredPercent      0 ->  1   (one track at 15%, twenty-one at 0, rounded)
  it("pins core verb coverage: Arabic is the first track off zero", () => {
    const { lessons, taxonomy } = loadEverything();
    const report = verbCoverage(lessons, taxonomy);

    expect(report.summary.coreVerbCount).toBe(40);
    expect(report.summary.tracksWithNoCoreVerb).toBe(21);
    expect(report.summary.universallyMissing).toHaveLength(34);
    expect(report.summary.meanCoveredPercent).toBe(1);

    // The six, in taxonomy order, so a regression that silently drops one is visible
    // as a named verb rather than as a count.
    const arabic = report.tracks.find((track) => track.language === "arabic")!;
    expect(arabic.covered).toEqual([
      "VERB-GO",
      "VERB-COME",
      "VERB-SAY",
      "VERB-SEE",
      "VERB-KNOW",
      "VERB-EAT",
    ]);
    expect(arabic.coveredPercent).toBe(15);

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
