import { describe, expect, it } from "vitest";
import { parseSoundTagRegistry } from "../src/sound-tags.js";

describe("sound-tag registry", () => {
  it("parses sorted, per-track vocabularies", () => {
    expect(
      parseSoundTagRegistry({
        version: 1,
        tracks: {
          spanish: ["silent-h", "vowel-a"],
          tamil: ["retroflex-l", "short-a"],
        },
      }),
    ).toEqual({
      version: 1,
      tracks: {
        spanish: ["silent-h", "vowel-a"],
        tamil: ["retroflex-l", "short-a"],
      },
    });
  });

  it("rejects malformed, unsorted, duplicate, and unsafe entries", () => {
    for (const bad of [
      { version: 2, tracks: {} },
      { version: 1, tracks: [] },
      { version: 1, tracks: { tamil: ["short-a"], spanish: ["vowel-a"] } },
      { version: 1, tracks: { spanish: ["vowel-a", "silent-h"] } },
      { version: 1, tracks: { spanish: ["vowel-a", "vowel-a"] } },
      { version: 1, tracks: { "../spanish": ["vowel-a"] } },
      { version: 1, tracks: { spanish: ["Vowel A"] } },
    ]) {
      expect(() => parseSoundTagRegistry(bad)).toThrow(/sound-tag registry/);
    }
  });
});
