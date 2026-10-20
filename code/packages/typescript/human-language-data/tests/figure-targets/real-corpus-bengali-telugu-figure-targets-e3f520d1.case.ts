import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Bengali and Telugu real-corpus figure targets", () => {
  it("draws a Telugu sign only where a lesson teaches it alone", () => {
    // Eleven Telugu sign lessons teach one of eleven signs by itself, each
    // cited to native writers in LipiTk's Telugu recognizer who wrote it
    // alone. Each is one glyph, never a sequence: Telugu has no written-order
    // table, so no sign is placed against a consonant.
    const signs = targets.filter(
      (target) => target.script === "telugu" && /^\p{M}+$/u.test(target.glyph),
    );
    expect(Object.fromEntries(signs.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "TE-S02-vowel-sign-u": "ు",
      "TE-S04-sign-anusvara": "ం",
      "TE-S05-sign-virama": "్",
      "TE-S07-vowel-sign-aa": "ా",
      "TE-S08-vowel-sign-i": "ి",
      "TE-S115-vowel-sign-e": "ె",
      "TE-S119-vowel-sign-ii": "ీ",
      "TE-S120-vowel-sign-ee": "ే",
      "TE-S134-vowel-sign-uu": "ూ",
      "TE-S153-vowel-sign-oo": "ో",
      "TE-S154-vowel-sign-o": "ొ",
    });
    expect(signs.every((target) => target.letters === undefined)).toBe(true);
    // Left undrawn: ై, whose recognizer class holds only the length mark
    // below, and ృ and ౌ, which the recognizer lacks.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of ["TE-S136-vowel-sign-ai", "TE-S131-vowel-sign-vocalic-r", "TE-S133-vowel-sign-au"]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws Bengali ং where its lesson teaches it", () => {
    const anusvar = targets.find((target) => target.lessonId === "BN-W41-anusvar");
    expect(anusvar?.script).toBe("bengali");
    expect(anusvar?.glyph).toBe("ং");
    expect(anusvar?.letters).toBeUndefined();
  });

  it("draws the Bengali letters whose headline is drawn last by convention", () => {
    // Fourteen letters cited to LipiTk's Bangla recognizer, each drawn with
    // its body in the traces' majority order and the headline last (no
    // headline placement wins a majority). ন has four writing lessons.
    const bengali = Object.fromEntries(
      targets
        .filter((target) => target.script === "bengali" && target.letters === undefined)
        .map((target) => [target.lessonId, target.glyph]),
    );
    expect(bengali).toMatchObject({
      "BN-W01-ha": "হ",
      "BN-W01-ma": "ম",
      "BN-W01-na": "ন",
      "BN-W01-na-trace": "ন",
      "BN-W01-na-guided-copy": "ন",
      "BN-W01-na-delayed-copy": "ন",
      "BN-W02-la": "ল",
      "BN-W02-ta": "ত",
      "BN-W02-ya": "য",
      "BN-W03-bha": "ভ",
      "BN-W03-da": "দ",
      "BN-W04-cha": "চ",
      "BN-W04-chha": "ছ",
      "BN-W04-dda": "ড",
      "BN-W04-i-indep": "ই",
      "BN-W04-ja": "জ",
      "BN-W41-pha": "ফ",
    });
    // Left undrawn, with the reason in data/scripts/bengali.json: ক (its
    // majority headline stops at the stem), আ গ ট ধ প স শ (no majority body),
    // ঝ (Noto splits its headline around the right stem).
    for (const id of ["BN-W01-ka", "BN-W01-aa", "BN-W03-ga", "BN-W05-tta", "BN-W03-dha", "BN-W04-pa", "BN-W01-sa", "BN-W41-sha", "BN-W41-jha"]) {
      expect(bengali[id], id).toBeUndefined();
    }
  });

});
