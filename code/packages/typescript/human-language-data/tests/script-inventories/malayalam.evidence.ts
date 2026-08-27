// Exact real-corpus evidence owned by the Malayalam inventory.
// See HL24: unrelated script authors must not share an executable edit surface.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

export const scriptInventoryEvidence = {
  name: "Malayalam",
  assert({
    taxonomy,
    lessons,
    scripts,
    affected,
    missingByScript,
  }: ScriptEvidenceContext): void {
    const candrakkala = scripts.malayalam!.marks!.find(
      (mark) => mark.mark === "്",
    )!;
    expect(candrakkala.role).toBe("virama");
    expect(candrakkala.compositionOrder).toEqual([
      "write the Malayalam carrier first",
      "add the candrakkala to suppress its inherent vowel or prepare the following conjunct",
    ]);
    expect(candrakkala.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(candrakkala.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.9\.3.*Candrakkala.*U\+0D4D/i,
    );
    expect(candrakkala.compositionSource?.variation).toMatch(
      /encoded composition.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const malayalamAnusvara = scripts.malayalam!.marks!.find(
      (mark) => mark.mark === "ം",
    )!;
    expect(malayalamAnusvara.role).toBe("anusvara");
    expect(malayalamAnusvara.compositionOrder).toEqual([
      "write the Malayalam base first",
      "add the anusvara after it",
    ]);
    expect(malayalamAnusvara.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/",
    );
    expect(malayalamAnusvara.compositionSource?.citation).toMatch(
      /Unicode Standard.*Version 17\.0.*12\.9\.3.*Anusvara.*U\+0D02/i,
    );
    expect(malayalamAnusvara.compositionSource?.variation).toMatch(
      /independent vowels.*dependent vowel signs.*Malayalam letters.*encoded composition.*not a universal handwriting direction.*no standalone ductus claim/i,
    );
    const malayalamE = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "എ",
    )!;
    expect(malayalamE.sound).toBe("e");
    expect(malayalamE.penLifts).toBe(1);
    expect(malayalamE.strokeOrder).toEqual([
      "turn around the compact left hook and carry the middle bar right",
      "without lifting, climb the upright, retrace it downward, and loop below the line",
      "after one lift, sweep up and over through the broad outer arch, ending below the line",
    ]);
    expect(malayalamE.strokeOrderNote).toMatch(
      /three visible movements.*two pen-down runs.*after one lift/i,
    );
    expect(malayalamE.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamE.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*എ.*00:01.?00:04.*University of Texas at Austin/i,
    );
    expect(malayalamE.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*click-to-play handwriting clip.*two pen-down runs.*inner loop and outer arch below the line.*Noto Sans Malayalam/i,
    );
    const malayalamA = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "അ",
    )!;
    expect(malayalamA.sound).toBe("a");
    expect(malayalamA.penLifts).toBe(1);
    expect(malayalamA.strokeOrder).toEqual([
      "climb the left outer arch, curve through the upper turn, and arrive at the central junction",
      "without lifting, circle the broad lower loop and return to the junction",
      "without lifting, sweep up through the central crown and descend the upright",
      "after one lift, sweep up and over through the right outer arch and descend its far side",
      "without lifting, curl left around the lower inner loop",
    ]);
    expect(malayalamA.strokeOrderNote).toMatch(
      /five visible movements.*two pen-down runs.*after one lift/i,
    );
    expect(malayalamA.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamA.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*അ.*00:00.?00:04.*University of Texas at Austin/i,
    );
    expect(malayalamA.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*click-to-play handwriting clip.*left-and-central body.*one lifted right-side run.*outer arch.*lower inner loop.*Noto Sans Malayalam/i,
    );
    const malayalamAa = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "ആ",
    )!;
    expect(malayalamAa.sound).toBe("ā");
    expect(malayalamAa.penLifts).toBe(1);
    expect(malayalamAa.strokeOrder).toEqual([
      "climb the left outer arch and curve inward at the top",
      "after one lift, turn inward around the compact inner curl and circle the broad lower loop",
      "without lifting, sweep up through the central crown and descend the upright",
      "without lifting, retrace the upright and sweep around the rounded right loop",
      "without lifting, descend the far side and curl left below the line",
    ]);
    expect(malayalamAa.strokeOrderNote).toMatch(
      /five visible movements.*two pen-down runs.*after one lift/i,
    );
    expect(malayalamAa.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B4%86_order.gif",
    );
    expect(malayalamAa.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ആ order\.gif.*Malayalam independent vowel ആ.*Gayathri.*73 frames.*11 seconds.*Wikimedia Commons.*1 June 2023/i,
    );
    expect(malayalamAa.strokeOrderSource?.variation).toMatch(
      /CC BY-SA 4\.0.*left outer arch.*frames 2.?9.*disconnected second run.*frame 10.*inner curl.*lower loop.*central upright.*rounded right loop.*below-line finish.*Noto Sans Malayalam.*one-lift order/i,
    );
    const malayalamI = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "ഇ",
    )!;
    expect(malayalamI.sound).toBe("i");
    expect(malayalamI.penLifts).toBe(0);
    expect(malayalamI.strokeOrder).toEqual([
      "begin at the compact inner tip, turn outward around the left spiral, and descend the central stem",
      "without lifting, retrace the central stem and sweep around the broad right lobe",
      "without lifting, curl left below the line",
      "without lifting, carry the finishing baseline to the right",
    ]);
    expect(malayalamI.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamI.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*ഇ.*00:00.?00:04.*University of Texas at Austin/i,
    );
    expect(malayalamI.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*i\.mp4.*compact inner tip.*left spiral.*descends and retraces the central stem.*broad right lobe.*below the line.*finishing baseline right.*zero-lift.*Noto Sans Malayalam/i,
    );
    const malayalamU = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "ഉ",
    )!;
    expect(malayalamU.sound).toBe("u");
    expect(malayalamU.penLifts).toBe(0);
    expect(malayalamU.strokeOrder).toEqual([
      "begin at the compact inner tip, turn outward around the left spiral, and carry the upper arch right",
      "without lifting, descend around the broad right lobe and curl left below the line",
      "without lifting, carry the finishing baseline to the right",
    ]);
    expect(malayalamU.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamU.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*ഉ.*00:00.?00:05.*University of Texas at Austin/i,
    );
    expect(malayalamU.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*u\.mp4.*compact inner tip.*left spiral.*broad upper and right lobe.*below-line curl.*finishing baseline.*zero-lift.*Noto Sans Malayalam/i,
    );
    const malayalamChilluL = scripts.malayalam!.finalConsonants!.find(
      (entry) => entry.glyph === "ൽ",
    )!;
    expect(malayalamChilluL.sound).toBe("l");
    expect(malayalamChilluL.role).toBe("consonant");
    expect(malayalamChilluL.penLifts).toBe(0);
    expect(malayalamChilluL.strokeOrder).toHaveLength(5);
    expect(malayalamChilluL.strokeOrderNote).toMatch(
      /five visible movements.*one continuous pen-down run/i,
    );
    expect(malayalamChilluL.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BD_order.gif",
    );
    expect(malayalamChilluL.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ൽ order\.gif.*chillu L.*00:00\.1.?00:09\.6.*Wikimedia Commons.*2 July 2023/i,
    );
    expect(malayalamChilluL.strokeOrderSource?.variation).toMatch(
      /97-frame Gayathri-font animation.*one uninterrupted run.*left entry arch.*central loop.*rightward upper shoulder.*right loop.*hook above the line.*University of Texas.*Noto Sans Malayalam/i,
    );
    expect(malayalamChilluL.notes).toMatch(
      /U\+0D7D.*vowel-free final consonant.*not the base ല/i,
    );
    const malayalamChilluN = scripts.malayalam!.finalConsonants!.find(
      (entry) => entry.glyph === "ൻ",
    )!;
    expect(malayalamChilluN.sound).toBe("n");
    expect(malayalamChilluN.role).toBe("consonant");
    expect(malayalamChilluN.penLifts).toBe(1);
    expect(malayalamChilluN.strokeOrder).toHaveLength(4);
    expect(malayalamChilluN.strokeOrderNote).toMatch(
      /four visible movements.*two pen-down runs.*one lifted right-side run/i,
    );
    expect(malayalamChilluN.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BB_order.gif",
    );
    expect(malayalamChilluN.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ൻ order\.gif.*chillu N.*00:03\.0.?00:09\.5.*Wikimedia Commons.*2 July 2023/i,
    );
    expect(malayalamChilluN.strokeOrderSource?.variation).toMatch(
      /67-frame Gayathri-font animation.*left arch.*central stem.*lifts once.*right outer loop.*inner return.*hook above the line.*Noto Sans Malayalam/i,
    );
    expect(malayalamChilluN.notes).toMatch(
      /U\+0D7B.*vowel-free final consonant.*not the base ന/i,
    );
    const malayalamChilluLL = scripts.malayalam!.finalConsonants!.find(
      (entry) => entry.glyph === "ൾ",
    )!;
    expect(malayalamChilluLL.sound).toBe("ḷ");
    expect(malayalamChilluLL.role).toBe("consonant");
    expect(malayalamChilluLL.penLifts).toBe(0);
    expect(malayalamChilluLL.strokeOrder).toEqual([
      "descend clockwise around the left bowl and climb the central rise",
      "without lifting, carry the upper shoulder right",
      "without lifting, sweep clockwise around the right loop and return to the upper crossing",
      "without lifting, rise into the chillu hook and curl left above the line",
    ]);
    expect(malayalamChilluLL.strokeOrderNote).toMatch(
      /four visible movements.*one continuous pen-down run/i,
    );
    expect(malayalamChilluLL.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BE_order.gif",
    );
    expect(malayalamChilluLL.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ൾ order\.gif.*chillu LL.*00:03\.0.?00:09\.3.*Wikimedia Commons.*14 July 2023/i,
    );
    expect(malayalamChilluLL.strokeOrderSource?.variation).toMatch(
      /65-frame Gayathri-font animation.*one uninterrupted run.*left bowl.*central rise.*upper shoulder.*right loop.*hook above the line.*Noto Sans Malayalam.*zero-lift order/i,
    );
    expect(malayalamChilluLL.notes).toMatch(
      /U\+0D7E.*vowel-free retroflex lateral final consonant.*not the base ള/i,
    );
    const malayalamChilluRR = scripts.malayalam!.finalConsonants!.find(
      (entry) => entry.glyph === "ർ",
    )!;
    expect(malayalamChilluRR.sound).toBe("r");
    expect(malayalamChilluRR.role).toBe("consonant");
    expect(malayalamChilluRR.penLifts).toBe(0);
    expect(malayalamChilluRR.strokeOrder).toEqual([
      "climb around the left arch and carry the upper shoulder right",
      "without lifting, sweep clockwise around the right loop and return to the upper crossing",
      "without lifting, rise into the chillu hook and curl left above the line",
    ]);
    expect(malayalamChilluRR.strokeOrderNote).toMatch(
      /three visible movements.*one continuous pen-down run/i,
    );
    expect(malayalamChilluRR.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BC_order.gif",
    );
    expect(malayalamChilluRR.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ർ order\.gif.*chillu RR.*00:03\.0.?00:08\.5.*Wikimedia Commons.*2 July 2023/i,
    );
    expect(malayalamChilluRR.strokeOrderSource?.variation).toMatch(
      /57-frame Gayathri-font animation.*one uninterrupted run.*lower-left tip.*left arch.*upper shoulder.*right loop.*inner side.*hook above the line.*Noto Sans Malayalam.*zero-lift order/i,
    );
    expect(malayalamChilluRR.notes).toMatch(
      /U\+0D7C.*vowel-free final consonant.*not the base ര/i,
    );
    const malayalamZha = scripts.malayalam!.letters.find(
      (entry) => entry.glyph === "ഴ",
    )!;
    expect(malayalamZha.sound).toBe("ḻa");
    expect(malayalamZha.role).toBe("syllable");
    expect(malayalamZha.penLifts).toBe(0);
    expect(malayalamZha.strokeOrder).toHaveLength(3);
    expect(malayalamZha.strokeOrderNote).toMatch(
      /three visible movements.*one continuous pen-down run/i,
    );
    expect(malayalamZha.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B4%B4_order.gif",
    );
    expect(malayalamZha.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ഴ order\.gif.*letter LLLA.*00:03\.0.?00:07\.4.*Wikimedia Commons.*1 July 2023/i,
    );
    expect(malayalamZha.strokeOrderSource?.variation).toMatch(
      /47-frame Gayathri-font animation.*one uninterrupted run.*left entry arch.*clockwise right loop.*inner return.*lower hook.*Noto Sans Malayalam/i,
    );
    expect(malayalamZha.notes).toMatch(
      /U\+0D34.*ISO 15919.*base consonant.*inherent a/i,
    );
    expect(missingByScript.get("malayalam.json")?.has("്")).toBe(false);
    expect(affected.get("്") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ം")).toBe(false);
    expect(affected.get("ം") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("അ")).toBe(false);
    expect(affected.get("അ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ൽ")).toBe(false);
    expect(affected.get("ൽ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ൻ")).toBe(false);
    expect(affected.get("ൻ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ൾ")).toBe(false);
    expect(affected.get("ൾ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ർ")).toBe(false);
    expect(affected.get("ർ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ഴ")).toBe(false);
    expect(affected.get("ഴ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ആ")).toBe(false);
    expect(affected.get("ആ") ?? 0).toBe(0);
  },
};
