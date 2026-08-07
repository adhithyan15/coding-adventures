// ---------------------------------------------------------------------------
// matrix.ts — an abugida's syllables as the consonant × vowel TABLE they are.
//
// A Dravidian syllabary isn't a flat list of ~450 signs; it's a grid. Every
// consonant marches across the SAME vowel row — ka kā ki kī … , kha khā khi … ,
// ga gā gi … — so once you see one row, the next is the same shape with a new
// starting consonant. The flat Browse list hides that regularity; laid out as a
// grid (rows = consonants, columns = vowels) the pattern is the first thing you
// see, which is the whole point of "build pattern recognition slowly".
//
// This module is the pure layout. It knows nothing about the DOM — it turns the
// generated, consonant-major syllable list into rows and vowel columns, reusing
// the grounded consonant boundary from syllabary.ts. Nothing here is invented:
// the glyphs are the generated syllables, the column vowels are read off the
// first consonant's own row (its sound minus its consonant), and if the rows
// don't all span the same vowels it refuses to build a grid at all — a
// misaligned cell would sit a syllable under the wrong vowel header, exactly the
// silent mislabel a native reader would trust. Unit-tested, with a control that
// a ragged input yields no matrix.
// ---------------------------------------------------------------------------

import { consonantGroups } from "./syllabary.ts";
import { specialConsonant } from "./core.ts";

/** The minimal syllable shape the matrix needs. */
interface MatrixLetter {
  sound: string;
  glyph: string;
  role: string;
  components: string[];
  inherentVowel?: string;
}

/** One cell: the syllable glyph, its romanization, and its index in the flat
 *  letter list (so the UI can select it and open the existing detail panel). */
export interface MatrixCell {
  index: number;
  glyph: string;
  sound: string;
}

/** The whole table: the shared vowel column headers, and one row per consonant. */
export interface SyllableMatrix {
  /** ISO-15919 vowel per column (a, ā, i, …, ai, au, r̥) — read off the data. */
  vowels: string[];
  /** Rows in consonant-major order; `label` is the consonant's inherent-"a"
   *  form (ka, kha, ḷa), `cells` its syllables across the vowel columns, and
   *  `special` marks the retroflex/alveolar consonants (ḷ/ṟ/ṉ) — the rows a
   *  reader confuses with the ordinary la/ra/na. */
  rows: { label: string; cells: MatrixCell[]; special: boolean }[];
}

/**
 * Lay the generated syllabary out as a consonant × vowel grid, or null if it
 * isn't a clean rectangle.
 *
 * The consonant boundary is the grounded one from `consonantGroups` (a new row
 * starts at each bare consonant). The column vowels are taken from the first
 * consonant's own row — its base syllable's sound minus its inherent vowel gives
 * the consonant prefix (ka → "k"), and stripping that prefix off each syllable
 * in the row yields the vowel it carries (kā → "ā", ki → "i", kr̥ → "r̥"). Every
 * consonant is generated across the same vowels, so column j is the same vowel
 * in every row; if that invariant is broken (a ragged row), we return null
 * rather than risk mislabelling a cell.
 */
export function buildSyllableMatrix(letters: MatrixLetter[]): SyllableMatrix | null {
  const groups = consonantGroups(letters);
  if (groups.length === 0) return null;

  const width = groups[0]!.length;
  if (!groups.every((g) => g.length === width)) return null; // ragged → no grid

  const base = letters[groups[0]![0]!]!;
  const iv = base.inherentVowel ?? "a";
  const prefix = base.sound.slice(0, Math.max(0, base.sound.length - iv.length));

  const vowels = groups[0]!.map((i) => {
    const s = letters[i]!.sound;
    return s.startsWith(prefix) ? s.slice(prefix.length) : s;
  });

  const rows = groups.map((g) => {
    const label = letters[g[0]!]!.sound;
    return {
      label,
      // Reuse the tested false-friend classifier so the matrix flags the same
      // retroflex/alveolar rows the Browse tiles do — no separate judgement here.
      special: specialConsonant({ sound: label }) !== null,
      cells: g.map((i) => ({ index: i, glyph: letters[i]!.glyph, sound: letters[i]!.sound })),
    };
  });

  return { vowels, rows };
}
