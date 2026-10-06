// Exact real-corpus evidence owned by the Gujarati inventory's marks.
// See HL24: unrelated script authors must not share an executable edit surface.
//
// Eleven Gujarati signs carry a cited stroke order (KanoAI's hand-made
// barakhadi templates: order, start, direction and lifts only, with the
// ambiguous MIT/GPL licence stated and no path data copied) and a cited
// written order: the consonant first, then the sign, even િ.
// figure-targets.ts' WRITTEN_SIGN_SIDES is held to these records in
// tests/figure-targets/gujarati-signs-in-written-order. The virama ્ and the
// vocalic-r sign ૃ have no Gujarati source and claim neither.

import { createHash } from "node:crypto";
import { expect } from "vitest";
import type { Mark } from "../../src/types.js";
import type { ScriptEvidenceContext } from "./helpers.js";

const KANO =
  "https://github.com/gajjartejas/KanoAI/tree/9d3e2949a3f265448e430e15329093ea3aa516b8/interpolate-svg/svgs/barakhadi";

interface SignEvidence {
  readonly mark: string;
  readonly digest: string;
  readonly penLifts?: number;
  readonly strokeOrder?: readonly string[];
  /** The KanoAI template file the citation names (the ka row). */
  readonly template?: string;
}

const MARKS: readonly SignEvidence[] = [
  {
    mark: "ા",
    digest: "3ef2db17af49bfe1747be8aa4e9c52da99ee1e66d2b136327e857508337a3d20",
    penLifts: 0,
    strokeOrder: ["start at the top of the bar and draw it straight down", "without lifting, turn into the short foot to the right — and only now lift"],
    template: "1_k/1_ka.svg",
  },
  {
    mark: "િ",
    digest: "9ccf656d917f3f15b653ed276a4137a3ecb121e8c4407197bd5f3ed064fc6bb7",
    penLifts: 0,
    strokeOrder: ["start at the right tip of the hook and curl up and over to the left", "without lifting, draw the stem down", "without lifting, turn into the short foot to the right — and only now lift"],
    template: "1_k/2_ki.svg",
  },
  {
    mark: "ી",
    digest: "d938eccbb51b279cbddea715954444058c6352cf49f5a50830a5df091d816012",
    penLifts: 0,
    strokeOrder: ["start at the left tip of the hook and arch up and over to the right", "without lifting, draw the stem down", "without lifting, turn into the short foot to the right — and only now lift"],
    template: "1_k/3_kee.svg",
  },
  {
    mark: "ુ",
    digest: "642d3781988ae774bfd9b2f1e6395b19cc9db6af3af9127448f705dbff1ff93c",
    penLifts: 0,
    strokeOrder: ["start directly under the consonant and swing to the right", "without lifting, round the bowl and come back to the left", "without lifting, sweep up to the left tip — and only now lift"],
    template: "1_k/4_ku.svg",
  },
  {
    mark: "ૂ",
    digest: "485cf5a9d0232ff1b39558ac7c8d53242f34cf560b07e1f2202623d88f7c08e0",
    penLifts: 0,
    strokeOrder: ["start at the low inner tip and curl to the left", "without lifting, climb and arch over the top", "without lifting, sweep down to the right — and only now lift"],
    template: "1_k/5_koo.svg",
  },
  {
    mark: "ે",
    digest: "859872f1ccbdba5cc65c6d2280d9a03ace6cc1b08a356a36bc0508632c2be245",
    penLifts: 0,
    strokeOrder: ["start at the upper-left tip and arc to the right", "without lifting, curve down to the right — and only now lift"],
    template: "1_k/6_ke.svg",
  },
  {
    mark: "ૈ",
    digest: "fa9f2f8c53d7b4e710e59a58608eeeb7024a65519c746513359020defbb1db52",
    penLifts: 1,
    strokeOrder: ["start at the lower flag's upper-left tip and arc it to the right", "without lifting, curve it down to the right", "lift, then start at the upper flag's upper-left tip and arc it to the right", "without lifting, curve it down to the right — and only now lift"],
    template: "1_k/7_kai.svg",
  },
  {
    mark: "ો",
    digest: "25162a5a89bba17d7e9066528063bca0807cff03611575ce9882d962b82551e4",
    penLifts: 1,
    strokeOrder: ["start at the top of the bar and draw it straight down", "without lifting, turn into the short foot to the right", "lift, then start at the flag's upper-left tip and arc it to the right", "without lifting, curve it down to the right — and only now lift"],
    template: "1_k/8_ko.svg",
  },
  {
    mark: "ૌ",
    digest: "c252cda30a03373b602f7d65a0195c5fe29243f3bb0211786bc67f6747c19c78",
    penLifts: 2,
    strokeOrder: ["start at the top of the bar and draw it straight down", "without lifting, turn into the short foot to the right", "lift, then start at the lower flag's upper-left tip and arc it to the right", "without lifting, curve it down to the right", "lift, then start at the upper flag's upper-left tip and arc it to the right", "without lifting, curve it down to the right — and only now lift"],
    template: "1_k/9_kau.svg",
  },
  {
    mark: "ં",
    digest: "649ecb2e9e7774e5909811d583225ed7a44765a7c6d56faacfd796ac889fbe61",
    penLifts: 0,
    strokeOrder: ["start at the top of the dot and curve down its left side", "without lifting, round the bottom and come up the right side to close the loop — and only now lift"],
    template: "1_k/10_kam.svg",
  },
  {
    mark: "ઃ",
    digest: "4aa8d2fb21b63bf05e076d4fe75ee27e4852550cfe9dabda472952d80f6f0629",
    penLifts: 1,
    strokeOrder: ["start at the top of the upper dot and curve down its left side", "without lifting, round its bottom and up its right side to close it", "lift, then start at the top of the lower dot and curve down its left side", "without lifting, round its bottom and up its right side to close it — and only now lift"],
    template: "1_k/11_kah.svg",
  },
  { mark: "્", digest: "b7af294522016a7ff77344b2943de61337a4c023d23d3660ef937d749c5cc5ae" },
  { mark: "ૃ", digest: "9dc0da60a0b3dc3c68911a65096f8f92cd4bac52d07ebee6a3bfeae1f9a84dbf" },
];

export const scriptInventoryEvidence = {
  name: "Gujarati marks",
  assert({ scripts }: ScriptEvidenceContext): void {
    const marks = scripts.gujarati!.marks ?? [];
    expect(marks.map((mark) => mark.mark)).toEqual(MARKS.map((entry) => entry.mark));
    for (const expected of MARKS) {
      const mark = marks.find((entry) => entry.mark === expected.mark) as Mark;
      expect(createHash("sha256").update(JSON.stringify(mark)).digest("hex"), mark.mark).toBe(
        expected.digest,
      );
      if (expected.strokeOrder === undefined) {
        // No Gujarati source: no ductus claim and no written-order claim.
        expect(mark.strokeOrderSource, mark.mark).toBeUndefined();
        expect(mark.compositionOrder, mark.mark).toBeUndefined();
        continue;
      }
      expect(mark.penLifts, mark.mark).toBe(expected.penLifts);
      expect(mark.strokeOrder, mark.mark).toEqual(expected.strokeOrder);
      expect(mark.components?.length, mark.mark).toBeGreaterThan(0);
      expect(mark.strokeOrderNote, mark.mark).toMatch(/cited to KanoAI's hand-made Gujarati barakhadi templates$/);
      expect(mark.strokeOrderSource?.url, mark.mark).toBe(KANO);
      expect(mark.strokeOrderSource?.citation, mark.mark).toContain(
        `Tejas Gajjar, KanoAI, Gujarati barakhadi centre-line stroke templates (interpolate-svg/svgs/barakhadi), commit 9d3e294, ${expected.template}, `,
      );
      expect(mark.strokeOrderSource?.citation, mark.mark).toMatch(/: the .* drawn after the consonant \(GitHub, 2026\)$/);
      expect(mark.strokeOrderSource?.variation, mark.mark).toMatch(
        /consonant rows.*KanoAI's templates are hand-made centre lines.*LICENSE file says MIT while its README says GNU GPL, so the licence is ambiguous.*no template path data was copied.*fitted to the bundled Noto Sans Gujarati outline of the sign on its own/,
      );
      // Written position: the consonant first, the sign after it.
      expect(mark.compositionOrder?.[0], mark.mark).toBe("write the Gujarati consonant first");
      expect(mark.compositionOrder?.[1], mark.mark).toMatch(/^write the .* after it/);
      expect(mark.compositionSource?.url, mark.mark).toBe(KANO);
      expect(mark.compositionSource?.variation, mark.mark).toMatch(
        /^Of the 34 consonant rows \(ક to જ્ઞ\), \d+ draw the consonant with the bare consonant's own outline.*the consonant is written first/,
      );
    }
    // The one sign that sits LEFT of its consonant is still written after it.
    const i = marks.find((entry) => entry.mark === "િ")!;
    expect(i.compositionOrder).toEqual([
      "write the Gujarati consonant first",
      "write the i sign after it, although it sits to the left of the consonant",
    ]);
    expect(i.compositionSource?.variation).toMatch(/in 33 of those 34 the consonant is written first.*ઢિ, lists the sign's group first/);
  },
};
