// ---------------------------------------------------------------------------
// core.ts — the pure, testable heart of the app.
//
// Everything here is a plain function of its inputs: no DOM, no globals, no
// randomness. Given a `ScriptData` (loaded from the curriculum's JSON), it
// produces the small view-models the UI renders. Keeping this layer pure is
// what lets us test the *pedagogy* — "is this letter flagged as a false friend?
// how many pieces does it break into?" — without spinning up a browser.
// ---------------------------------------------------------------------------

import type { Letter, ScriptData } from "./types.ts";

/**
 * A "false friend" is a letter that LOOKS like a Latin letter but sounds
 * nothing like it — the single biggest trap when an English reader meets
 * Cyrillic (в=v, р=r, с=s, н=n) or Greek-derived shapes. The curriculum flags
 * these in a letter's `notes` with the phrase "FALSE FRIEND". We detect that
 * marker case-insensitively so the UI can badge them.
 *
 *   isFalseFriend({ notes: "FALSE FRIEND: looks like B, says v" }) === true
 *   isFalseFriend({ notes: "From Greek delta." })                 === false
 *   isFalseFriend({})                                             === false
 */
export function isFalseFriend(letter: Pick<Letter, "notes">): boolean {
  return /false friend/i.test(letter.notes ?? "");
}

/**
 * The three Dravidian "special" consonants a learner must tell apart from their
 * plain look-/sound-alikes: the RETROFLEX ḷ and the ALVEOLAR ṟ / ṉ, set against
 * the ordinary l / r / n. Telugu, Kannada, and Malayalam all carry them, and to
 * an outsider ల vs ళ (la vs ḷa) is exactly the kind of near-miss that stalls
 * reading — so the app flags them the way it flags Latin false friends.
 *
 * We key on the syllable's ISO-15919 romanization, which is script-agnostic (ḷ
 * looks different in each script but romanizes the same): the LEADING code point
 * of the sound is the retroflex/alveolar marker —
 *   ḷ = U+1E37 (LATIN SMALL LETTER L WITH DOT BELOW),
 *   ṟ = U+1E5F (… R WITH LINE BELOW),
 *   ṉ = U+1E49 (… N WITH LINE BELOW).
 * These markers appear ONLY on these consonants in our data (the vocalic-R vowel
 * uses a ring-below r̥, U+0325 — a different code point — so there is no clash),
 * so testing the first character is exact, not heuristic. Returns the contrast
 * hint for a special consonant, or null for an ordinary letter.
 *
 *   specialConsonant({ sound: "ḷa" }).plain === "l"
 *   specialConsonant({ sound: "la" })       === null
 */
export interface SpecialConsonant {
  /** The ordinary letter it is most confused with. */
  plain: string;
  /** A grounded, cross-Dravidian-safe note on how it differs. */
  hint: string;
}
const SPECIAL_CONSONANTS: Record<string, SpecialConsonant> = {
  "ḷ": {
    plain: "l",
    hint: "Retroflex ḷ — the tongue tip curls back to the roof of the mouth. Set apart from the ordinary l (ల-type).",
  },
  "ṟ": {
    plain: "r",
    hint: "Alveolar ṟ — a distinct, harder r made at the ridge behind the teeth. Set apart from the ordinary tapped r.",
  },
  "ṉ": {
    plain: "n",
    hint: "Alveolar ṉ — an n made at that same ridge, further back than the ordinary (dental) n it contrasts with.",
  },
};

export function specialConsonant(letter: Pick<Letter, "sound">): SpecialConsonant | null {
  const first = [...(letter.sound ?? "")][0];
  return (first !== undefined && SPECIAL_CONSONANTS[first]) || null;
}

/** The view-model for a single letter card + its "how to write it" detail. */
export interface LetterView {
  glyph: string;
  sound: string;
  role: string;
  tone?: string;
  inherentVowel?: string;
  /** The pieces to draw, in order. */
  components: string[];
  strokeOrder: string[];
  strokeOrderNote: string;
  notes: string;
  falseFriend: boolean;
  /** Set when this is a retroflex/alveolar special consonant (ḷ/ṟ/ṉ). */
  special: SpecialConsonant | null;
  /** How many distinct strokes/pieces — a rough "how hard to draw" signal. */
  strokeCount: number;
}

/** Turn one raw `Letter` into its view-model. */
export function toLetterView(letter: Letter): LetterView {
  return {
    glyph: letter.glyph,
    sound: letter.sound,
    role: letter.role,
    tone: letter.tone,
    inherentVowel: letter.inherentVowel,
    components: letter.components ?? [],
    strokeOrder: letter.strokeOrder ?? [],
    strokeOrderNote: letter.strokeOrderNote ?? "",
    notes: letter.notes ?? "",
    falseFriend: isFalseFriend(letter),
    special: specialConsonant(letter),
    strokeCount: (letter.strokeOrder ?? []).length,
  };
}

/** Turn a whole script's letters into view-models, in inventory order. */
export function buildScriptView(data: ScriptData): LetterView[] {
  return data.letters.map(toLetterView);
}

/** A one-line-per-field summary of a script, for the header + the picker. */
export interface ScriptSummary {
  script: string;
  name: string;
  system: string;
  direction: "ltr" | "rtl";
  letterCount: number;
  falseFriendCount: number;
  complete: boolean;
}

export function scriptSummary(data: ScriptData): ScriptSummary {
  const views = buildScriptView(data);
  return {
    script: data.script,
    name: data.name,
    system: data.system,
    direction: data.direction,
    letterCount: views.length,
    falseFriendCount: views.filter((v) => v.falseFriend).length,
    complete: data.complete ?? false,
  };
}

/**
 * Only the false-friend letters, for the "letters that lie" study list — the
 * fastest way into a Cyrillic-like script. Returns them in inventory order.
 */
export function falseFriends(data: ScriptData): LetterView[] {
  return buildScriptView(data).filter((v) => v.falseFriend);
}
