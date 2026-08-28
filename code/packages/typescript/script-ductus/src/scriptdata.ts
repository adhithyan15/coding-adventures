/// <reference path="./virtual-script-inventories.d.ts" />

// ---------------------------------------------------------------------------
// scriptdata.ts — the curriculum's own script files, and the shapes they hold
// ---------------------------------------------------------------------------
//
// The ONLY place that reaches out to the curriculum's canonical script files.
// Ordinary inventories are imported directly. Canonical shard sets
// arrive through one fixed build-time virtual module. Neither route copies the
// curriculum, so readers see exactly what the lessons teach from.
//
// This lives beside the pen paths on purpose. `strokes.ts` claims a stroke
// ORDER for a letter and cites a source for it; these files are where that
// same letter's citation is recorded for the app and the books. Keeping the
// two in one package is what lets a test assert they agree -- that every
// cited letter resolves back to the script that owns it, and to the font its
// pen path was verified against.
//
// Direct paths climb out of this package into data/scripts/. Vite bundles the
// ordinary JSON and folds the selected `.d/` directories at build time; the
// app's `server.fs.allow` lets the dev server read the curriculum root.

import cyrillic from "../../../../learning/human-languages/data/scripts/cyrillic.json";
import hebrew from "../../../../learning/human-languages/data/scripts/hebrew.json";
import chinese from "../../../../learning/human-languages/data/scripts/chinese.json";
import arabic from "../../../../learning/human-languages/data/scripts/arabic.json";
import {
  japanese as japaneseInventory,
  persoArabic as persoArabicInventory,
  tamil as tamilInventory,
  urduNastaliq as urduNastaliqInventory,
} from "virtual:script-ductus-inventories";
import devanagari from "../../../../learning/human-languages/data/scripts/devanagari.json";
import gujarati from "../../../../learning/human-languages/data/scripts/gujarati.json";
// The Dravidian syllabaries (Telugu/Kannada/Malayalam) are GENERATED from Unicode
// by data/scripts/generate_syllabary.py — each "letter" is a base consonant
// composed with a vowel sign (ka, ki, ku, kha, …), for reading-recognition.
import kannada from "../../../../learning/human-languages/data/scripts/kannada.json";
import telugu from "../../../../learning/human-languages/data/scripts/telugu.json";
import malayalam from "../../../../learning/human-languages/data/scripts/malayalam.json";

/** One base letter (or character/radical) of a script. */
export interface Letter {
  /** The citation form as written. */
  glyph: string;
  /** Romanization / phonetic value, e.g. "v (as in 'van')". */
  sound: string;
  /** consonant | vowel | letter | logograph | … */
  role: string;
  /** Tonal scripts (Mandarin): the canonical tone. */
  tone?: string;
  /** Abugida: the vowel a bare consonant carries. */
  inherentVowel?: string;
  /** Cursive/abjad positional forms (Arabic). */
  forms?: Record<string, string>;
  /** The literal "pieces" a learner draws — the heart of "break it apart". */
  components: string[];
  /** Named shape parts in their usual writing order; not a pen-lift count. */
  strokeOrder: string[];
  strokeOrderNote: string;
  /** Present together only when a cited, font-checked ductus verifies the claim. */
  penLifts?: number;
  strokeOrderSource?: { citation: string; url: string; variation?: string };
  /** Free-text notes; may flag a "FALSE FRIEND" of a Latin letter. */
  notes?: string;
}

/** A combining sign, including source-backed carrier-composition metadata. */
export interface Mark {
  mark: string;
  sound: string;
  role: string;
  attachesAs: string;
  example?: { base: string; combined: string; sound: string };
  examples?: Array<{
    base: string;
    combined: string;
    sound: string;
    carrier: string;
    note?: string;
  }>;
  compositionOrder?: string[];
  compositionSource?: { citation: string; url: string; variation?: string };
}

/** An obligatory joined shape whose editable text remains existing letters. */
export interface Ligature {
  sequence: string;
  displayGlyph: string;
  sound: string;
  role: string;
  forms: Record<string, string>;
  components: string[];
  strokeOrder: string[];
  strokeOrderNote: string;
  penLifts?: number;
  strokeOrderSource?: { citation: string; url: string; variation?: string };
  notes?: string;
}

/** A whole script: its metadata plus its inventory of letters. */
export interface ScriptData {
  script: string;
  name: string;
  font: string;
  direction: "ltr" | "rtl";
  system: string; // alphabet | abugida | abjad | logographic | …
  /** The one visual feature that gives this script away at a glance — for a
   *  "spot the script" identification mode. Verified by rendering the font. */
  signature?: string;
  letters: Letter[];
  ligatures?: Ligature[];
  /** For an abugida: the independent (word-initial) vowels — the letters a word
   *  writes when it BEGINS with a vowel (అ a, ఆ ā), as opposed to the vowel signs
   *  that ride on a consonant. Kept separate from `letters` so the consonant
   *  syllabary (and anything keyed on it being all-syllables) is untouched. */
  independentVowels?: Letter[];
  /** Atomic vowel-free consonants used in final position, kept outside a
   *  generated all-syllable grid so matrix and sibling invariants stay intact. */
  finalConsonants?: Letter[];
  /** For a script with its own numerals: the digits 0–9 in the script's glyphs
   *  (౦౧౨…). Kept separate from `letters`, like `independentVowels`. */
  digits?: Letter[];
  marks?: Mark[];
  combination?: string;
  complete?: boolean;
  notes?: string;
}

const japanese = japaneseInventory as ScriptData;
const persoArabic = persoArabicInventory as ScriptData;
const tamil = tamilInventory as ScriptData;
const urduNastaliq = urduNastaliqInventory as ScriptData;

// The JSON files are authored to the ScriptData shape; assert it once here.
export const SCRIPTS: ScriptData[] = [
  cyrillic as ScriptData,
  hebrew as ScriptData,
  chinese as ScriptData,
  arabic as ScriptData,
  persoArabic as ScriptData,
  urduNastaliq as ScriptData,
  devanagari as ScriptData,
  gujarati as ScriptData,
  tamil as ScriptData,
  japanese as ScriptData,
  // The Dravidian trio, in chain order (Tamil → Kannada → Telugu → Malayalam).
  kannada as ScriptData,
  telugu as ScriptData,
  malayalam as ScriptData,
];

/** Resolve a cited letter back to the exact canonical script font that owns it. */
export function verifiedLetterFont(
  glyph: string,
  sourceUrl: string,
): string | undefined {
  return SCRIPTS.find((script) =>
    [
      ...script.letters,
      ...(script.independentVowels ?? []),
      ...(script.finalConsonants ?? []),
    ].some(
      (letter) =>
        letter.glyph === glyph && letter.strokeOrderSource?.url === sourceUrl,
    ),
  )?.font;
}
