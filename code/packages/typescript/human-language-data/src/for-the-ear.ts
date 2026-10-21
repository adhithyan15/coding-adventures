// Saying a typed prompt, and a heading, for the ear.
//
// The narration is a script for someone who cannot look at a screen. Most of the
// lesson is already written for that listener, because it is prose and cues. Two
// things are not, because they were written for a reader with the app or the book
// open in front of them:
//
//   - a scored activity's PROMPT, which the app shows above a text box:
//       "Type the Japanese for 'a stick, a pole'."
//       "Type the Japanese for 'a dog', in rōmaji."
//       "Say to speak aloud, then type it romanized and the stem it leaves behind."
//   - a section's HEADING, which the book prints above the section:
//       "Guided Practice — read, copy, recall"
//       "(el mensaje del vecino) — read it, then answer"
//
// Read aloud as written, both ask a driver to do something with a hand or an eye.
// This module rewrites them for the ear. It is deliberately small: a closed list of
// substitutions, each one the spoken equivalent of what the reader was asked, and
// nothing that guesses.
//
// What it does NOT do is decide that a prompt is impossible by ear. "Write son in
// Chinese characters" cannot be said for the ear — writing 儿子 is the task — and
// rewording it as "Say the word for son" would grade a different skill under the
// same activity id. That judgement is `activityNeedsEyesOrHands` in
// `drivable-instructions.ts`, and the narration DEFERS such a prompt instead of
// rewording it. The two are applied in that order: a prompt is first asked whether
// it needs eyes or hands, and only a prompt that does not is said for the ear here.
//
// ---------------------------------------------------------------------------
// Why the narration and not the lessons
// ---------------------------------------------------------------------------
//
// Rewording the 1,268 "Type …" prompts in the lessons was the alternative, and it is
// wrong for the app: the prompt sits above a text box, and "Say the Japanese for 'a
// stick'" above a box that wants keystrokes is a worse instruction there than "Type"
// is in the car. Each output gets the verb its medium can do — the same split the
// cue grammar already makes, where `[YOU WRITE: …]` is printed "*Write it:* …" in
// the book and spoken "once you have stopped driving — write: …" in the narration.
// The authored prompt stays the single source; the renderer chooses the words.

import { headingStepIndex } from "./drivable-instructions.js";

// ---------------------------------------------------------------------------
// A typed prompt, said
// ---------------------------------------------------------------------------
//
// Two rules, applied in order:
//
//   1. The typing verb becomes the speaking verb, wherever a step starts, in the
//      case it was written in:
//
//        Type the Persian word for 'tea'.        → Say the Persian word for 'tea'.
//        You hear nahī̃. Type yes or no.          → You hear nahī̃. Say yes or no.
//        … aloud, then type the romanized turns. → … aloud, then say the turns.
//
//      "type" as a noun is left alone: "Give … its borrowing type", "a type of".
//
//   2. A phrase that only tells the TYPIST how to spell the answer is dropped,
//      because a spoken answer has no spelling:
//
//        phrase                          example                                  becomes
//        ------------------------------  ---------------------------------------  ----------------------------------
//        ", in rōmaji"                   Say the Japanese for 'a dog', in rōmaji.  Say the Japanese for 'a dog'.
//        ", in pinyin with its tone      Say the Mandarin word, in pinyin with     Say the Mandarin word that means
//          mark,"                          its tone mark, that means 'you'.          'you'.
//        ", with its tone mark,"         Say the pinyin, with its tone mark, for   Say the pinyin for the number two.
//                                          the number two.
//        "In pinyin, " / ", in pinyin"   In pinyin, what one word gets it          What one word gets it repeated?
//          / " in pinyin "                 repeated?
//        "romanized" (as an adjective    then say the romanized words you said     then say the words you said
//          or after "it")                then say it romanized and name …          then say it and name …
//        "its romanization"              then say its romanization.                then say it.
//
//      Rule 2 runs on every spoken prompt, not only typed ones: "Ask for tea
//      politely, in rōmaji." has no typing verb and still asks for a spelling.
//      What it never touches is a question ABOUT the writing ("In the pinyin for
//      'morning', why does the second syllable have no tone mark?"): "In the
//      pinyin" is not "In pinyin,".
//
// Every pattern is a fixed literal with optional fixed literals around it, applied
// once as a whole-string (/g) replace, so the rewrite is linear in the prompt.

/**
 * "type" where a step starts — the step links of `TYPING_STEP` in
 * `drivable-instructions.ts`, which the gate uses to prove none is left — with the
 * link and any bold opener or connective captured, so they are kept, and the verb
 * captured, so its case can be.
 */
const TYPE_VERB =
  /(^|[.!?;:]["”’)*]{0,3} |, (?:and |or )?|; | and | then )((?:\*\*)?(?:then |now |and |also |first )?)(type)(?= |\*\* )(?! of\b)/gi;

/** The speaking verb in the typing verb's case: "Type" → "Say", "type" → "say", "TYPE" → "SAY". */
function sayInCaseOf(type: string): string {
  if (type === "TYPE") return "SAY";
  return type.startsWith("T") ? "Say" : "say";
}

/** Typist-only spelling instructions, each with what replaces it. See the table in rule 2. */
const SPELLING_ONLY: ReadonlyArray<readonly [RegExp, string]> = [
  [/,? in r[oō]maji\b/gi, ""],
  [/, in pinyin with (?:its|their) tone marks?,/gi, ""],
  [/, with (?:its|their) tone marks?,/gi, ""],
  [/(^|[.!?] )In pinyin, /g, "$1"],
  [/, in pinyin(?=[.?!])/gi, ""],
  [/ in pinyin (?=the |as )/gi, " "],
  [/\bits romanization\b/gi, "it"],
  [/\bromanized /gi, ""],
  [/ romanized\b/gi, ""],
];

/** Capitalise the first letter of the text and of every sentence after a dropped opener. */
function capitaliseSentences(text: string): string {
  return text.replace(/(^|[.!?] )(\p{Ll})/gu, (_match, lead: string, letter: string) => `${lead}${letter.toUpperCase()}`);
}

/**
 * A scored prompt as a driver should hear it. See the two rules above.
 *
 *   "Type the Japanese for 'a stick, a pole'."   → "Say the Japanese for 'a stick, a pole'."
 *   "Type the Japanese for 'a dog', in rōmaji."  → "Say the Japanese for 'a dog'."
 *   "Which one points at a day?"                 → unchanged
 *
 * Call it only on a prompt `activityNeedsEyesOrHands` cleared: a prompt that
 * types a SCRIPT ("Type 'a car' in hiragana") is a writing task, and saying it
 * "Say 'a car' in hiragana" would be nonsense the narration defers instead.
 */
export function activityPromptForTheEar(prompt: string): string {
  let text = prompt.replace(
    TYPE_VERB,
    (_match, link: string, connective: string, type: string) => `${link}${connective}${sayInCaseOf(type)}`,
  );
  const before = text;
  for (const [pattern, replacement] of SPELLING_ONLY) text = text.replace(pattern, replacement);
  // Only re-capitalise when a spelling phrase actually went, so an authored
  // lower-case opening ("kokonotsu begins with …") is never touched.
  return text === before ? text : capitaliseSentences(text);
}

// ---------------------------------------------------------------------------
// A heading, said
// ---------------------------------------------------------------------------
//
// `headingStepIndex` finds the first eyes-or-hands step in a heading. The heading
// is then cut back to its LABEL — what precedes the label separator (": ", " — ",
// " – ") that comes before the step:
//
//   heading                                                  said
//   -------------------------------------------------------  -------------------------
//   Guided Practice — read, copy, recall                     Guided Practice
//   Practice — sort the evening words, then read the Latin   Practice
//   Sounds you'll need: Read the cedilla                     Sounds you'll need
//   (el mensaje del vecino) — read it, then answer           (el mensaje del vecino)
//   Read two, say one                                        (nothing: no label)
//   Understand and read return at R4                         (nothing: no label)
//   Grammar Lens — the sign は, read *wa*                     Grammar Lens (heard as "read wa")
//   Guided Practice: point every verb at somebody            unchanged (no step)
//
// A heading that is ALL step has no label to keep, and the caller decides what to
// say instead (the narration falls back to the headword, then to nothing).
//
// A LESSON title has one shape a section title does not: the headword and its gloss,
// "ಬರೆ (bare) — write". The tail is the meaning of the word being taught, and the
// learner must hear it. So when the title's last " — " separates a head that holds
// the headword from a tail that IS the lesson's gloss, the tail is kept whatever it
// says. Both conditions, not either: "(el mensaje del vecino) — read it, then
// answer" holds its headword, but its gloss is "the payoff — read a message …", so
// the tail is an instruction and goes.

/** The separators between a heading's label and what its section does. */
const LABEL_SEPARATORS: readonly string[] = [": ", " — ", " – "];

/** Lower-case, markdown-free, whitespace-collapsed: for comparing a title's tail with a gloss. */
function comparable(text: string): string {
  return text.replace(/[*_`]/g, "").replace(/\s+/g, " ").trim().toLowerCase();
}

/** What a lesson title is a title FOR: its headword and gloss, as authored. */
export interface TitleContext {
  headword?: string;
  gloss?: string;
}

/**
 * A heading with any eyes-or-hands step cut away, back to its label; "" when the
 * whole heading is a step. Works on the authored Markdown, so a bold label opener
 * ("**Read.**") is seen where it was written; the caller makes the result speakable.
 */
export function headingForTheEar(heading: string, lessonTitle: TitleContext = {}): string {
  const text = heading.replace(/\s+/g, " ").trim();
  // For a "headword — gloss" title only the head is judged, and an untouched
  // heading comes back whole, gloss and all.
  const judged = withoutGlossTail(text, lessonTitle);
  const step = headingStepIndex(judged);
  if (step === -1) return text;
  let cut = -1;
  for (const separator of LABEL_SEPARATORS) {
    // The separator that STARTS the step counts too ("Guided Practice — write"):
    // search up to and including the step's own index.
    const at = judged.lastIndexOf(separator, step);
    if (at > cut) cut = at;
  }
  if (cut <= 0) return "";
  return withoutTrailingJoiners(judged.slice(0, cut));
}

/** What a label may trail once its step is cut away: spaces and the joining punctuation. */
const TRAILING_JOINERS: ReadonlySet<string> = new Set([",", ";", ":", "—", "–", "-"]);

/**
 * A label without the spaces and joining punctuation it trails: "Practice, " →
 * "Practice". One walk back from the end, so it is linear.
 *
 * Not `replace(/[\s,;:—–-]+$/u, "")`: an end-anchored run is retried from every
 * start position, so a label holding a long run of those characters that does
 * NOT reach the end ("Practice" + 100,000 commas + "x") cost a quadratic scan —
 * and a heading is just text in a pull request. The walk looks at each trailing
 * character once and stops at the first that stays.
 */
function withoutTrailingJoiners(label: string): string {
  let end = label.length;
  while (end > 0) {
    const last = label[end - 1] ?? "";
    if (!TRAILING_JOINERS.has(last) && !/\s/u.test(last)) break;
    end -= 1;
  }
  return label.slice(0, end);
}

/**
 * Does a heading, as it would be spoken, still ask for an eye or a hand? The test
 * {@link headingForTheEar} makes true-or-false, for a gate over what the narration
 * actually says: a lesson title's gloss tail is set aside exactly as it is there.
 *
 *   "Guided Practice — write"                          true
 *   "Guided Practice"                                  false
 *   "ಬರೆ (bare) — write", headword ಬರೆ, gloss "write"   false
 */
export function headingAsksForEyesOrHands(heading: string, lessonTitle: TitleContext = {}): boolean {
  return headingStepIndex(withoutGlossTail(heading.replace(/\s+/g, " ").trim(), lessonTitle)) !== -1;
}

/**
 * The part of a heading the step check reads: a lesson title of the shape
 * "headword — gloss" without its gloss (see the header), anything else whole.
 *
 *   "ಬರೆ (bare) — write", headword ಬರೆ, gloss "write"            → "ಬರೆ (bare)"
 *   "(el mensaje del vecino) — read it, then answer", gloss
 *     "the payoff — read a message …"                            → the whole title
 */
function withoutGlossTail(text: string, { headword = "", gloss = "" }: TitleContext): string {
  const at = text.lastIndexOf(" — ");
  if (at === -1 || headword.trim() === "" || gloss.trim() === "") return text;
  const head = text.slice(0, at);
  const isGloss = head.includes(headword.trim()) && comparable(text.slice(at + 3)) === comparable(gloss);
  return isGloss ? head : text;
}
