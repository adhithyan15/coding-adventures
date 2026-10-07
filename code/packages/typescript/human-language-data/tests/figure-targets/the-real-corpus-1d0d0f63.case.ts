// HL-C443 — a single-letter writing lesson gets its filmstrip without anyone
// declaring it, and every filmstrip the curriculum resolves is actually PRINTED.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { resolvedFigureTargets } from "../../src/figure-cli.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import { defaultCurriculumRoot, loadLessons } from "../../src/loader.js";
import type { ParsedLesson } from "../../src/parse.js";
import { loadFilmstripTargetCountPins } from "../filmstrip-target-count-pins.js";

describe("the real corpus", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadLessons(root);
  const targets = resolvedFigureTargets(root, lessons).filter(
    (target): target is ScriptFilmstripTarget => target.kind === "script-filmstrip",
  );

  it("draws a filmstrip for every Tamil letter lesson whose letter has a cited ductus", () => {
    // 38 one-glyph lessons, 36 glyphs. வ has both TA-S01-va and the guided
    // copy TA-W00, and the puḷḷi ் both TA-S08-pulli and TA-W03-pulli-vanakkam,
    // while a letter lesson without cited ductus remains an undrawn
    // candidate. Seven of the 36 are signs taught by themselves (ா ி ீ ெ ே ை
    // and the puḷḷi ்); the signs ு and ூ have no cited ductus and stay
    // undrawn.
    const tamil = targets.filter((target) => target.lessonId.startsWith("TA-"));
    const single = tamil.filter((target) => target.letters === undefined);
    expect(single).toHaveLength(38);
    expect(new Set(single.map((target) => target.glyph)).size).toBe(36);
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

  it("draws every Gujarati sign lesson, and every Gujarati word or list whose signs are cited", () => {
    // Nine lessons teach a sign by itself; seven word lessons (four words, and
    // હા three times) and six lists carry one: 22 in all. All are drawn
    // consonant first, sign after: KanoAI's barakhadi templates write every
    // sign after its consonant, even િ.
    const gujarati = targets.filter((target) => target.lessonId.startsWith("GU-"));
    const withSign = gujarati.filter((target) =>
      (target.letters ?? [target.glyph]).some((piece) => /^\p{M}+$/u.test(piece)),
    );
    expect(
      Object.fromEntries(
        withSign.map((target) => [target.lessonId, (target.letters ?? [target.glyph]).join(" ")]),
      ),
    ).toEqual({
      "GU-C32-ane-write": "અ ન ે",
      "GU-C33-ke-write": "ક ે",
      "GU-C34-kemke-write": "ક ે મ ક ે",
      "GU-C36-te-write": "ત ે",
      "GU-R04-doorway-nine-r2": "જ ો ં ી ુ છ ક ણ શ",
      "GU-R04-first-four-r1": "ળ થ અ િ",
      "GU-R07-second-four-r1": "ૂ ટ ઈ ઢ",
      "GU-R13-doorway-nine-r3": "ણ જ ુ શ ં ક ો છ ી",
      "GU-R15-u-matra-r4": "ુ ી",
      "GU-R19-doorway-nine-r4": "ો ક ં જ ી ણ છ શ ુ",
      "GU-W01-aa-matra": "ા",
      "GU-W01-e-matra": "ે",
      "GU-W01-haa-delayed-copy": "હ ા",
      "GU-W01-haa-dictation": "હ ા",
      "GU-W01-haa-guided-copy": "હ ા",
      "GU-W03-anusvara": "ં",
      "GU-W03-ii-matra": "ી",
      "GU-W03-o-matra": "ો",
      "GU-W03-u-matra": "ુ",
      "GU-W04-i-matra": "િ",
      "GU-W07-uu-matra": "ૂ",
      "GU-W45-ai-sign": "ૈ",
    });
    // The virama ્ and the vocalic-r sign ૃ have no Gujarati source, so their
    // own lessons and every word that carries one stay undrawn. જો is refused
    // because the bundled font joins the ā bar of ો to જ.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "GU-W01-virama",
      "GU-W02-vocalic-r",
      "GU-W01-namaste-read",
      "GU-C37-kyaan-write",
      "GU-C35-jo-write",
      "GU-R23-route-three-r1",
      "GU-R23-shaalaa-rasto-r2",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws a Devanagari sign only where a lesson teaches it alone", () => {
    // 32 Hindi, Marathi, Sanskrit and Marwadi lessons teach one of eight
    // signs by itself (ु ू े ं ़ ् ृ ँ), each cited to native writers who wrote
    // it alone. Each is one glyph, never a sequence: Devanagari has no
    // written-order table, so no sign is ever placed against a consonant.
    const devanagari = targets.filter((target) => target.script === "devanagari");
    const signs = devanagari.filter((target) => /^\p{M}+$/u.test(target.glyph));
    expect(signs.every((target) => target.letters === undefined)).toBe(true);
    expect(Object.fromEntries(signs.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "HI-S05-sign-virama": "्",
      "HI-S112-vowel-sign-e": "े",
      "HI-S118-sign-candrabindu": "ँ",
      "HI-S120-vowel-sign-u": "ु",
      "HI-S140-nuqta": "़",
      "HI-S144-vowel-sign-vocalic-r": "ृ",
      "HI-S148-sign-anusvara": "ं",
      "HI-S149-vowel-sign-uu": "ू",
      "HI-W12-chandrabindu": "ँ",
      "HI-W12-u-matra": "ु",
      "MR-W01-virama": "्",
      "MR-W02-anusvara": "ं",
      "MR-W02-e-matra": "े",
      "MR-W05-candrabindu": "ँ",
      "MR-W05-ru-matra": "ृ",
      "MR-W05-u-matra": "ु",
      "MR-W05-uu-matra": "ू",
      "MW-W03-anusvara": "ं",
      "MW-W05-virama": "्",
      "MW-W06-uu-matra": "ू",
      "MW-W07-e-matra": "े",
      "MW-W13-u-matra": "ु",
      "MW-W15-nukta": "़",
      "SA-S05-sign-virama": "्",
      "SA-S112-vowel-sign-e": "े",
      "SA-S203-vowel-sign-u": "ु",
      "SA-S205-vowel-sign-vocalic-r": "ृ",
      "SA-S209-sign-anusvara": "ं",
      "SA-S222-vowel-sign-uu": "ू",
      "SA-W05-vocalic-r-delayed-copy": "ृ",
      "SA-W05-vocalic-r-dictation": "ृ",
      "SA-W05-vocalic-r-guided-copy": "ृ",
    });
    // Left undrawn: the signs Noto prints with a piece of headline the traces
    // never draw (ा ि ी ो ः), the signs whose traces split (ै ौ), a sign
    // lesson with no Writing or Script block (HI-W03-preposed-i), and every
    // word or list that puts a sign on a consonant.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "HI-S06-vowel-sign-aa",
      "HI-W128-vowel-sign-i",
      "HI-S116-vowel-sign-ii",
      "HI-S150-vowel-sign-o",
      "HI-S134-vowel-sign-au",
      "HI-W12-ai-matra",
      "MR-W02-visarga",
      "SA-S201-sign-visarga",
      "HI-W03-preposed-i",
      "HI-W03-matras-naam",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("composes words only in scripts whose letters stand apart", () => {
    // Words whose pieces spell the headword back: four Japanese words (ありがとう
    // joined once が gained its ductus), the
    // twelve Tamil words whose signs are all written AFTER their consonant
    // (சரி, சரியா, and ten words whose puḷḷi follows its consonant, such as
    // வணக்கம் and நீங்கள்), and the Gujarati words, whose signs are all written after their
    // consonant (અને, કે, કેમકે, તે, and હા three times), and the Malayalam
    // word നമ three times, once ന and മ gained their Thooval-cited ductus
    // (its fourth lesson, ML-W01-na-ma-trace, lists "ന മ" and is not a
    // word). A Tamil word
    // with a sign written before its consonant is drawn in written order,
    // which does not spell it back (see above). The Arabic family (سلام)
    // and Cyrillic (привет) have fully cited words that are deliberately
    // NOT drawn: see SEPARATE_LETTER_SCRIPTS for why each would draw
    // something false. A Devanagari word (मम) is not a sequence either; it
    // is drawn as ONE composed entry with a shared headline (next case).
    const words = targets.filter(
      (target) => target.letters !== undefined && target.letters.join("") === target.glyph,
    );
    expect(words.map((target) => target.lessonId).sort()).toEqual([
      // German print letters stand apart: weil, once its letters gained a
      // Grundschrift-cited ductus. Großschreibung is cited letter for letter
      // too, but at 14 pieces it is past MAX_SEQUENCE_PIECES and prints none.
      "GE-W04-vier-zeilen",
      "GU-C32-ane-write",
      "GU-C33-ke-write",
      "GU-C34-kemke-write",
      "GU-C36-te-write",
      "GU-W01-haa-delayed-copy",
      "GU-W01-haa-dictation",
      "GU-W01-haa-guided-copy",
      "JA-W01-hai-read",
      "JA-W01-konnichiwa-read",
      "JA-W03-arigatou-read",
      "JA-W08-sayounara-read",
      "ML-W01-na-ma-delayed-copy",
      "ML-W01-na-ma-dictation",
      "ML-W01-na-ma-guided-copy",
      "TA-W03-write-vanakkam",
      "TA-W05-write-aam",
      "TA-W07-write-sari",
      "TA-W08-read-en",
      "TA-W10-read-naan",
      "TA-W11-read-niingal",
      "TA-W16-read-tamizh",
      "TA-W18-read-uur",
      "TA-W27-read-payam",
      "TA-W30-read-sariyaa",
      "TA-W31-read-aanaal",
      "TA-W33-read-een",
    ]);
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of ["UR-W04-joining", "RU-W05-privet-guided-copy"]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws a Devanagari word as its letters' bodies and one shared headline", () => {
    // The only Devanagari writing headwords that are one word of bare letters
    // are Sanskrit मम, three times. Every other Devanagari word carries a
    // vowel sign, a virama, a nasal or a visarga, none of which has a cited
    // written order against its consonant or the shared headline, so it is
    // not even a candidate (नाम, नमस्ते, नमः, धन्यवाद, ...).
    const shared = targets.filter((target) => target.composition === "shared-headline");
    expect(Object.fromEntries(shared.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "SA-W03-mama-delayed-copy": "मम",
      "SA-W03-mama-dictation": "मम",
      "SA-W03-mama-guided-copy": "मम",
    });
    expect(shared.every((target) => target.letters === undefined && target.script === "devanagari")).toBe(true);
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "SA-W03-mama-nama-guided-copy",
      "SA-W03-namah-guided-copy",
      "SA-W10-asti-guided-copy",
      "HI-W05-write-namaste",
      "HI-W12-schwa-drop",
      "MR-W03-dhanyavad-write",
      "MW-W01-saa",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws the Latin lessons whose every letter is cited, and no word with an a", () => {
    // Four lessons: ñ, the list "¿ ¡", ß, and the German word weil.
    // Großschreibung's letters are all cited, but 14 pieces is past
    // MAX_SEQUENCE_PIECES, so it prints no strip rather than an illegible one. Every other Spanish and German writing lesson holds an a
    // (Noto's two-storey a has no source), an accent whose only lesson also
    // holds one, punctuation inside a word, or an uncited mark (ä ö).
    const latin = targets.filter((target) => target.script === "latin");
    expect(
      Object.fromEntries(latin.map((target) => [target.lessonId, (target.letters ?? [target.glyph]).join(" ")])),
    ).toEqual({
      "ES-W02-enye": "ñ",
      "ES-W03-inverted": "¿ ¡",
      "GE-W01-eszett": "ß",
      "GE-W04-vier-zeilen": "w e i l",
    });
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "ES-W00-hola-guided-copy",
      "ES-W01-acento",
      "ES-W01-frase-propia",
      "ES-W02-cuatro-lineas-ayer",
      "ES-W02-enye-formas",
      "ES-C03-como-acento",
      "ES-W03-question-span",
      "GE-W01-hallo-guided-copy",
      "GE-W02-umlauts",
      "FR-W01-salut-guided-copy",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws every switched-on track exactly the letters its ductus cites", () => {
    const counts: Record<string, number> = {};
    for (const target of targets) {
      const prefix = target.lessonId.split("-")[0]!;
      counts[prefix] = (counts[prefix] ?? 0) + 1;
    }
    const pins = loadFilmstripTargetCountPins(
      join(dirname(fileURLToPath(import.meta.url)), "..", "filmstrip-target-counts"),
    );
    const byTrack = Object.fromEntries(
      Object.entries(counts).map(([prefix, count]) => [
        targets.find((target) => target.lessonId.startsWith(`${prefix}-`))!.output.split("/", 1)[0]!,
        count,
      ]),
    );
    expect(byTrack).toEqual(pins);
  });

  it("prints every resolved filmstrip in its chapter", () => {
    // A generated figure that is never placed is a figure no reader sees.
    const byLesson = new Map(lessons.map((entry) => [entry.realization.lessonId, entry]));
    for (const target of targets) {
      const owner = byLesson.get(target.lessonId);
      expect(owner, target.lessonId).toBeDefined();
      const chapterFile = chapterTexFor(root, owner!);
      const pdf = target.output.split("/").pop()!.replace(/\.svg$/, ".pdf");
      expect(readFileSync(chapterFile, "utf8"), `${target.lessonId} in ${chapterFile}`).toContain(pdf);
    }
  });
});

function chapterTexFor(root: string, owner: ParsedLesson): string {
  const targets = JSON.parse(
    readFileSync(
      join(
        root,
        "core",
        "book-generation.d",
        "targets.d",
        `${owner.language}-${String(owner.realization.chapter).padStart(4, "0")}.json`,
      ),
      "utf8",
    ),
  ) as { output: string };
  return join(root, targets.output);
}
