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
    // The Unicode source still claims placement only; the sign's own ductus,
    // drawn alone, is a separate claim cited to Jayasree (checked below).
    expect(candrakkala.compositionSource?.variation).toMatch(
      /encoded composition.*not a universal handwriting direction.*own ductus, drawn alone, is cited separately in strokeOrderSource/i,
    );
    const malayalamAnusvara = scripts.malayalam!.marks!.find(
      (mark) => mark.mark === "ം",
    )!;
    expect(malayalamAnusvara.role).toBe("anusvara");
    expect(malayalamAnusvara.compositionOrder).toEqual([
      "write the Malayalam base first",
      "add the anusvara after it",
    ]);
    // The anusvara's written order now cites Moag's അം (the ring is movement
    // 9, after the eight of അ); Unicode is kept in the variation for what it
    // does say (the sign follows its base) and what it does not (when).
    expect(malayalamAnusvara.compositionSource?.url).toBe(
      "https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-029.jpg",
    );
    expect(malayalamAnusvara.compositionSource?.citation).toMatch(
      /Rodney F\. Moag.*Table II.*p\. xix: movements 1-9 for അം.*Table III, p\. xxiv/,
    );
    expect(malayalamAnusvara.compositionSource?.variation).toMatch(
      /movement 9.*base is written first and the anusvara after it.*Unicode Standard.*12\.9\.3.*says nothing about when the sign is written/i,
    );
    expect(malayalamAnusvara.strokeOrder).toEqual(["circle clockwise"]);
    expect(malayalamAnusvara.penLifts).toBe(0);
    expect(malayalamAnusvara.strokeOrderSource?.url).toBe(
      "https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-034.jpg",
    );
    expect(malayalamAnusvara.strokeOrderSource?.variation).toMatch(
      /clockwise.*medium-low confidence.*anticlockwise.*pen lifts after the base/,
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
    const malayalamUu = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "ഊ",
    )!;
    expect(malayalamUu.sound).toBe("ū");
    expect(malayalamUu.penLifts).toBe(1);
    expect(malayalamUu.strokeOrder).toEqual([
      "begin at the compact inner tip, turn outward around the left spiral, and carry the upper arch right",
      "without lifting, descend around the broad right lobe and curl left below the line",
      "without lifting, carry the finishing baseline to the right",
      "after one lift, sweep over the compact upper arch and cross into the right lobe",
      "without lifting, circle the broad right lobe and descend its finishing tail",
    ]);
    expect(malayalamUu.strokeOrderNote).toMatch(
      /five visible movements.*two pen-down runs.*after one lift/i,
    );
    expect(malayalamUu.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamUu.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*ഊ.*00:00.?00:06.*University of Texas at Austin/i,
    );
    expect(malayalamUu.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*uu\.mp4.*spiral-to-baseline body.*lifts once.*compact upper arch.*broad right loop.*descending tail.*one-lift.*Noto Sans Malayalam/i,
    );
    const malayalamO = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "ഒ",
    )!;
    expect(malayalamO.sound).toBe("o");
    expect(malayalamO.penLifts).toBe(1);
    expect(malayalamO.strokeOrder).toEqual([
      "begin at the compact inner tip, curl clockwise, and sweep outward through the broad left arch",
      "after one lift, sweep right from the upper junction and descend around the rounded lower lobe",
    ]);
    expect(malayalamO.strokeOrderNote).toMatch(
      /two visible movements.*two pen-down runs.*after one lift/i,
    );
    expect(malayalamO.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamO.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*ഒ.*00:00.?00:04.*University of Texas at Austin/i,
    );
    expect(malayalamO.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*o\.mp4.*compact inner curl.*broad left arch.*lifts once.*right shoulder.*rounded lower lobe.*one-lift.*Noto Sans Malayalam/i,
    );
    const malayalamOo = scripts.malayalam!.independentVowels!.find(
      (entry) => entry.glyph === "ഓ",
    )!;
    expect(malayalamOo.sound).toBe("ō");
    expect(malayalamOo.penLifts).toBe(2);
    expect(malayalamOo.strokeOrder).toEqual([
      "begin at the compact inner tip, curl clockwise, and sweep outward through the broad left arch",
      "after one lift, sweep right from the upper junction and descend around the rounded lower lobe",
      "after a second lift, descend around the separate outer arc at the far right",
    ]);
    expect(malayalamOo.strokeOrderNote).toMatch(
      /three visible movements.*three pen-down runs.*after a second lift/i,
    );
    expect(malayalamOo.strokeOrderSource?.url).toBe(
      "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
    );
    expect(malayalamOo.strokeOrderSource?.citation).toMatch(
      /Donald R\. Davis Jr\..*The Malayalam Script.*Initial Vowels.*ഓ.*00:00.?00:04.*University of Texas at Austin/i,
    );
    expect(malayalamOo.strokeOrderSource?.variation).toMatch(
      /word-initial forms.*oo\.mp4.*same two lifted runs as short o.*lifts a second time.*far-right outer arc.*top to bottom.*two-lift.*Noto Sans Malayalam/i,
    );
    const malayalamChilluNN = scripts.malayalam!.finalConsonants!.find(
      (entry) => entry.glyph === "ൺ",
    )!;
    expect(malayalamChilluNN.sound).toBe("ṇ");
    expect(malayalamChilluNN.role).toBe("consonant");
    expect(malayalamChilluNN.penLifts).toBe(0);
    expect(malayalamChilluNN.strokeOrder).toHaveLength(5);
    expect(malayalamChilluNN.strokeOrderNote).toMatch(
      /five visible movements.*one continuous pen-down run/i,
    );
    expect(malayalamChilluNN.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BA_order.gif",
    );
    expect(malayalamChilluNN.strokeOrderSource?.citation).toMatch(
      /Sriveenkat.*Ml ൺ order\.gif.*chillu NN.*00:03\.0.?00:12\.8.*Wikimedia Commons.*2 July 2023/i,
    );
    expect(malayalamChilluNN.strokeOrderSource?.variation).toMatch(
      /97-frame Gayathri-font animation.*one uninterrupted run.*inner loop.*outer-left bowl.*central stem.*middle arch.*second stem.*right loop.*hook above the line.*Noto Sans Malayalam.*zero-lift order/i,
    );
    expect(malayalamChilluNN.notes).toMatch(
      /U\+0D7A.*vowel-free retroflex nasal final consonant.*not the base ണ/i,
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
    expect(missingByScript.get("malayalam.json")?.has("്") ?? false).toBe(false);
    expect(affected.get("്") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ം") ?? false).toBe(false);
    expect(affected.get("ം") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("അ") ?? false).toBe(false);
    expect(affected.get("അ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ൽ") ?? false).toBe(false);
    expect(affected.get("ൽ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ൻ") ?? false).toBe(false);
    expect(affected.get("ൻ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ൾ") ?? false).toBe(false);
    expect(affected.get("ൾ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ർ") ?? false).toBe(false);
    expect(affected.get("ർ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ഴ") ?? false).toBe(false);
    expect(affected.get("ഴ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ആ") ?? false).toBe(false);
    expect(affected.get("ആ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ഊ") ?? false).toBe(false);
    expect(affected.get("ഊ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ഒ") ?? false).toBe(false);
    expect(affected.get("ഒ") ?? 0).toBe(0);
    expect(missingByScript.get("malayalam.json")?.has("ഓ") ?? false).toBe(false);
    expect(affected.get("ഓ") ?? 0).toBe(0);
    // Seventeen base consonants cite SPACE Kerala's Thooval formation arrows
    // (facts only: GPL-3.0, nothing copied) and are written in one run each.
    const thooval: Record<string, readonly [string, number]> = {
      "ന": ["NA", 4], "മ": ["MA", 4], "സ": ["SA", 5], "ര": ["RA", 3],
      "ത": ["TA", 4], "ഷ": ["SSA", 6], "പ": ["PA", 3], "വ": ["VA", 3],
      "ണ": ["NNA", 6], "ട": ["TTA", 3], "ദ": ["DA", 3], "ഹ": ["HA", 4],
      "ഗ": ["GA", 3], "റ": ["RRA", 2], "ല": ["LA", 5], "ശ": ["SHA", 4],
      "ബ": ["BA", 6],
    };
    for (const [glyph, [slug, movements]] of Object.entries(thooval)) {
      const row = scripts.malayalam!.letters.find(
        (entry) => entry.glyph === glyph,
      )!;
      expect(row.role).toBe("syllable");
      expect(row.penLifts).toBe(0);
      expect(row.strokeOrder).toHaveLength(movements);
      expect(row.strokeOrder.slice(1).every((s) => s.startsWith("without lifting, "))).toBe(true);
      expect(row.strokeOrderNote).toMatch(
        /visible movements in one continuous pen-down run.*Thooval keeps the pen down by design/i,
      );
      expect(row.strokeOrderSource?.url).toBe(
        `https://github.com/spacekerala/Thooval/blob/87143b560bf5aab43837d9da2cddab9bd59cd391/data/${slug}.png`,
      );
      expect(row.strokeOrderSource?.citation).toMatch(
        new RegExp(`SPACE Kerala.*Thooval.*formation arrows for ${glyph} in data/${slug}\\.png.*GPL-3\\.0, 2013`),
      );
      expect(row.strokeOrderSource?.variation).toMatch(
        /only these facts are cited.*santhoshtr\/hand, MIT.*one stroke.*grahyam.*counts only.*none of the \d+ unique samples jumps.*Noto Sans Malayalam/,
      );
    }
    // Moag's Malayalam: A University Course and Reference Grammar (Tables
    // II-IV, numbered movements in a native writer's hand; facts only, CC
    // BY-NC-SA 4.0, nothing copied) cites twelve more consonants, among them
    // ക and യ (held earlier while the sources disagreed on their start), the
    // vowel ഏ, and eight vowel signs drawn alone. Each is one run: Moag
    // numbers movements, not lifts, and the recordings show none.
    const moagScan = (page: string): string =>
      `https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-${page}.jpg`;
    const moagRows: Record<string, readonly [string, number]> = {
      "ക": ["035", 7], "യ": ["040", 4], "ഖ": ["035", 4], "ങ": ["035", 5],
      "ച": ["036", 4], "ഛ": ["036", 5], "ഞ": ["036", 7], "ഥ": ["038", 3],
      "ധ": ["038", 4], "ഭ": ["039", 4], "ഫ": ["039", 3], "ള": ["041", 4],
      "ഏ": ["028", 7],
      // ജ joined later: its short stem's descent is read between arrows 2
      // and 3, which its variation says at medium confidence.
      "ജ": ["036", 6],
    };
    for (const [glyph, [page, movements]] of Object.entries(moagRows)) {
      const row = [
        ...scripts.malayalam!.letters,
        ...(scripts.malayalam!.independentVowels ?? []),
      ].find((entry) => entry.glyph === glyph)!;
      expect(row.role, glyph).toBe(glyph === "ഏ" ? "vowel" : "syllable");
      expect(row.penLifts, glyph).toBe(0);
      expect(row.strokeOrder, glyph).toHaveLength(movements);
      expect(row.strokeOrder.slice(1).every((s) => s.startsWith("without lifting, ")), glyph).toBe(true);
      expect(row.strokeOrderNote, glyph).toMatch(
        /numbered movements in one continuous pen-down run.*Moag numbers movements, not pen lifts/,
      );
      expect(row.strokeOrderSource?.url, glyph).toBe(moagScan(page));
      expect(row.strokeOrderSource?.citation, glyph).toMatch(
        new RegExp(`^Rodney F\\. Moag, Malayalam: A University Course and Reference Grammar .*CC BY-NC-SA 4\\.0.*movements 1-${movements} for ${glyph}, written by hand by Thomas Joseph$`),
      );
      expect(row.strokeOrderSource?.variation, glyph).toMatch(
        /only these facts .* are cited; no drawing is copied.*not pen lifts.*grahyam.*counts only.*Noto Sans Malayalam/,
      );
    }
    const moagSigns: Record<string, readonly [string, number]> = {
      "ാ": ["030", 1], "ി": ["030", 1], "ീ": ["030", 2], "ു": ["031", 3],
      "ൂ": ["031", 4], "ൃ": ["031", 2], "െ": ["032", 2], "േ": ["032", 3],
    };
    for (const [sign, [page, movements]] of Object.entries(moagSigns)) {
      const mark = scripts.malayalam!.marks!.find((entry) => entry.mark === sign)!;
      expect(mark.role, sign).toBe("vowel-sign");
      expect(mark.penLifts, sign).toBe(0);
      expect(mark.strokeOrder, sign).toHaveLength(movements);
      expect(mark.strokeOrderSource?.url, sign).toBe(moagScan(page));
      expect(mark.strokeOrderSource?.citation, sign).toMatch(
        new RegExp(`Table III 'How to Write Internal Vowel Symbols'.* for ${sign}, written by hand by Thomas Joseph$`),
      );
      // Moag never numbers the consonant against a vowel sign, so no sign
      // record claims a written order.
      expect(mark.compositionOrder, sign).toBeUndefined();
      expect(missingByScript.get("malayalam.json")?.has(sign) ?? false, sign).toBe(false);
    }
    // ൈ is the one two-run sign: two coils of െ, the gap between them a lift.
    // No recording of ൈ itself exists here, so its record reasons the lift
    // and says confidence is medium; like the other signs, it claims no
    // written order against its consonant.
    const ai = scripts.malayalam!.marks!.find((entry) => entry.mark === "ൈ")!;
    expect(ai.role).toBe("vowel-sign");
    expect(ai.example).toEqual({ base: "ക", combined: "കൈ", sound: "kai" });
    expect(ai.penLifts).toBe(1);
    expect(ai.strokeOrder).toEqual([
      "circle the first small loop",
      "without lifting, arch over and down to the foot",
      "lift, then circle the second small loop",
      "without lifting, arch over and down to its foot",
    ]);
    expect(ai.strokeOrderSource?.url).toBe(moagScan("032"));
    expect(ai.strokeOrderSource?.citation).toMatch(
      /Table III 'How to Write Internal Vowel Symbols'.*p\. xxii: movements 1-4 for ൈ, written by hand by Thomas Joseph$/,
    );
    expect(ai.strokeOrderSource?.variation).toMatch(
      /only these facts .* are cited; no drawing is copied.*No recording of ൈ itself.*confidence is medium.*claims no written order/,
    );
    expect(ai.compositionOrder).toBeUndefined();
    // Jayasree (github.com/sachn1/jayasree, CC BY 4.0) is a recording: one
    // recorder's pen-down gestures over the Manjari typeface. It cites the
    // candrakkala drawn alone, ഠ (breaking the earlier tie on its direction)
    // and the digits ൧-൯. Every one is a single recorded stroke, so no lift,
    // and every record credits the work by name and licence. ൦ is recorded
    // too but no lesson draws it, so its row stays recognition only.
    const jayasree =
      "https://github.com/sachn1/jayasree/blob/e0c9d57dd32031c948da4d5f8432aae3e22c5bba/js/src/stroke-data.raw.json";
    const jayasreeRows: Record<string, number> = {
      "്": 2, "ഠ": 2, "൧": 4, "൨": 3, "൩": 5, "൪": 4, "൫": 5, "൬": 6, "൭": 3,
      "൮": 5, "൯": 6,
    };
    for (const [glyph, movements] of Object.entries(jayasreeRows)) {
      const row = [
        ...scripts.malayalam!.letters,
        ...(scripts.malayalam!.digits ?? []),
        ...(scripts.malayalam!.marks ?? []).map((mark) => ({ ...mark, glyph: mark.mark })),
      ].find((entry) => entry.glyph === glyph)!;
      expect(row.penLifts, glyph).toBe(0);
      expect(row.strokeOrder, glyph).toHaveLength(movements);
      expect(row.strokeOrder.slice(1).every((s) => s.startsWith("without lifting, ")), glyph).toBe(true);
      expect(row.strokeOrderSource?.url, glyph).toBe(jayasree);
      expect(row.strokeOrderSource?.citation, glyph).toMatch(
        new RegExp(`^Sachin Nandakumar, Jayasree: .*commit e0c9d57.*centre-line stroke for ${glyph}.*"Jayasree" by Sachin Nandakumar, CC BY 4\\.0$`),
      );
      expect(row.strokeOrderSource?.variation, glyph).toMatch(
        /one stroke means the recorder did not lift.*no recorded coordinate is copied.*CC BY 4\.0 \(https:\/\/creativecommons\.org\/licenses\/by\/4\.0\/\).*Noto Sans Malayalam.*confidence is medium/,
      );
    }
    expect(
      scripts.malayalam!.letters.find((entry) => entry.glyph === "ഠ")!.strokeOrderSource?.variation,
    ).toMatch(/Thooval .*grahyam .*anticlockwise too, while Moag's Table IV arrow .*clockwise.*three sources to one/);
    expect(candrakkala.compositionOrder).toBeDefined();
    const zero = scripts.malayalam!.digits!.find((entry) => entry.glyph === "൦")!;
    expect(zero.strokeOrder).toEqual([]);
    expect(zero.strokeOrderSource).toBeUndefined();
    // ൊ and ോ are two recorded strokes each: the left sign, a lift, then ാ.
    // The placeholder dot Noto prints between the parts is not written, and
    // each record says so; neither claims a written order against a consonant.
    for (const [sign, leftMovements] of [["ൊ", 2], ["ോ", 3]] as const) {
      const mark = scripts.malayalam!.marks!.find((entry) => entry.mark === sign)!;
      expect(mark.role, sign).toBe("vowel-sign");
      expect(mark.penLifts, sign).toBe(1);
      expect(mark.strokeOrder, sign).toHaveLength(leftMovements + 1);
      expect(mark.strokeOrder.at(-1), sign).toBe("lift, then draw ാ clockwise");
      expect(mark.strokeOrderSource?.url, sign).toBe(jayasree);
      expect(mark.strokeOrderSource?.citation, sign).toMatch(
        new RegExp(`strokes for ${sign}, drawn alone; "Jayasree" by Sachin Nandakumar, CC BY 4\\.0$`),
      );
      expect(mark.strokeOrderSource?.variation, sign).toMatch(
        /placeholder dot where the consonant would sit.*not written.*claims no written order.*CC BY 4\.0.*confidence is medium/,
      );
      expect(mark.compositionOrder, sign).toBeUndefined();
      expect(missingByScript.get("malayalam.json")?.has(sign) ?? false, sign).toBe(false);
    }
    expect(scripts.malayalam!.marks!.map((mark) => mark.mark)).toEqual([
      "ം", "്", "ാ", "ി", "ീ", "ു", "ൂ", "ൃ", "െ", "േ", "ൈ", "ൊ", "ോ",
    ]);
  },
};
