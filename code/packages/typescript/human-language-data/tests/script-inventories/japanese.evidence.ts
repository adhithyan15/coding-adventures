// Exact real-corpus evidence owned by the Japanese inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Japanese",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const japaneseSmallTsu = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "っ",
    )!;
    expect(japaneseSmallTsu.sound).toMatch(
      /one mora.*closure.*doubles.*following consonant/i,
    );
    expect(japaneseSmallTsu.penLifts).toBe(0);
    expect(japaneseSmallTsu.strokeOrder).toEqual([
      "begin at the upper left and sweep right across the high shoulder",
      "without lifting, round down the right side and finish by sweeping left along the lower curve",
    ]);
    expect(japaneseSmallTsu.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*つ.*24 frames.*Unicode Standard 17\.0.*U\+3063.*small tsu/i,
    );
    expect(japaneseSmallTsu.strokeOrderSource?.variation).toMatch(
      /one uninterrupted run.*Unicode 17.*small tsu.*zero-lift.*scaling.*explicit.*independent handwriting evidence/i,
    );
    const japaneseShi = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "し",
    )!;
    expect(japaneseShi.sound).toBe("shi");
    expect(japaneseShi.role).toBe("hiragana");
    expect(japaneseShi.penLifts).toBe(0);
    expect(japaneseShi.strokeOrder).toEqual([
      "descend nearly straight from the top",
      "without lifting, turn around the broad lower curve and sweep upward to the right",
    ]);
    expect(japaneseShi.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*Hiragana し stroke order animation\.gif.*23 frames.*2\.3 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseShi.strokeOrderSource?.variation).toMatch(
      /one uninterrupted run.*descend from the top.*broad lower curve.*upward to the right.*Noto Sans JP.*zero-lift order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("し")).toBe(false);
    expect(affected.get("し") ?? 0).toBe(0);
    const japaneseKu = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "く",
    )!;
    expect(japaneseKu.sound).toBe("ku");
    expect(japaneseKu.role).toBe("hiragana");
    expect(japaneseKu.penLifts).toBe(0);
    expect(japaneseKu.strokeOrder).toEqual([
      "sweep down and left from the upper right into the central turn",
      "without lifting, sweep down and right to the lower tip",
    ]);
    expect(japaneseKu.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*Hiragana く stroke order animation\.gif.*20 frames.*2\.0 seconds.*Wikimedia Commons.*8 March 2010/i,
    );
    expect(japaneseKu.strokeOrderSource?.variation).toMatch(
      /one uninterrupted run.*upper right.*sharp central turn.*down and right.*lower tip.*Noto Sans JP.*zero-lift order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("く")).toBe(false);
    expect(affected.get("く") ?? 0).toBe(0);
    const japaneseTa = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "た",
    )!;
    expect(japaneseTa.sound).toBe("ta");
    expect(japaneseTa.role).toBe("hiragana");
    expect(japaneseTa.penLifts).toBe(3);
    expect(japaneseTa.strokeOrder).toEqual([
      "draw the upper horizontal from left to right",
      "lift and descend through the crossing stem, curving left at the foot",
      "lift and draw the short right horizontal from left to right",
      "lift and descend into the lower-right bowl, then sweep right along its base",
    ]);
    expect(japaneseTa.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*Hiragana た stroke order animation\.gif.*31 frames.*3\.1 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseTa.strokeOrderSource?.variation).toMatch(
      /four pen-down runs.*three lifts.*upper horizontal.*left-falling stem.*short right horizontal.*lower-right bowl.*Noto Sans JP.*four-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("た")).toBe(false);
    expect(affected.get("た") ?? 0).toBe(0);
    const japaneseNe = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "ね",
    )!;
    expect(japaneseNe.sound).toBe("ne");
    expect(japaneseNe.role).toBe("hiragana");
    expect(japaneseNe.penLifts).toBe(1);
    expect(japaneseNe.strokeOrder).toEqual([
      "descend through the short left vertical",
      "lift, then begin at the upper right, sweep left across the vertical, hook down along the diagonal and return to the crossing, then finish clockwise around the lower-right loop",
    ]);
    expect(japaneseNe.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*ね.*35 frames.*3\.5 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseNe.strokeOrderSource?.variation).toMatch(
      /two pen-down runs.*one lift.*short left vertical.*upper right.*cross left.*diagonal.*return.*clockwise.*lower-right loop.*Noto Sans JP.*two-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("ね")).toBe(false);
    expect(affected.get("ね") ?? 0).toBe(0);
    const japaneseMi = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "み",
    )!;
    expect(japaneseMi.sound).toBe("mi");
    expect(japaneseMi.role).toBe("hiragana");
    expect(japaneseMi.penLifts).toBe(1);
    expect(japaneseMi.strokeOrder).toEqual([
      "draw the top bar left to right, descend diagonally, continue around the lower-left loop, and sweep out through the middle",
      "lift, begin high on the right, and curve down and left before turning upward at the finish",
    ]);
    expect(japaneseMi.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*み.*29 frames.*2\.9 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseMi.strokeOrderSource?.variation).toMatch(
      /two pen-down runs.*one lift.*top bar.*lower-left loop.*sweep right through the middle.*high on the right.*curve down and left.*turning upward.*Noto Sans JP.*two-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("み")).toBe(false);
    expect(affected.get("み") ?? 0).toBe(0);
    const japaneseSe = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "せ",
    )!;
    expect(japaneseSe.sound).toBe("se");
    expect(japaneseSe.role).toBe("hiragana");
    expect(japaneseSe.penLifts).toBe(2);
    expect(japaneseSe.strokeOrder).toEqual([
      "draw the long crossing horizontal from left to right",
      "lift, begin above the left crossing, descend through it, and curve right along the base",
      "lift again, begin above the right crossing, descend through it, and hook left at the finish",
    ]);
    expect(japaneseSe.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*せ.*33 frames.*3\.3 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseSe.strokeOrderSource?.variation).toMatch(
      /three pen-down runs.*two lifts.*long horizontal.*left stem.*curving right.*right stem.*hooking left.*Noto Sans JP.*three-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("せ")).toBe(false);
    expect(affected.get("せ") ?? 0).toBe(0);
    const japaneseTe = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "て",
    )!;
    expect(japaneseTe.sound).toBe("te");
    expect(japaneseTe.role).toBe("hiragana");
    expect(japaneseTe.penLifts).toBe(0);
    expect(japaneseTe.strokeOrder).toEqual([
      "draw the high horizontal from left to right",
      "without lifting, turn back down and left through the diagonal",
      "without lifting, round the broad lower curve and sweep right to the finish",
    ]);
    expect(japaneseTe.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*て.*28 frames.*2\.8 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseTe.strokeOrderSource?.variation).toMatch(
      /one uninterrupted run.*high horizontal.*left to right.*down and left.*diagonal.*broad lower curve.*sweep right.*Noto Sans JP.*zero-lift order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("て")).toBe(false);
    expect(affected.get("て") ?? 0).toBe(0);
    const japaneseNa = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "な",
    )!;
    expect(japaneseNa.sound).toBe("na");
    expect(japaneseNa.role).toBe("hiragana");
    expect(japaneseNa.penLifts).toBe(3);
    expect(japaneseNa.strokeOrder).toEqual([
      "draw the upper-left horizontal from left to right",
      "lift and descend through the crossing left-falling stem",
      "lift and draw the short upper-right diagonal down and right",
      "lift, descend through the lower-right stem, turn around the loop, and sweep right to the finish",
    ]);
    expect(japaneseNa.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*な.*32 frames.*3\.2 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseNa.strokeOrderSource?.variation).toMatch(
      /four pen-down runs.*three lifts.*upper-left horizontal.*left-falling stem.*upper-right diagonal.*lower-right stem.*loop.*right.*Noto Sans JP.*four-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("な")).toBe(false);
    expect(affected.get("な") ?? 0).toBe(0);
    const japaneseMo = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "も",
    )!;
    expect(japaneseMo.sound).toBe("mo");
    expect(japaneseMo.role).toBe("hiragana");
    expect(japaneseMo.penLifts).toBe(2);
    expect(japaneseMo.strokeOrder).toEqual([
      "descend from the top and turn around the broad lower bowl to the rising right tip",
      "lift, then draw the upper horizontal from left to right across the stem",
      "lift again and draw the lower horizontal from left to right across the stem",
    ]);
    expect(japaneseMo.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*Hiragana も stroke order animation\.gif.*28 frames.*2\.8 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseMo.strokeOrderSource?.variation).toMatch(
      /three pen-down runs.*two lifts.*descending stem.*broad lower bowl.*upper and lower bars.*left to right.*Noto Sans JP.*three-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("も")).toBe(false);
    expect(affected.get("も") ?? 0).toBe(0);
    expect(missingByScript.get("japanese.json")?.has("っ")).toBe(false);
    expect(affected.get("っ") ?? 0).toBe(0);
    const japaneseWa = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "わ",
    )!;
    expect(japaneseWa.sound).toBe("wa");
    expect(japaneseWa.penLifts).toBe(1);
    expect(japaneseWa.strokeOrder).toEqual([
      "descend through the long left vertical",
      "lift, then begin at the upper left, sweep right across the vertical, hook down and left, turn back through the central crossing, and continue clockwise around the broad right loop",
    ]);
    expect(japaneseWa.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*わ.*30 frames.*3\.0 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseWa.strokeOrderSource?.variation).toMatch(
      /CC0.*two pen-down runs.*long left vertical.*cross right.*hook down and left.*central crossing.*clockwise.*right loop.*Noto Sans JP.*two-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("わ")).toBe(false);
    expect(affected.get("わ") ?? 0).toBe(0);
    const japaneseYu = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "ゆ",
    )!;
    expect(japaneseYu.sound).toBe("yu");
    expect(japaneseYu.penLifts).toBe(1);
    expect(japaneseYu.strokeOrder).toEqual([
      "descend through the left stem, turn up and right across the high shoulder, then continue clockwise around the broad loop and curve left to the inner finish",
      "lift, begin high above the loop, descend through its center, and curve down and left to the finish",
    ]);
    expect(japaneseYu.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*ゆ.*30 frames.*3\.0 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseYu.strokeOrderSource?.variation).toMatch(
      /CC0.*two pen-down runs.*left stem.*high shoulder.*clockwise.*broad loop.*inner finish.*above the loop.*center.*down-left curve.*Noto Sans JP.*two-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("ゆ")).toBe(false);
    expect(affected.get("ゆ") ?? 0).toBe(0);
    const japaneseYo = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "よ",
    )!;
    expect(japaneseYo.sound).toBe("yo");
    expect(japaneseYo.penLifts).toBe(1);
    expect(japaneseYo.strokeOrder).toEqual([
      "draw the short upper horizontal from left to right",
      "lift, begin above the horizontal, descend through it, then turn left and continue clockwise around the broad lower loop to the rightward finish",
    ]);
    expect(japaneseYo.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*よ.*26 frames.*2\.6 seconds.*Wikimedia Commons.*1 October 2009.*corrected.*4 January 2012/i,
    );
    expect(japaneseYo.strokeOrderSource?.variation).toMatch(
      /CC0.*two pen-down runs.*one lift.*corrected first stroke.*left to right.*descends through.*turns left.*clockwise.*lower loop.*rightward finish.*Noto Sans JP.*two-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("よ")).toBe(false);
    expect(affected.get("よ") ?? 0).toBe(0);
    const japaneseMe = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "め",
    )!;
    expect(japaneseMe.sound).toBe("me");
    expect(japaneseMe.penLifts).toBe(1);
    expect(japaneseMe.strokeOrder).toEqual([
      "descend from the upper left and curve down and right to the central finish",
      "lift, begin high near the center, descend diagonally left through the first stroke, loop around the lower left, sweep upward across the top, then continue clockwise around the broad right curve to the lower finish",
    ]);
    expect(japaneseMe.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*め.*32 frames.*3\.2 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseMe.strokeOrderSource?.variation).toMatch(
      /CC0.*two pen-down runs.*one lift.*left descending curve.*high central restart.*crosses the first stroke.*lower left.*across the top.*clockwise.*broad right curve.*lower finish.*Noto Sans JP.*two-run order/i,
    );
    expect(missingByScript.get("japanese.json")?.has("め")).toBe(false);
    expect(affected.get("め") ?? 0).toBe(0);
  },
};
