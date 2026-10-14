import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Tamil real-corpus figure targets", () => {
  it("draws a filmstrip for every Tamil letter lesson whose letter has a cited ductus", () => {
    // 39 one-glyph lessons, 37 glyphs. வ has both TA-S01-va and the guided
    // copy TA-W00, and the puḷḷi ் both TA-S08-pulli and TA-W03-pulli-vanakkam,
    // while a letter lesson without cited ductus remains an undrawn
    // candidate. Seven of the 37 are signs taught by themselves (ா ி ீ ெ ே ை
    // and the puḷḷi ்); the signs ு and ூ have no cited ductus and stay
    // undrawn. The Grantha ஸ (TA-S129) is cited to LipiTk's Tamil recognizer.
    const tamil = targets.filter((target) => target.lessonId.startsWith("TA-"));
    const single = tamil.filter((target) => target.letters === undefined);
    expect(single).toHaveLength(39);
    expect(new Set(single.map((target) => target.glyph)).size).toBe(37);
    expect(single.find((target) => target.lessonId === "TA-S129-letter-sa")?.glyph).toBe("ஸ");
    expect(
      Object.fromEntries(
        single
          .filter((target) => /^\p{M}+$/u.test(target.glyph))
          .map((target) => [target.lessonId, target.glyph]),
      ),
    ).toEqual({
      "TA-S08-pulli": "்",
      "TA-S09-i-sign": "ி",
      "TA-S114-vowel-sign-aa": "ா",
      "TA-S123-vowel-sign-ee": "ே",
      "TA-S128-vowel-sign-ai": "ை",
      "TA-S132-vowel-sign-ii": "ீ",
      "TA-S133-vowel-sign-e": "ெ",
      "TA-W03-pulli-vanakkam": "்",
    });
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of ["TA-S121-vowel-sign-u", "TA-S135-vowel-sign-uu"]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws every Tamil letter list, and every Tamil word whose signs have a cited written order", () => {
    // Four lessons list letters ("வ, க"). The rest are drawn in WRITTEN order:
    // a sign written left of its consonant (ெ ே ை, and the left half of ொ ோ)
    // comes before it, so மேசை is ே, ம, ை, ச, and a puḷḷi comes after its own
    // consonant, so வணக்கம் is வ, ண, க, ், க, ம, ். A word with ு or ூ (பேசு,
    // சொல்லுங்கள்) or the fused டி (எப்படி) is still refused before the
    // ledger is asked.
    const sequences = targets.filter(
      (target) => target.lessonId.startsWith("TA-") && target.letters !== undefined,
    );
    expect(
      Object.fromEntries(sequences.map((target) => [target.lessonId, target.letters!.join(" ")])),
    ).toEqual({
      "TA-S102-vowel-sign-oo": "ே ா",
      "TA-W01-abugida-va-ka": "வ க",
      "TA-W02-ma-retroflex-na": "ம ண",
      "TA-W02-three-ns": "ந ன ண",
      "TA-W03-write-vanakkam": "வ ண க ் க ம ்",
      "TA-W04-i-sign-write-nandri": "ி ந ன ் ற ி",
      "TA-W04-vowel-signs-nandri": "ந ன ற",
      "TA-W05-write-aam": "ஆ ம ்",
      "TA-W06-write-illai": "இ ல ் ை ல",
      "TA-W07-write-sari": "ச ர ி",
      "TA-W08-read-en": "எ ன ்",
      "TA-W08-short-o-observe": "ெ ச ா",
      "TA-W09-read-peyar": "ெ ப ய ர ்",
      "TA-W10-read-naan": "ந ா ன ்",
      "TA-W11-read-niingal": "ந ீ ங ் க ள ்",
      "TA-W15-read-po": "ே ப ா",
      "TA-W16-read-tamizh": "த ம ி ழ ்",
      "TA-W18-read-uur": "ஊ ர ்",
      "TA-W22-read-mele": "ே ம ே ல",
      "TA-W23-read-sattai": "ச ட ் ை ட",
      "TA-W24-read-kadai": "க ை ட",
      "TA-W26-read-mesai": "ே ம ை ச",
      "TA-W27-read-payam": "ப ய ம ்",
      "TA-W30-read-sariyaa": "ச ர ி ய ா",
      "TA-W31-read-aanaal": "ஆ ன ா ல ்",
      "TA-W32-read-sol": "ெ ச ா ல ்",
      "TA-W33-read-een": "ஏ ன ்",
    });
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "TA-W08-sollungal-guided-copy",
      "TA-W12-read-eppadi",
      "TA-W14-read-pesu",
      "TA-W19-read-muunru",
      "TA-W25-read-vandi",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

});
