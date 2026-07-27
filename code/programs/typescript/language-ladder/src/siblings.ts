// siblings.ts — the SAME syllable, as written in the other Dravidian scripts.
//
// The learner's chain ends with four Dravidian cousins — Tamil → Kannada →
// Telugu → Malayalam — and the whole point of learning them together (the
// spiral model) is that the *connections* become the memory hooks. Telugu,
// Kannada and Malayalam are especially close: the syllable ki is written కి
// (Telugu), ಕಿ (Kannada), കി (Malayalam) — three different shapes for one and
// the same sound. Once you can read one, the other two are a short hop, and the
// fastest way to feel that is to see the three side by side.
//
// So when you're looking at a syllable in Browse, this helper finds that same
// syllable — matched by its romanization — in the sibling syllabaries, and the
// detail panel shows the sibling glyphs next to it. Nothing here is invented:
// every sibling glyph is a real letter already present in another script's
// generated data, pulled out by an exact `sound` match.
//
// Why the match is safe. Telugu, Kannada and Malayalam are all produced by the
// one generator (generate_syllabary.py) from the same ISO-15919 scheme, so a
// given syllable carries a byte-identical `sound` across the three ("ki" is
// "ki" everywhere). We therefore compare sounds by exact string equality — no
// fuzzy matching, no transliteration guesswork.
//
// Why we restrict to syllabaries. Only the generated Dravidian trio has every
// letter tagged role "syllable" (see `isSyllabary`). Tamil, Devanagari and
// Gujarati are also abugidas, but their data models a *consonant* and a
// *vowel-sign* separately (roles "consonant" / "independent-vowel"), never a
// pre-composed ka/ki/ku — so their "ka" is a different pedagogical unit, and
// cross-matching it would be misleading. Restricting the sibling search to
// scripts that are wholly syllabic keeps the comparison honest and apples-to-
// apples. (Tamil is thus deliberately absent here; its precomposed-syllable
// view would need its own, separate slice.)

import type { ScriptData } from "./types.ts";
import { isSyllabary } from "./syllabary.ts";

/** One sibling rendering of a syllable: the same sound, a different script. */
export interface Sibling {
  /** The sibling script's id, e.g. "kannada". */
  script: string;
  /** Its human name, e.g. "Kannada" — what the panel labels the glyph with. */
  name: string;
  /** The syllable as that script writes it, e.g. "ಕಿ". */
  glyph: string;
  /** The shared romanization (identical to the source syllable's), e.g. "ki". */
  sound: string;
}

/**
 * The same syllable, as written in the OTHER Dravidian syllabaries.
 *
 * Given a syllable's romanization and the id of the script you're currently
 * reading, scan every *other* script and, for each one that is wholly syllabic,
 * return the glyph whose `sound` matches exactly. Scripts with no match — e.g.
 * Malayalam's alveolar-n row (ṉa, ṉi, …), which simply has no Telugu/Kannada
 * counterpart — contribute nothing, which is correct.
 *
 * Pure: it reads its arguments and allocates a fresh array; it touches no
 * module state, mutates nothing, and never looks at the current script's own
 * letters.
 *
 *   crossScriptSiblings("ki", "telugu", SCRIPTS)
 *     → [ {script:"kannada", name:"Kannada", glyph:"ಕಿ", sound:"ki"},
 *         {script:"malayalam", name:"Malayalam", glyph:"കി", sound:"ki"} ]
 */
export function crossScriptSiblings(
  sound: string,
  currentScript: string,
  allScripts: ScriptData[],
): Sibling[] {
  const out: Sibling[] = [];
  for (const data of allScripts) {
    if (data.script === currentScript) continue; // never echo the source script
    if (!isSyllabary(data.letters)) continue; // only the fully-syllabic cousins
    const match = data.letters.find((l) => l.sound === sound);
    if (match) {
      out.push({ script: data.script, name: data.name, glyph: match.glyph, sound });
    }
  }
  return out;
}
