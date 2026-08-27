// Exact real-corpus evidence owned by the Urdu Nastaliq inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Urdu Nastaliq",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const urduDal = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "د",
    )!;
    expect(urduDal.strokeOrder).toEqual([
      "begin at the independent form's upper tip and descend through the folded shoulder",
      "without lifting, turn left along the baseline",
    ]);
    expect(urduDal.penLifts).toBe(0);
    expect(urduDal.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/dal-re-and-waw/",
    );
    expect(urduDal.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent د.*Dāl instructions/i,
    );
    expect(urduDal.strokeOrderSource?.variation).toMatch(
      /one uninterrupted stroke.*folded shoulder.*leftward baseline.*90-degree angle.*does not drop below.*Naskh.*Nastaliq.*Urdu-specific/i,
    );
    const urduWaw = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "و",
    )!;
    expect(urduWaw.strokeOrder).toEqual([
      "shape the independent wāw's looped head",
      "continue down and left through the tail without lifting",
    ]);
    expect(urduWaw.penLifts).toBe(0);
    expect(urduWaw.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/dal-re-and-waw/",
    );
    expect(urduWaw.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent و.*Wāw instructions.*Northwestern University Libraries Open Textbook.*2026/i,
    );
    expect(urduWaw.strokeOrderSource?.variation).toMatch(
      /one uninterrupted stroke.*head as a loop.*tail without lifting.*nonconnector.*v\/w.*o, au, and ū.*Noto Naskh fallback.*Nastaliq.*Urdu-specific/i,
    );
    const urduFe = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "ف",
    )!;
    expect(urduFe.strokeOrder).toEqual([
      "loop clockwise around the rounded head above the main line",
      "continue left through the shallow curved tail without lifting",
      "after one lift, place the single dot above",
    ]);
    expect(urduFe.penLifts).toBe(1);
    const urduQaf = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "ق",
    )!;
    expect(urduQaf.penLifts).toBe(2);
    expect(urduQaf.strokeOrder).toEqual([
      "loop clockwise around the rounded head above the main line",
      "continue down and left through the deep bowl without lifting",
      "after one lift, place the upper-right dot",
      "after another lift, place the upper-left dot",
    ]);
    expect(urduQaf.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
    const persianQaf = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "ق",
    )!;
    expect(persianQaf.strokeOrderSource?.url).not.toBe(
      urduQaf.strokeOrderSource?.url,
    );
    const urduToe = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "ط",
    )!;
    expect(urduToe.penLifts).toBe(1);
    expect(urduToe.strokeOrder).toEqual([
      "draw the independent to'e-series loop and its leftward finish",
      "after one lift, draw the tall upright",
    ]);
    expect(urduToe.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/toe-zoe-se-zhe-ghain/",
    );
    const persianTah = scripts["perso-arabic"]!.letters.find(
      (letter) => letter.glyph === "ط",
    )!;
    expect(persianTah.strokeOrderSource?.url).not.toBe(
      urduToe.strokeOrderSource?.url,
    );
    expect(urduFe.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
    expect(urduFe.strokeOrderSource?.variation).toMatch(
      /clockwise.*above the main line.*shallow curved tail.*lift.*dot.*looped head.*Noto Naskh fallback.*Nastaliq.*Urdu-specific/i,
    );
    const urduBariHe = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "ح",
    )!;
    expect(urduBariHe.sound).toBe("h");
    expect(urduBariHe.penLifts).toBe(0);
    expect(urduBariHe.strokeOrder).toEqual([
      "sweep left through the pointed hooked head",
      "continue down and around the deep bowl without lifting",
    ]);
    expect(urduBariHe.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/sin-shin-bari-he-nun-nun-ghunna/",
    );
    expect(urduBariHe.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*Sīn.*shīn.*baṛī he.*independent ح handwriting animation.*Northwestern.*2026/i,
    );
    expect(urduBariHe.strokeOrderSource?.variation).toMatch(
      /pointed hooked head.*deep bowl.*one uninterrupted body-first stroke.*Arabic-derived words.*Noto Naskh fallback.*Nastaliq.*Urdu-specific/i,
    );
    const urduBe = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "ب",
    )!;
    expect(urduBe.sound).toBe("b");
    expect(urduBe.penLifts).toBe(1);
    expect(urduBe.strokeOrder).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the single dot below",
    ]);
    expect(urduBe.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/be-kaf-and-short-vowels/",
    );
    expect(urduBe.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent ب.*Be instructions.*Northwestern/i,
    );
    expect(urduBe.strokeOrderSource?.variation).toMatch(
      /bowl first.*right-to-left.*single dot below.*one lift.*shallow curve.*main line.*dots.*Noto Naskh fallback.*Nastaliq.*Urdu-specific/i,
    );
    const urduPe = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "پ",
    )!;
    expect(urduPe.sound).toBe("p");
    expect(urduPe.penLifts).toBe(3);
    expect(urduPe.strokeOrder).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the lower-left dot nearer the main line",
      "after another lift, place the lower-right dot nearer the main line",
      "after a third lift, place the lower-center dot",
    ]);
    expect(urduPe.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
    );
    expect(urduPe.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent پ.*Pe instructions.*Northwestern/i,
    );
    expect(urduPe.strokeOrderSource?.variation).toMatch(
      /bowl first.*right-to-left.*lower-left dot.*lower-right dot.*lower-center dot.*three pen lifts.*triangular arrangement.*two-dot side.*main line.*Noto Naskh fallback.*Nastaliq.*Urdu-specific/i,
    );
    const urduTte = scripts["urdu-nastaliq"]!.letters.find(
      (letter) => letter.glyph === "ٹ",
    )!;
    expect(urduTte.sound).toBe("ṭ");
    expect(urduTte.penLifts).toBe(1);
    expect(urduTte.strokeOrder).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, draw the small retroflex mark downward, back upward, and down again to close its loop",
    ]);
    expect(urduTte.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/fe-qaf-te-dal-re/",
    );
    expect(urduTte.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent ٹ.*Ṭe instructions.*Northwestern/i,
    );
    expect(urduTte.strokeOrderSource?.variation).toMatch(
      /be-series body.*upper retroflex mark.*two pen-down runs.*dental te.*small to'e-shaped mark.*down.*back up.*down again.*loop.*body-first.*one-lift.*Noto Naskh fallback.*Nastaliq.*Urdu retroflex/i,
    );
    expect(missingByScript.get("urdu-nastaliq.json")?.has("د")).toBe(false);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("و")).toBe(false);
    expect(affected.get("و") ?? 0).toBe(0);
    expect(affected.get("د") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("پ")).toBe(false);
    expect(affected.get("پ") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ھ")).toBe(false);
    expect(affected.get("ھ") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("چ")).toBe(false);
    expect(affected.get("چ") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ٓ")).toBe(false);
    expect(affected.get("ٓ") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("خ")).toBe(false);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ف")).toBe(false);
    expect(affected.get("ف") ?? 0).toBe(0);
    const urduGaf = scripts["urdu-nastaliq"]!.letters.find(
      (entry) => entry.glyph === "گ",
    )!;
    expect(urduGaf.sound).toBe("g");
    expect(urduGaf.penLifts).toBe(2);
    expect(urduGaf.strokeOrder).toEqual([
      "draw the independent stem downward",
      "flow right to left through the flatter bowl and finish with the pronounced hook without lifting",
      "after one lift, draw the long slash down from the upper right toward the stem",
      "after another lift, draw the shorter floating slash above the first",
    ]);
    expect(urduGaf.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent گ.*calligraphic and handwriting animations.*Gāf instructions.*Northwestern/i,
    );
    expect(urduGaf.strokeOrderSource?.variation).toMatch(
      /kāf-family main-line body first.*long downward slash.*shorter floating slash.*two pen lifts.*three-run order.*Noto Naskh.*Nastaliq/i,
    );
    expect(missingByScript.get("urdu-nastaliq.json")?.has("گ")).toBe(false);
    expect(affected.get("گ") ?? 0).toBe(0);
    const urduTe = scripts["urdu-nastaliq"]!.letters.find(
      (entry) => entry.glyph === "ت",
    )!;
    expect(urduTe.sound).toBe("t");
    expect(urduTe.penLifts).toBe(2);
    expect(urduTe.strokeOrder).toEqual([
      "sweep the independent be-series bowl from right to left",
      "after one lift, place the left dot above the main line",
      "after another lift, place the right dot beside it",
    ]);
    expect(urduTe.strokeOrderSource?.citation).toMatch(
      /Zer o Zabar.*independent ت.*handwriting animation.*Te instructions.*Northwestern/i,
    );
    expect(urduTe.strokeOrderSource?.variation).toMatch(
      /be-series bowl first.*right-to-left.*left dot.*right dot.*two pen lifts.*two dots side by side.*squiggle or horizontal line.*Noto Naskh.*Nastaliq.*Urdu-specific/i,
    );
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ت")).toBe(false);
    expect(affected.get("ت") ?? 0).toBe(0);
    const urduHamzaAbove = scripts["urdu-nastaliq"]!.marks!.find(
      (entry) => entry.mark === "ٔ",
    )!;
    expect(urduHamzaAbove.examples?.map((example) => example.combined)).toEqual(
      ["ئ"],
    );
    expect(urduHamzaAbove.compositionOrder).toEqual([
      "write the tooth carrier as part of the word's right-to-left main line",
      "after lifting, add the small hamza above the carrier as its ain-head shape or accepted diagonal squiggle",
    ]);
    expect(urduHamzaAbove.compositionSource?.citation).toMatch(
      /Zer o Zabar.*Ain and hamza.*initial and medial ئ handwriting animations.*Northwestern/i,
    );
    expect(urduHamzaAbove.compositionSource?.variation).toMatch(
      /vowel-separator.*بھائی.*carrier-plus-mark.*U\+0626.*U\+064A.*U\+0654.*U\+06CC/i,
    );
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ي")).toBe(false);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ٔ")).toBe(false);
    expect(affected.get("ي") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ح")).toBe(false);
    expect(affected.get("ح") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ٹ")).toBe(false);
    expect(affected.get("ٹ") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ظ")).toBe(false);
    expect(affected.get("ظ") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ب")).toBe(false);
    expect(affected.get("ب") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ق")).toBe(false);
    expect(affected.get("ق") ?? 0).toBe(0);
    expect(missingByScript.get("urdu-nastaliq.json")?.has("ط")).toBe(false);
    expect(affected.get("ط") ?? 0).toBe(0);
  },
};
