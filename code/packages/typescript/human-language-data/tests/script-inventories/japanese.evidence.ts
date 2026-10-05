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
    const missingJapanese = missingByScript.get("japanese.json") ?? new Set<string>();
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
    expect(missingJapanese.has("し")).toBe(false);
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
    expect(missingJapanese.has("く")).toBe(false);
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
    expect(missingJapanese.has("た")).toBe(false);
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
    expect(missingJapanese.has("ね")).toBe(false);
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
    expect(missingJapanese.has("み")).toBe(false);
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
    expect(missingJapanese.has("せ")).toBe(false);
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
    expect(missingJapanese.has("て")).toBe(false);
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
    expect(missingJapanese.has("な")).toBe(false);
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
    expect(missingJapanese.has("も")).toBe(false);
    expect(affected.get("も") ?? 0).toBe(0);
    expect(missingJapanese.has("っ")).toBe(false);
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
    expect(missingJapanese.has("わ")).toBe(false);
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
    expect(missingJapanese.has("ゆ")).toBe(false);
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
    expect(missingJapanese.has("よ")).toBe(false);
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
    expect(missingJapanese.has("め")).toBe(false);
    expect(affected.get("め") ?? 0).toBe(0);
    const japaneseTsu = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "つ",
    )!;
    expect(japaneseTsu.sound).toBe("tsu");
    expect(japaneseTsu.role).toBe("hiragana");
    expect(japaneseTsu.penLifts).toBe(0);
    expect(japaneseTsu.strokeOrder).toEqual([
      "begin at the upper left, sweep right across the high arch, curve down around the outer right side, then return left along the broad lower curve to the finish",
    ]);
    expect(japaneseTsu.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*つ.*24 frames.*2\.4 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseTsu.strokeOrderSource?.variation).toMatch(
      /CC0.*one uninterrupted run.*upper left.*high arch.*outer right curve.*back left.*broad lower return.*Noto Sans JP.*zero-lift order/i,
    );
    expect(missingJapanese.has("つ")).toBe(false);
    expect(affected.get("つ") ?? 0).toBe(0);
    // HL-C360: the two signs the cardinals one to ten cost. ろ was observed
    // frame by frame from the cited animation -- the start marker never leaves
    // the upper-left origin across all 26 frames, so the run is single and
    // penLifts is 0. ゅ claims NO independent handwriting evidence: it reuses
    // ゆ's observed two-run movement and says so, exactly as っ reuses つ's.
    const japaneseRo = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "ろ",
    )!;
    expect(japaneseRo.sound).toBe("ro");
    expect(japaneseRo.role).toBe("hiragana");
    expect(japaneseRo.penLifts).toBe(0);
    expect(japaneseRo.strokeOrder).toEqual([
      "begin at the upper left and draw the short high shoulder to the right, turning down at the corner",
      "without lifting, descend the long diagonal down and left toward the lower centre",
      "without lifting, swing right into the broad clockwise belly and finish with a short tail curving left at the bottom",
    ]);
    expect(japaneseRo.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*Hiragana ろ stroke order animation\.gif.*26 frames.*2\.6 seconds.*Wikimedia Commons.*1 October 2009/i,
    );
    expect(japaneseRo.strokeOrderSource?.variation).toMatch(
      /CC0.*one uninterrupted run.*high shoulder.*long diagonal.*clockwise belly.*left tail.*Noto Sans JP.*zero-lift order/i,
    );
    expect(missingJapanese.has("ろ")).toBe(false);
    expect(affected.get("ろ") ?? 0).toBe(0);
    const japaneseSmallYu = scripts.japanese!.letters.find(
      (entry) => entry.glyph === "ゅ",
    )!;
    expect(japaneseSmallYu.sound).toMatch(
      /small yu.*joins.*preceding sign.*one mora/i,
    );
    expect(japaneseSmallYu.role).toBe("hiragana");
    expect(japaneseSmallYu.penLifts).toBe(1);
    expect(japaneseSmallYu.strokeOrderSource?.citation).toMatch(
      /Sirgazil.*ゆ.*30 frames.*U\+3085 HIRAGANA LETTER SMALL YU/i,
    );
    expect(japaneseSmallYu.strokeOrderSource?.variation).toMatch(
      /two pen-down runs.*one lift.*U\+3085.*small yu.*scaling.*explicit.*independent handwriting evidence/i,
    );
    expect(missingJapanese.has("ゅ")).toBe(false);
    expect(affected.get("ゅ") ?? 0).toBe(0);
    // Chapter 131: small ゃ and small ょ follow the ゅ rule exactly -- the
    // full-size sign's observed movement, its citation, and an explicit
    // statement that the size adaptation is not independent evidence. Both also
    // name KanjiVG's own file for the small code point, which holds the same
    // strokes in the same order. を has no Sirgazil animation in the record: its
    // order and direction come from KanjiVG's three directed paths, and the
    // variation says the coordinates are the font's, not KanjiVG's.
    //
    // Phrase by phrase with toContain rather than one regex with greedy gaps,
    // for the backtracking reason recorded in script-ductus's japanese test.
    for (const [glyph, full, name, lifts] of [
      ["ゃ", "や", "SMALL YA", 2],
      ["ょ", "よ", "SMALL YO", 1],
    ] as const) {
      const small = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      const fullSize = scripts.japanese!.letters.find((entry) => entry.glyph === full)!;
      expect(small.role).toBe("hiragana");
      expect(small.sound).toContain("joins to the preceding sign and makes one mora");
      expect(small.penLifts).toBe(lifts);
      expect(small.penLifts).toBe(fullSize.penLifts);
      expect(small.strokeOrder).toHaveLength(fullSize.strokeOrder.length);
      expect(small.strokeOrderSource?.url).toBe(fullSize.strokeOrderSource?.url);
      for (const phrase of ["Sirgazil", `Hiragana ${full} stroke order animation.gif`, "KanjiVG", `HIRAGANA LETTER ${name}`]) {
        expect(small.strokeOrderSource?.citation, phrase).toContain(phrase);
      }
      for (const phrase of ["scaling", "explicit rather than presented as independent handwriting evidence"]) {
        expect(small.strokeOrderSource?.variation, phrase).toContain(phrase);
      }
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    const japaneseWo = scripts.japanese!.letters.find((entry) => entry.glyph === "を")!;
    expect(japaneseWo.role).toBe("hiragana");
    expect(japaneseWo.sound).toContain("only the particle");
    expect(japaneseWo.penLifts).toBe(2);
    expect(japaneseWo.strokeOrder).toHaveLength(3);
    expect(japaneseWo.strokeOrderSource?.url).toBe(
      "https://github.com/KanjiVG/kanjivg/blob/master/kanji/03092.svg",
    );
    for (const phrase of ["KanjiVG", "U+3092 HIRAGANA LETTER WO", "CC BY-SA 3.0"]) {
      expect(japaneseWo.strokeOrderSource?.citation, phrase).toContain(phrase);
    }
    for (const phrase of ["Only the order and direction", "Noto Sans JP outline's own medial line"]) {
      expect(japaneseWo.strokeOrderSource?.variation, phrase).toContain(phrase);
    }
    expect(missingJapanese.has("を")).toBe(false);
    expect(affected.get("を") ?? 0).toBe(0);
    // Chapter 132: そ, れ and る take their order and direction from KanjiVG's
    // directed paths, as を does, and say that the coordinates are the font's.
    // Their lift counts are KanjiVG's path counts minus one: そ and る are one
    // path each, れ is two. Phrase by phrase with toContain, as above.
    for (const [glyph, file, name, lifts] of [
      ["そ", "0305d", "U+305D HIRAGANA LETTER SO", 0],
      ["れ", "0308c", "U+308C HIRAGANA LETTER RE", 1],
      ["る", "0308b", "U+308B HIRAGANA LETTER RU", 0],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.penLifts).toBe(lifts);
      expect(letter.strokeOrderSource?.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of ["KanjiVG", name, "Ulrich Apel and contributors, CC BY-SA 3.0"]) {
        expect(letter.strokeOrderSource?.citation, phrase).toContain(phrase);
      }
      for (const phrase of ["Only the order and direction", "Noto Sans JP outline's own medial line"]) {
        expect(letter.strokeOrderSource?.variation, phrase).toContain(phrase);
      }
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    // Chapter 133: き, け, ぬ and へ, the last four basic hiragana, are cited
    // the same way. KanjiVG has four paths for き, three for け, two for ぬ and
    // one for へ, so the lift counts are 3, 2, 1 and 0. ら, written since
    // chapter 8 but missing from the inventory until chapter 133, is cited the
    // same way: two paths, one lift. Phrase by phrase with toContain, as above.
    for (const [glyph, file, name, lifts] of [
      ["き", "0304d", "U+304D HIRAGANA LETTER KI", 3],
      ["け", "03051", "U+3051 HIRAGANA LETTER KE", 2],
      ["ぬ", "0306c", "U+306C HIRAGANA LETTER NU", 1],
      ["へ", "03078", "U+3078 HIRAGANA LETTER HE", 0],
      ["ら", "03089", "U+3089 HIRAGANA LETTER RA", 1],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.penLifts).toBe(lifts);
      expect(letter.strokeOrderSource?.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of ["KanjiVG", name, "Ulrich Apel and contributors, CC BY-SA 3.0"]) {
        expect(letter.strokeOrderSource?.citation, phrase).toContain(phrase);
      }
      for (const phrase of ["Only the order and direction", "Noto Sans JP outline's own medial line"]) {
        expect(letter.strokeOrderSource?.variation, phrase).toContain(phrase);
      }
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    // あ, い, う, え, お and か have been written since chapters 1, 3 and 10,
    // but their rows said only "authoritative" and cited nothing, so they had
    // no ductus and their writing lessons printed no filmstrip. They now cite
    // KanjiVG the same way: three paths for あ, two each for い, う and え,
    // three each for お and か, so the lift counts are 2, 1, 1, 1, 2 and 2.
    // Phrase by phrase with toContain, as above.
    for (const [glyph, file, name, lifts] of [
      ["あ", "03042", "U+3042 HIRAGANA LETTER A", 2],
      ["い", "03044", "U+3044 HIRAGANA LETTER I", 1],
      ["う", "03046", "U+3046 HIRAGANA LETTER U", 1],
      ["え", "03048", "U+3048 HIRAGANA LETTER E", 1],
      ["お", "0304a", "U+304A HIRAGANA LETTER O", 2],
      ["か", "0304b", "U+304B HIRAGANA LETTER KA", 2],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.penLifts).toBe(lifts);
      expect(letter.strokeOrderNote).not.toBe("authoritative");
      expect(letter.strokeOrderSource?.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of ["KanjiVG", name, "Ulrich Apel and contributors, CC BY-SA 3.0"]) {
        expect(letter.strokeOrderSource?.citation, phrase).toContain(phrase);
      }
      for (const phrase of ["Only the order and direction", "Noto Sans JP outline's own medial line"]) {
        expect(letter.strokeOrderSource?.variation, phrase).toContain(phrase);
      }
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    // こ, さ, す, ち and と have been written since chapters 2 to 4, but their
    // rows said only "authoritative" and cited nothing, so they had no ductus
    // and their writing lessons printed no filmstrip. They now cite KanjiVG
    // the same way: two paths each for こ, す, ち and と and three for さ, so
    // the lift counts are 1, 2, 1, 1 and 1. Phrase by phrase with toContain,
    // as above.
    for (const [glyph, file, name, lifts] of [
      ["こ", "03053", "U+3053 HIRAGANA LETTER KO", 1],
      ["さ", "03055", "U+3055 HIRAGANA LETTER SA", 2],
      ["す", "03059", "U+3059 HIRAGANA LETTER SU", 1],
      ["ち", "03061", "U+3061 HIRAGANA LETTER TI", 1],
      ["と", "03068", "U+3068 HIRAGANA LETTER TO", 1],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.penLifts).toBe(lifts);
      expect(letter.strokeOrderNote).not.toBe("authoritative");
      expect(letter.strokeOrderSource?.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of ["KanjiVG", name, "Ulrich Apel and contributors, CC BY-SA 3.0"]) {
        expect(letter.strokeOrderSource?.citation, phrase).toContain(phrase);
      }
      for (const phrase of ["Only the order and direction", "Noto Sans JP outline's own medial line"]) {
        expect(letter.strokeOrderSource?.variation, phrase).toContain(phrase);
      }
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    // に, は, ま, り and ん, written since chapters 1 to 4, were the last five
    // basic hiragana whose rows said only "authoritative". They now cite
    // KanjiVG the same way: three paths each for に, は and ま, two for り and
    // one for ん, so the lift counts are 2, 2, 2, 1 and 0. Phrase by phrase
    // with toContain, as above. After them no Japanese row says only
    // "authoritative" for any of the 46 basic hiragana, あ to ん with を.
    for (const [glyph, file, name, lifts] of [
      ["に", "0306b", "U+306B HIRAGANA LETTER NI", 2],
      ["は", "0306f", "U+306F HIRAGANA LETTER HA", 2],
      ["ま", "0307e", "U+307E HIRAGANA LETTER MA", 2],
      ["り", "0308a", "U+308A HIRAGANA LETTER RI", 1],
      ["ん", "03093", "U+3093 HIRAGANA LETTER N", 0],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.penLifts).toBe(lifts);
      expect(letter.strokeOrderNote).not.toBe("authoritative");
      expect(letter.strokeOrderSource?.url).toBe(
        `https://github.com/KanjiVG/kanjivg/blob/master/kanji/${file}.svg`,
      );
      for (const phrase of ["KanjiVG", name, "Ulrich Apel and contributors, CC BY-SA 3.0"]) {
        expect(letter.strokeOrderSource?.citation, phrase).toContain(phrase);
      }
      for (const phrase of ["Only the order and direction", "Noto Sans JP outline's own medial line"]) {
        expect(letter.strokeOrderSource?.variation, phrase).toContain(phrase);
      }
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    for (const glyph of "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをん") {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph);
      expect(letter?.strokeOrderSource, glyph).toBeDefined();
      expect(letter?.strokeOrderNote, glyph).not.toBe("authoritative");
    }
    // Chapters 134 and 135: eight voiced kana, each recorded the way が, ご, ざ
    // and ぼ already are -- the base sign plus the dakuten, written as the base
    // sign in full and then the two short strokes at the upper right. None
    // cites a stroke-order source of its own, so none has a ductus, and the
    // record does not pretend otherwise (no penLifts, no strokeOrderSource).
    // The precomposed glyph is what script closure counts, so each must also
    // be covered, not merely decomposable.
    for (const [glyph, base, sound] of [
      ["で", "て", "de"], ["ば", "は", "ba"], ["べ", "へ", "be"], ["ぶ", "ふ", "bu"],
      ["び", "ひ", "bi"], ["ぐ", "く", "gu"], ["げ", "け", "ge"], ["ぎ", "き", "gi"],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.sound).toBe(sound);
      expect(letter.components).toEqual([base, "the dakuten ゛"]);
      expect(letter.strokeOrder).toEqual([`${base} in full`, "two short strokes at the upper right"]);
      expect(letter.strokeOrderSource).toBeUndefined();
      expect(scripts.japanese!.letters.some((entry) => entry.glyph === base)).toBe(true);
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
    // Chapters 136 and 137: the rest of the z row and the whole p row. ぞ, ず
    // and ぜ are recorded as ざ is (the base sign plus the dakuten). ぱ, ぴ, ぷ
    // and ぺ are recorded as ぽ is: the base sign plus the handakuten, written
    // as the base sign in full and then a small circle at the upper right. As
    // above, none cites a stroke-order source, and each precomposed glyph must
    // be covered in its own right.
    for (const [glyph, base, sound, mark, last] of [
      ["ぞ", "そ", "zo", "the dakuten ゛", "two short strokes at the upper right"],
      ["ず", "す", "zu", "the dakuten ゛", "two short strokes at the upper right"],
      ["ぜ", "せ", "ze", "the dakuten ゛", "two short strokes at the upper right"],
      ["ぱ", "は", "pa", "the handakuten ゜", "a small circle at the upper right"],
      ["ぴ", "ひ", "pi", "the handakuten ゜", "a small circle at the upper right"],
      ["ぷ", "ふ", "pu", "the handakuten ゜", "a small circle at the upper right"],
      ["ぺ", "へ", "pe", "the handakuten ゜", "a small circle at the upper right"],
    ] as const) {
      const letter = scripts.japanese!.letters.find((entry) => entry.glyph === glyph)!;
      expect(letter.role).toBe("hiragana");
      expect(letter.sound).toBe(sound);
      expect(letter.components).toEqual([base, mark]);
      expect(letter.strokeOrder).toEqual([`${base} in full`, last]);
      expect(letter.strokeOrderSource).toBeUndefined();
      expect(scripts.japanese!.letters.some((entry) => entry.glyph === base)).toBe(true);
      expect(missingJapanese.has(glyph)).toBe(false);
      expect(affected.get(glyph) ?? 0).toBe(0);
    }
  },
};
