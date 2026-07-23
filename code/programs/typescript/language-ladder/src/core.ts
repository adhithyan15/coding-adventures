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
