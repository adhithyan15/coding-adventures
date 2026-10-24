import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Devanagari real-corpus figure targets", () => {
  it("draws a Devanagari sign only where a lesson teaches it alone", () => {
    // 47 Hindi, Marathi, Sanskrit and Marwadi lessons teach one of twelve
    // signs by itself (ा ु ू े ं ़ ् ृ ँ ी ो ः), each cited to native writers
    // who wrote it alone. The 47th is Hindi's visarga (HI-W281-sign-visarga),
    // added with the B1 core on its anchor word अतः. Each is one glyph, never a sequence: Devanagari has no
    // written-order table, so no sign is placed against a consonant here
    // (ā joins its consonant only inside a composed word, next cases).
    const devanagari = targets.filter((target) => target.script === "devanagari");
    const signs = devanagari.filter((target) => /^\p{M}+$/u.test(target.glyph));
    expect(signs.every((target) => target.letters === undefined)).toBe(true);
    expect(Object.fromEntries(signs.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "HI-S05-sign-virama": "्",
      "HI-S06-vowel-sign-aa": "ा",
      "HI-S112-vowel-sign-e": "े",
      "HI-S116-vowel-sign-ii": "ी",
      "HI-S118-sign-candrabindu": "ँ",
      "HI-S120-vowel-sign-u": "ु",
      "HI-S140-nuqta": "़",
      "HI-S144-vowel-sign-vocalic-r": "ृ",
      "HI-S148-sign-anusvara": "ं",
      "HI-S149-vowel-sign-uu": "ू",
      "HI-S150-vowel-sign-o": "ो",
      "HI-W12-chandrabindu": "ँ",
      "HI-W12-ii-matra": "ी",
      "HI-W12-u-matra": "ु",
      "HI-W281-sign-visarga": "ः",
      "MR-W01-aa-matra": "ा",
      "MR-W01-ii-matra": "ी",
      "MR-W01-o-matra": "ो",
      "MR-W01-virama": "्",
      "MR-W02-anusvara": "ं",
      "MR-W02-e-matra": "े",
      "MR-W02-visarga": "ः",
      "MR-W05-candrabindu": "ँ",
      "MR-W05-ru-matra": "ृ",
      "MR-W05-u-matra": "ु",
      "MR-W05-uu-matra": "ू",
      "MW-W01-aa-matra": "ा",
      "MW-W03-anusvara": "ं",
      "MW-W04-ii-matra": "ी",
      "MW-W05-o-matra": "ो",
      "MW-W05-virama": "्",
      "MW-W06-uu-matra": "ू",
      "MW-W07-e-matra": "े",
      "MW-W13-u-matra": "ु",
      "MW-W15-nukta": "़",
      "SA-S05-sign-virama": "्",
      "SA-S06-vowel-sign-aa": "ा",
      "SA-S112-vowel-sign-e": "े",
      "SA-S201-sign-visarga": "ः",
      "SA-S203-vowel-sign-u": "ु",
      "SA-S205-vowel-sign-vocalic-r": "ृ",
      "SA-S209-sign-anusvara": "ं",
      "SA-S210-vowel-sign-ii": "ी",
      "SA-S214-vowel-sign-o": "ो",
      "SA-S222-vowel-sign-uu": "ू",
      "SA-W05-vocalic-r-delayed-copy": "ृ",
      "SA-W05-vocalic-r-guided-copy": "ृ",
    });
    // ी, ो and ः are drawn without the piece of headline Noto prints on
    // them: it is the word's headline, which native writers draw last across
    // the whole word (script-ductus excuses exactly that stub in its coverage
    // check). Left undrawn: the signs whose traces split (ि ै ौ), a sign
    // lesson with no Writing or Script block and no modelled practice stage
    // (HI-W03-preposed-i), and every word or list that puts a sign on a
    // consonant.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "HI-W128-vowel-sign-i",
      "MR-W05-i-matra",
      "SA-S109-vowel-sign-i",
      "HI-S134-vowel-sign-au",
      "HI-W12-ai-matra",
      "MR-W56-au-matra",
      "SA-S223-vowel-sign-ai",
      "HI-W03-preposed-i",
      "HI-W03-matras-naam",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws eligible words and phrases with one shared headline per word", () => {
    const shared = targets.filter(
      (target) => target.script === "devanagari" && target.composition === "shared-headline",
    );
    expect(
      Object.fromEntries(
        shared
          .filter((target) => target.letters === undefined)
          .map((target) => [target.lessonId, target.glyph]),
      ),
    ).toEqual({
      "HI-A1F01-name-label": "नाम",
      "HI-W12-schwa-drop": "नाम",
      "MW-W01-saa": "सा",
      "SA-W03-mama-delayed-copy": "मम",
      "SA-W03-mama-guided-copy": "मम",
    });
    expect(
      Object.fromEntries(
        shared
          .filter((target) => target.letters !== undefined)
          .map((target) => [target.lessonId, target.letters]),
      ),
    ).toEqual({
      "SA-W03-mama-nama-delayed-copy": ["मम", "नाम"],
      "SA-W03-mama-nama-guided-copy": ["मम", "नाम"],
    });

    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "SA-W03-namah-guided-copy",
      "SA-W10-asti-guided-copy",
      "HI-W05-write-namaste",
      "MR-W03-dhanyavad-write",
      "MR-W01-ho-delayed-copy",
      "HI-A1F01-name-supported",
      "HI-W04-write-mera-naam",
      "MR-A1M01-reader-greeting",
      "MW-C07-read-later",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

});
