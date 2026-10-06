// Exact real-corpus evidence owned by the Devanagari inventory's marks.
// See HL24: unrelated script authors must not share an executable edit surface.
//
// Eight Devanagari signs carry a cited stroke order: native writers' tablet
// pen traces in HP Labs India's LipiTk Devanagari recognizer (MIT-licensed
// model; the underlying data is research-only, so only counts and shares are
// cited). The writers wrote each sign alone, so no record claims where the
// sign is written against its consonant or the headline, and the composer's
// WRITTEN_SIGN_SIDES has no Devanagari row
// (tests/figure-targets/devanagari-signs-drawn-alone). The other seven signs
// claim nothing; each says why below.

import { createHash } from "node:crypto";
import { expect } from "vitest";
import type { Mark } from "../../src/types.js";
import type { ScriptEvidenceContext } from "./helpers.js";

const LIPITK = "https://lipitk.sourceforge.net/lipi-reco.htm";

interface SignEvidence {
  readonly mark: string;
  readonly digest: string;
  readonly penLifts?: number;
  readonly strokeOrder?: readonly string[];
  /** The recognizer class the citation names. */
  readonly recognizerClass?: number;
}

const MARKS: readonly SignEvidence[] = [
  // Noto prints it with a piece of headline the native traces do not draw (68% draw the bar alone).
  { mark: "ा", digest: "4533604d4b31af58e78dbe71665990a6f7549f939a72bc6a60b41a4300e432ec" },
  // Noto prints it with a piece of headline the native traces do not draw.
  { mark: "ि", digest: "ff2a4b8e5a32f6c2ace602bc5183abc3be40a8c4e72c51e74b3e16cdf6713a0c" },
  // The drawn form is a weak majority (43%), and Noto adds a piece of headline.
  { mark: "ी", digest: "bc386a01d3c8eb5cdd9b66caad6f36447760414883559a1bbb3444bce9e2131a" },
  {
    mark: "ु",
    digest: "b13e795f2b9e5bc37c61cff989b8d3e800dc888ced6686db1bbbdff4defc80fd",
    penLifts: 0,
    strokeOrder: ["start at the tip of the upper arm and swing to the right", "without lifting, round the bowl and come back to the left", "without lifting, sweep out to the lower-left tip — and only now lift"],
    recognizerClass: 50,
  },
  {
    mark: "ू",
    digest: "89cd77b00a0f142122a897e2fa3534a690ff397fc6435a6d8843d3145b5454ae",
    penLifts: 0,
    strokeOrder: ["start at the inner tip of the loop and curl to the left", "without lifting, climb and arch over the top", "without lifting, sweep down to the right — and only now lift"],
    recognizerClass: 51,
  },
  {
    mark: "े",
    digest: "a61c1db78fbd7e09e76eb13dd5cc5e1fd4acf6fd3d2332bc51ab44b3aafee938",
    penLifts: 0,
    strokeOrder: ["start at the upper-left tip and arc to the right", "without lifting, curve down to the right — and only now lift"],
    recognizerClass: 53,
  },
  // The traces split on each flag's direction (34% draw both upper-left to lower-right).
  { mark: "ै", digest: "20b8a928777f2882c70efda2d860d0310bb3d2ff5e504bda74d32d19059af15e" },
  // A weak majority (51% two strokes), and Noto adds a piece of headline.
  { mark: "ो", digest: "634ca02495f9a580b472381d59a2edfb44971b80667f590f7f0f4f8c4897425e" },
  // No majority (42% three strokes).
  { mark: "ौ", digest: "6a2eb4500b715859c2b188dba1c7e842de6aeba131faca187a58b24255733621" },
  {
    mark: "ं",
    digest: "e92c14ab29e6450e9c553250e31d4dadf8bbf6c1b1ce4e807e6aa640edd2b988",
    penLifts: 0,
    strokeOrder: ["start at the top of the dot and curve down its left side", "without lifting, round the bottom and come up the right side to close the loop — and only now lift"],
    recognizerClass: 57,
  },
  {
    mark: "़",
    digest: "f11947cd5b3c3bb761100ead990ce88bbd6c6a2e68378ec4417e5280dfb33142",
    penLifts: 0,
    strokeOrder: ["dab the dot from its upper right down to the left — and only now lift"],
    recognizerClass: 62,
  },
  {
    mark: "्",
    digest: "89219f7da297ee79a81c00362e9ae29aa78189509a005bf838f27715347f2304",
    penLifts: 0,
    strokeOrder: ["start at the upper-left end and draw the stroke down to the right — and only now lift"],
    recognizerClass: 61,
  },
  {
    mark: "ृ",
    digest: "458b7c6d5ea989813d8e30378c692ef930e84b2c0674caebc9ef704e641c93c3",
    penLifts: 0,
    strokeOrder: ["start at the upper tip and curve to the left", "without lifting, round down and along the bottom", "without lifting, run out to the lower-right tip — and only now lift"],
    recognizerClass: 52,
  },
  {
    mark: "ँ",
    digest: "25ea031047c766528e5592a13f9e35b3ce3f038cac5626b23e8a5971497c3b45",
    penLifts: 1,
    strokeOrder: ["start at the left tip of the crescent and curve down", "without lifting, round the bottom and come up to the right tip", "lift, then dab the dot above the crescent — and only now lift"],
    recognizerClass: 59,
  },
  // Noto prints it with a piece of headline the native traces (two dots) do not draw.
  { mark: "ः", digest: "bd0255e27decb117bd71caddbee81858b9367691762bcf0dc7b927e4f8ebefe7" },
];

export const scriptInventoryEvidence = {
  name: "Devanagari marks",
  assert({ scripts }: ScriptEvidenceContext): void {
    const marks = scripts.devanagari!.marks ?? [];
    expect(marks.map((mark) => mark.mark)).toEqual(MARKS.map((entry) => entry.mark));
    for (const expected of MARKS) {
      const mark = marks.find((entry) => entry.mark === expected.mark) as Mark;
      expect(createHash("sha256").update(JSON.stringify(mark)).digest("hex"), mark.mark).toBe(
        expected.digest,
      );
      if (expected.strokeOrder === undefined) {
        expect(mark.strokeOrderSource, mark.mark).toBeUndefined();
        expect(mark.strokeOrder, mark.mark).toBeUndefined();
        continue;
      }
      expect(mark.penLifts, mark.mark).toBe(expected.penLifts);
      expect(mark.strokeOrder, mark.mark).toEqual(expected.strokeOrder);
      expect(mark.components?.length, mark.mark).toBeGreaterThan(0);
      expect(mark.strokeOrderNote, mark.mark).toMatch(
        new RegExp(`cited to native writers' pen traces in HP Labs India's LipiTk Devanagari recognizer, class ${expected.recognizerClass}$`),
      );
      expect(mark.strokeOrderSource?.url, mark.mark).toBe(LIPITK);
      expect(mark.strokeOrderSource?.citation, mark.mark).toContain(
        `Devanagari recognizer, class ${expected.recognizerClass} (${mark.mark}, `,
      );
      expect(mark.strokeOrderSource?.variation, mark.mark).toMatch(
        /^The recognizer stores native writers' tablet pen traces, each resampled to 60 points\. .*Each sign was written by itself, with no consonant beside it.*fitted to the bundled Noto Sans Devanagari outline of the sign on its own\..*research use only, so only counts and shares are cited here.*Handwriting varies by writer\.$/,
      );
      // The traces were written one sign at a time: no written-order claim,
      // except the nukta's carrier-first convention, cited to Unicode earlier.
      if (mark.mark === "\u093C") {
        expect(mark.compositionSource?.url).toMatch(/^https:\/\/www\.unicode\.org\//);
      } else {
        expect(mark.compositionOrder, mark.mark).toBeUndefined();
      }
    }
    // A sign whose foot meets the headline says it is drawn floating.
    const e = marks.find((entry) => entry.mark === "\u0947")!;
    expect(e.strokeOrderNote).toContain("drawn floating, without the headline its foot meets in a word");
  },
};
