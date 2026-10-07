// filmstrip-caption.ts — the words under a stroke-order filmstrip, when the
// strip writes a LIST of letters (HL-C443).
//
// ---------------------------------------------------------------------------
// The problem this closes
// ---------------------------------------------------------------------------
//
// A writing lesson's headword is sometimes one letter (ক), sometimes a word
// (はい), and sometimes a LIST of letters taught together: "ক — ণ — শ",
// "ن، ت، ث", "வ, க". Every caption used to drop the headword into one English
// sentence as it stood, and for a list that goes wrong twice:
//
//   1. The sentence reads as if the list were ONE thing. The app printed
//      "How ক — ণ — শ is written, stroke by stroke".
//
//   2. The list's own separators come along. An Arabic list is separated by
//      the ARABIC comma ، (U+060C), which belongs to the Arabic script
//      (Script_Extensions=Arab, among others). The book's LaTeX renderer
//      wraps every run of a script's characters in that script's font macro,
//      so the comma joined the letter before it:
//
//          headword            caption in the book .tex
//          ------------------  ------------------------------------
//          ن، ت، ث             \ar{ن،} \ar{ت،} \ar{ث}
//
//      The renderer is right in general (an Arabic comma INSIDE Arabic prose
//      is Arabic punctuation and wants the Arabic font). It is the caption
//      that is wrong to hand it Arabic punctuation: the sentence is English,
//      and a list in an English sentence is separated the English way.
//
// So a list caption names the letters as "these letters", then lists them
// AFTER the sentence, separated by an ASCII comma and a space, whatever the
// lesson used to separate them:
//
//          headword            caption
//          ------------------  ---------------------------------------------
//          ক — ণ — শ           How these letters are written, stroke by
//                              stroke: ক, ণ, শ
//          ن، ت، ث             ... : ن, ت, ث     ->  \ar{ن}, \ar{ت}, \ar{ث}
//
// Each letter now stands alone in its own macro, and the comma between two of
// them is ordinary English punctuation in the body font.
//
// This module has no imports on purpose: the app (`language-ladder`) imports
// it into the browser to caption the same strip the same way, and the book's
// `figure-targets.ts` (which needs `node:path`) imports it too, so "what is a
// list" is answered in exactly one place.

/**
 * What separates the items of a list headword: whitespace, the ASCII comma,
 * the Arabic comma ، (U+060C), the ideographic comma 、 (U+3001), the em
 * dash — (U+2014) and the middle dot · (U+00B7). These are the separators the
 * authored lessons actually use; a run of them (", " or " — ") is one break.
 */
export const LIST_SEPARATORS = /[\s,،、—·]+/u;

const GRAPHEMES = new Intl.Segmenter("und", { granularity: "grapheme" });

/** A grapheme made of combining signs alone: a vowel sign taught by itself. */
const SIGNS_ONLY = /^\p{M}+$/u;

/**
 * The items of a headword that is a LIST of letters, or `undefined` when it is
 * not one.
 *
 * A list is two or more items between `LIST_SEPARATORS`, each item exactly ONE
 * grapheme (a letter, a letter with its signs, a sign by itself, a mark like
 * ¿). That is the same test `writingSequenceOf` uses to decide that a
 * headword is drawn as a list:
 *
 *     headword        items                     a list?
 *     --------------  ------------------------  ----------------------------
 *     "ক — ণ — শ"     ["ক", "ণ", "শ"]            yes
 *     "ن، ت، ث"       ["ن", "ت", "ث"]            yes
 *     "¿ ¡"           ["¿", "¡"]                 yes
 *     "ক"             ["ক"]                      no: one item is a letter
 *     "はい"          ["はい"]                    no: one item is a word
 *     "buenos días"   ["buenos", "días"]         no: the items are words
 */
export function listItemsOf(headword: string): string[] | undefined {
  const items = headword.trim().split(LIST_SEPARATORS).filter((item) => item !== "");
  if (items.length < 2) return undefined;
  const oneGraphemeEach = items.every((item) => [...GRAPHEMES.segment(item)].length === 1);
  return oneGraphemeEach ? items : undefined;
}

/**
 * What to call a list's items in an English sentence: "letters", or "signs"
 * when every item is a combining sign taught by itself (ુ, ી), or "letters and
 * signs" when the two are mixed (ૂ, ટ, ઈ, ઢ). A sign is not a letter, and the
 * book already says so: a strip that holds one is captioned "part by part".
 */
export function listNoun(items: readonly string[]): string {
  const signs = items.filter((item) => SIGNS_ONLY.test(item)).length;
  if (signs === 0) return "letters";
  return signs === items.length ? "signs" : "letters and signs";
}

/**
 * The opening of a list caption and its letters, as two halves the caller
 * puts either side of its own manner phrase (the book says "one after
 * another" or "part by part"; the app, which cannot see the strip's pieces,
 * says neither):
 *
 *     listCaptionParts(["ن", "ت", "ث"])
 *       -> { subject: "How these letters are written", items: "ن, ت, ث" }
 *
 *     `${subject}, stroke by stroke: ${items}`
 *       -> "How these letters are written, stroke by stroke: ن, ت, ث"
 */
export function listCaptionParts(items: readonly string[]): { subject: string; items: string } {
  return { subject: `How these ${listNoun(items)} are written`, items: items.join(", ") };
}

/**
 * The caption the APP prints under a writing lesson's filmstrip, from the
 * lesson's headword alone (the app does not load figure targets):
 *
 *     "ক"           -> "How ক is written, stroke by stroke"
 *     "はい"        -> "How はい is written, stroke by stroke"
 *     "ক — ণ — শ"   -> "How these letters are written, stroke by stroke: ক, ণ, শ"
 *
 * A single letter and a word read exactly as they always have; only a list
 * changes.
 */
export function filmstripCaption(headword: string): string {
  const items = listItemsOf(headword);
  if (items === undefined) return `How ${headword} is written, stroke by stroke`;
  const parts = listCaptionParts(items);
  return `${parts.subject}, stroke by stroke: ${parts.items}`;
}
