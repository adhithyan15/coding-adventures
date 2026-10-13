// Exact real-corpus evidence owned by the Devanagari inventory's marks.
// See HL24: unrelated script authors must not share an executable edit surface.
//
// Twelve Devanagari signs carry a cited stroke order: native writers' tablet
// pen traces in HP Labs India's LipiTk Devanagari recognizer (MIT-licensed
// model; the underlying data is research-only, so only counts and shares are
// cited). The writers wrote each sign alone, so the traces never say where
// the sign is written against its consonant or the headline, and the
// composer's WRITTEN_SIGN_SIDES has no Devanagari row
// (tests/figure-targets/devanagari-signs-drawn-alone). ā alone cites that
// place separately, to the cited आ (its compositionSource), which is what
// lets it join a word composed with one shared headline. ी, ो and ः are
// drawn without the piece of headline Noto prints on them (it is the word's
// headline, drawn last across the whole word; script-ductus excuses exactly
// that stub in its coverage check), and their records say so. The other
// three signs claim nothing; each says why below.

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
  // The stem, then the piece of headline Noto prints on it (23 of 81 writers draw that top stroke too).
  {
    mark: "ा",
    digest: "f4bca6ec5afd13afb4b4725dc6d8c45f52f00f72efa54251d3f8d7b3ddec8d81",
    penLifts: 1,
    strokeOrder: ["start at the top of the stem and draw it straight down", "lift, then draw the piece of headline left to right — and only now lift"],
    recognizerClass: 47,
  },
  // No majority: of 75 writers, 31 draw the stem, lift, then the arch; 22 one
  // run from the arch's right tip down the stem; 20 one run up the stem and over.
  { mark: "ि", digest: "ff2a4b8e5a32f6c2ace602bc5183abc3be40a8c4e72c51e74b3e16cdf6713a0c" },
  // One run, arch first (49 of 91 arch first, 44 of them unbroken); the stub is left undrawn.
  {
    mark: "ी",
    digest: "ae5e1347c3a167ef52af8d34d7d47c99a7906866c25079b1dbf303b89842f30b",
    penLifts: 0,
    strokeOrder: ["start at the lower tip of the hook and climb its left side", "without lifting, arch over the top and down to the right", "without lifting, draw the stem straight down — and only now lift"],
    recognizerClass: 49,
  },
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
  // The stem down, a lift, then the flag from its upper-left tip (58 of 83 stem then
  // a lift; 41 of those start the flag at its tip); the stub is left undrawn.
  {
    mark: "ो",
    digest: "c3e7214b0a438b430497c95c7d4ee5019c7ea86e38a18c6e4c5c0981211f4b9e",
    penLifts: 1,
    strokeOrder: ["start at the top of the stem and draw it straight down", "lift, then start at the flag's upper-left tip and arc to the right", "without lifting, curve down to the top of the stem — and only now lift"],
    recognizerClass: 55,
  },
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
  // Two loops, upper dot first (77 of 81 two strokes, 76 of them upper first); the stub is left undrawn.
  {
    mark: "ः",
    digest: "2535f443c6e4b5e4ec601e5b171ab934daaad5160b80399204600cf783737241",
    penLifts: 1,
    strokeOrder: ["start at the top of the upper dot and curve down its left side", "without lifting, round the bottom and up the right side to close the loop", "lift, then start at the top of the lower dot and curve down its left side", "without lifting, round the bottom and up the right side to close the loop — and only now lift"],
    recognizerClass: 60,
  },
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
      // except the nukta's carrier-first convention, cited to Unicode earlier,
      // and ā's place in a word, cited to the cited आ.
      if (mark.mark === "\u093C") {
        expect(mark.compositionSource?.url).toMatch(/^https:\/\/www\.unicode\.org\//);
      } else if (mark.mark === "\u093E") {
        expect(mark.compositionSource?.url).toBe(
          "https://commons.wikimedia.org/wiki/File:Devanagari_%E0%A4%86_stroke_order.svg",
        );
      } else {
        expect(mark.compositionOrder, mark.mark).toBeUndefined();
      }
    }
    // The three signs drawn without the stub Noto prints on them say so, and why.
    for (const glyph of ["\u0940", "\u094B", "\u0903"]) {
      const mark = marks.find((entry) => entry.mark === glyph)!;
      expect(mark.strokeOrderNote, glyph).toContain(
        "the short piece of headline the printed sign carries is left undrawn, because in a word it is part of the one headline drawn last across the whole word",
      );
    }
    // A sign whose foot meets the headline says it is drawn floating.
    const e = marks.find((entry) => entry.mark === "\u0947")!;
    expect(e.strokeOrderNote).toContain("drawn floating, without the headline its foot meets in a word");
  },
};
