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
    // Ten lessons teach a sign by itself, and thirteen word lessons (eleven
    // words, and હા twice) carry one: 23 in all. The six lists that carry one
    // (GU-R04 … GU-R19) and હા's dictation are writing from sound, so they
    // print no strip (no-strip-above-a-dictation). All are drawn
    // consonant first, sign after: KanoAI's barakhadi templates write every
    // sign after its consonant, even િ. GU-W21-ai-matra and the seven place
    // words of chapters 20-23 (મંદિર … ગામ) have no Writing block: their strip
    // lands in their guided- or delayed-copy practice (figure-targets.ts).
    const gujarati = targets.filter((target) => target.lessonId.startsWith("GU-"));
    const withSign = gujarati.filter((target) =>
      (target.letters ?? [target.glyph]).some((piece) => /^\p{M}+$/u.test(piece)),
    );
    expect(
      Object.fromEntries(
        withSign.map((target) => [target.lessonId, (target.letters ?? [target.glyph]).join(" ")]),
      ),
    ).toEqual({
      "GU-C20-mandir": "મ ં દ િ ર",
      "GU-C21-haath": "હ ા થ",
      "GU-C21-paisa": "પ ૈ સ ા",
      "GU-C22-shaalaa": "શ ા ળ ા",
      "GU-C22-shahar": "શ હ ે ર",
      "GU-C23-dukaan": "દ ુ ક ા ન",
      "GU-C23-gaam": "ગ ા મ",
      "GU-C32-ane-write": "અ ન ે",
      "GU-C33-ke-write": "ક ે",
      "GU-C34-kemke-write": "ક ે મ ક ે",
      "GU-C36-te-write": "ત ે",
      "GU-W01-aa-matra": "ા",
      "GU-W01-e-matra": "ે",
      "GU-W01-haa-delayed-copy": "હ ા",
      "GU-W01-haa-guided-copy": "હ ા",
      "GU-W03-anusvara": "ં",
      "GU-W03-ii-matra": "ી",
      "GU-W03-o-matra": "ો",
      "GU-W03-u-matra": "ુ",
      "GU-W04-i-matra": "િ",
      "GU-W07-uu-matra": "ૂ",
      "GU-W21-ai-matra": "ૈ",
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
    // 35 Hindi, Marathi, Sanskrit and Marwadi lessons teach one of nine
    // signs by itself (ा ु ू े ं ़ ् ृ ँ), each cited to native writers who
    // wrote it alone. Each is one glyph, never a sequence: Devanagari has no
    // written-order table, so no sign is placed against a consonant here
    // (ā joins its consonant only inside a composed word, next cases).
    const devanagari = targets.filter((target) => target.script === "devanagari");
    const signs = devanagari.filter((target) => /^\p{M}+$/u.test(target.glyph));
    expect(signs.every((target) => target.letters === undefined)).toBe(true);
    expect(Object.fromEntries(signs.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "HI-S05-sign-virama": "्",
      "HI-S06-vowel-sign-aa": "ा",
      "HI-S112-vowel-sign-e": "े",
      "HI-S118-sign-candrabindu": "ँ",
      "HI-S120-vowel-sign-u": "ु",
      "HI-S140-nuqta": "़",
      "HI-S144-vowel-sign-vocalic-r": "ृ",
      "HI-S148-sign-anusvara": "ं",
      "HI-S149-vowel-sign-uu": "ू",
      "HI-W12-chandrabindu": "ँ",
      "HI-W12-u-matra": "ु",
      "MR-W01-aa-matra": "ा",
      "MR-W01-virama": "्",
      "MR-W02-anusvara": "ं",
      "MR-W02-e-matra": "े",
      "MR-W05-candrabindu": "ँ",
      "MR-W05-ru-matra": "ृ",
      "MR-W05-u-matra": "ु",
      "MR-W05-uu-matra": "ू",
      "MW-W01-aa-matra": "ा",
      "MW-W03-anusvara": "ं",
      "MW-W05-virama": "्",
      "MW-W06-uu-matra": "ू",
      "MW-W07-e-matra": "े",
      "MW-W13-u-matra": "ु",
      "MW-W15-nukta": "़",
      "SA-S05-sign-virama": "्",
      "SA-S06-vowel-sign-aa": "ा",
      "SA-S112-vowel-sign-e": "े",
      "SA-S203-vowel-sign-u": "ु",
      "SA-S205-vowel-sign-vocalic-r": "ृ",
      "SA-S209-sign-anusvara": "ं",
      "SA-S222-vowel-sign-uu": "ू",
      "SA-W05-vocalic-r-delayed-copy": "ृ",
      "SA-W05-vocalic-r-guided-copy": "ृ",
    });
    // Left undrawn: the signs Noto prints with a piece of headline the traces
    // never draw (ि ी ो ः), the signs whose traces split (ै ौ), a sign
    // lesson with no Writing or Script block and no modelled practice stage
    // (HI-W03-preposed-i), and every word or list that puts a sign on a
    // consonant.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
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
    // consonant (અને, કે, કેમકે, તે, and હા twice, and the eight place
    // words ઘર … ગામ of chapters 20-23, drawn in their copy practice since
    // strips may land in a modelled practice block), and the Malayalam
    // word നമ twice, once ന and മ gained their Thooval-cited ductus
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
      // Latin print letters stand apart: every word whose a waited for an
      // outline with a one-storey a (hola, salut, ciao, olá, Hallo), in its
      // observe and copy lessons. Their dictations, and the composition
      // headwords weil, quia and ayer, print none: a dictation or composition
      // block shows no model. Großschreibung is cited letter for letter too,
      // but at 14 pieces it is past MAX_SEQUENCE_PIECES and prints none.
      "ES-W00-hola-delayed-copy",
      "ES-W00-hola-guided-copy",
      "ES-W00-hola-observe",
      "FR-W01-salut-delayed-copy",
      "FR-W01-salut-guided-copy",
      "FR-W01-salut-observe",
      "GE-W01-hallo-delayed-copy",
      "GE-W01-hallo-guided-copy",
      "GU-C20-ghar",
      "GU-C20-mandir",
      "GU-C21-haath",
      "GU-C21-paisa",
      "GU-C22-shaalaa",
      "GU-C22-shahar",
      "GU-C23-dukaan",
      "GU-C23-gaam",
      "GU-C32-ane-write",
      "GU-C33-ke-write",
      "GU-C34-kemke-write",
      "GU-C36-te-write",
      "GU-W01-haa-delayed-copy",
      "GU-W01-haa-guided-copy",
      "IT-W01-ciao-delayed-copy",
      "IT-W01-ciao-guided-copy",
      "JA-W01-hai-read",
      "JA-W01-konnichiwa-read",
      "JA-W03-arigatou-read",
      "JA-W08-sayounara-read",
      "ML-W01-na-ma-delayed-copy",
      "ML-W01-na-ma-guided-copy",
      "PT-W01-ola-delayed-copy",
      "PT-W01-ola-guided-copy",
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
    // The Devanagari writing headwords that are one word of bare letters, or
    // of letters and the ā sign straight after a consonant: Sanskrit मम (two
    // copy lessons; its dictation prints none), Hindi नाम (two) and Marwadi सा. ā is the one sign whose place
    // against its consonant and the headline is cited (its mark record cites
    // the cited आ). Every other word carries a sign, a virama, a nasal or a
    // visarga with no cited place, so it is not even a candidate (नमस्ते, नमः,
    // धन्यवाद, हो, ...).
    const shared = targets.filter((target) => target.composition === "shared-headline");
    const words = shared.filter((target) => target.letters === undefined);
    expect(Object.fromEntries(words.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "HI-A1F01-name-label": "नाम",
      "HI-W12-schwa-drop": "नाम",
      "MW-W01-saa": "सा",
      "SA-W03-mama-delayed-copy": "मम",
      "SA-W03-mama-guided-copy": "मम",
    });
    expect(shared.every((target) => target.script === "devanagari")).toBe(true);
    // A phrase of such words, separated by single spaces, is drawn word by
    // word, each word with its own headline: Sanskrit मम नाम, its two copy
    // lessons.
    const phrases = shared.filter((target) => target.letters !== undefined);
    expect(Object.fromEntries(phrases.map((target) => [target.lessonId, target.letters]))).toEqual({
      "SA-W03-mama-nama-delayed-copy": ["मम", "नाम"],
      "SA-W03-mama-nama-guided-copy": ["मम", "नाम"],
    });
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "SA-W03-namah-guided-copy", // ः
      "SA-W10-asti-guided-copy", // conjunct
      "HI-W05-write-namaste", // conjunct
      "MR-W03-dhanyavad-write", // conjunct
      "MR-W01-ho-delayed-copy", // ो
      "HI-A1F01-name-supported", // a label: "नाम: मीरा"
      "HI-W04-write-mera-naam", // े in मेरा
      "MR-A1M01-reader-greeting", // a sentence: "नमस्कार मीरा."
      "MW-C07-read-later", // े and ू: "पाछे मिलसू"
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("draws the Latin lessons whose every letter is cited, one-storey a included", () => {
    // 16 lessons over five Latin-script tracks (Latin's one, a composition,
    // prints none now that a composition block takes no strip). The a waited for an outline
    // that prints the one-storey a every source teaches (LatinPrint-Subset.ttf,
    // from SIL's Andika); the acute vowels follow UJIpenchars2's writers.
    const latin = targets.filter((target) => target.script === "latin");
    expect(
      Object.fromEntries(latin.map((target) => [target.lessonId, (target.letters ?? [target.glyph]).join(" ")])),
    ).toEqual({
      "ES-W00-hola-delayed-copy": "h o l a",
      "ES-W00-hola-guided-copy": "h o l a",
      "ES-W00-hola-observe": "h o l a",
      "ES-W01-acento": "á é í ó ú",
      "ES-W02-enye": "ñ",
      "ES-W03-inverted": "¿ ¡",
      "FR-W01-salut-delayed-copy": "s a l u t",
      "FR-W01-salut-guided-copy": "s a l u t",
      "FR-W01-salut-observe": "s a l u t",
      "GE-W01-eszett": "ß",
      "GE-W01-hallo-delayed-copy": "H a l l o",
      "GE-W01-hallo-guided-copy": "H a l l o",
      "IT-W01-ciao-delayed-copy": "c i a o",
      "IT-W01-ciao-guided-copy": "c i a o",
      "PT-W01-ola-delayed-copy": "o l á",
      "PT-W01-ola-guided-copy": "o l á",
    });
    // Still undrawn: an uncited mark (è ê ç ï ë ä ö ē œ), punctuation inside
    // a word, a slash between two words, or more than MAX_SEQUENCE_PIECES.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "FR-W01-accents",
      "FR-W02-cedille",
      "FR-W03-trema",
      "FR-C10-oe",
      "GE-W02-umlauts",
      "GE-W03-capitalization",
      "LA-W01-salve-delayed-copy",
      "LA-W01-salve-guided-copy",
      "ES-C03-como-acento",
      "ES-W01-tilde-diacritica",
      "ES-W02-enye-formas",
      "ES-W03-question-span",
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
    const byTrack: Record<string, number> = Object.fromEntries(
      Object.entries(counts).map(([prefix, count]) => [
        targets.find((target) => target.lessonId.startsWith(`${prefix}-`))!.output.split("/", 1)[0]!,
        count,
      ]),
    );
    // A pinned track may draw nothing at all (latin, whose one strip was on a
    // composition), and a pin of 0 still has to hold.
    for (const track of Object.keys(pins)) byTrack[track] ??= 0;
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
