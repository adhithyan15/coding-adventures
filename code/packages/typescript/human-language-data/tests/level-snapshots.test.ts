import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  LEVEL_SNAPSHOT_DIR,
  generatedLevelSnapshotOutputsFromSummary,
  summarizeLevelTracks,
} from "../src/level-snapshot-cli.js";
import { defaultCurriculumRoot, loadEverything } from "../src/loader.js";
import { CEFR_LEVELS, summarizeLevels, type LevelSummary } from "../src/levels.js";

describe("conflict-resistant level snapshots", () => {
  it("pins every track exactly and reconstructs the exact corpus summary", () => {
    const { lessons, curricula, spine } = loadEverything();
    const summary = summarizeLevels(lessons, curricula, spine);
    const outputs = generatedLevelSnapshotOutputsFromSummary(summary);

    expect(outputs.size).toBe(summary.tracks.length);
    for (const [relative, expected] of outputs) {
      expect(relative.startsWith(`${LEVEL_SNAPSHOT_DIR}/`)).toBe(true);
      expect(readFileSync(resolve(defaultCurriculumRoot(), relative), "utf8"), relative).toBe(
        expected,
      );
    }

    const snappedTracks = [...outputs.values()].map((value) => JSON.parse(value));
    expect(snappedTracks).toEqual(summary.tracks);
    expect(summarizeLevelTracks(snappedTracks)).toEqual({
      totalLessons: summary.totalLessons,
      byLevel: summary.byLevel,
      unmapped: summary.unmapped,
      mappedPercent: summary.mappedPercent,
    });
  });

  it("gives two language tranches disjoint authored outputs", () => {
    const histogram = () => Object.fromEntries(CEFR_LEVELS.map((level) => [level, 0]));
    const baseline = {
      totalLessons: 2,
      byLevel: { ...histogram(), "pre-A1": 2 },
      unmapped: 0,
      mappedPercent: 100,
      tracks: [
        {
          language: "alpha",
          lessonCount: 1,
          byLevel: { ...histogram(), "pre-A1": 1 },
          unmapped: 0,
          reach: "pre-A1",
        },
        {
          language: "beta",
          lessonCount: 1,
          byLevel: { ...histogram(), "pre-A1": 1 },
          unmapped: 0,
          reach: "pre-A1",
        },
      ],
    } as LevelSummary;

    const alphaChange = structuredClone(baseline);
    alphaChange.tracks[0]!.lessonCount += 1;
    alphaChange.tracks[0]!.byLevel["pre-A1"] += 1;
    const betaChange = structuredClone(baseline);
    betaChange.tracks[1]!.lessonCount += 1;
    betaChange.tracks[1]!.byLevel["pre-A1"] += 1;

    const before = generatedLevelSnapshotOutputsFromSummary(baseline);
    const changedKeys = (after: Map<string, string>) =>
      [...after].filter(([key, value]) => before.get(key) !== value).map(([key]) => key);

    expect(changedKeys(generatedLevelSnapshotOutputsFromSummary(alphaChange))).toEqual([
      `${LEVEL_SNAPSHOT_DIR}/alpha.json`,
    ]);
    expect(changedKeys(generatedLevelSnapshotOutputsFromSummary(betaChange))).toEqual([
      `${LEVEL_SNAPSHOT_DIR}/beta.json`,
    ]);
  });
});
