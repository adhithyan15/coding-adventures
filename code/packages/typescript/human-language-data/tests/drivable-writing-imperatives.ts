// Finding the sentence that tells a driver to pick up a pen.
//
// ---------------------------------------------------------------------------
// The hazard
// ---------------------------------------------------------------------------
//
// `core/lesson-modality` marks a lesson `drivable: true` when every block of it
// can be listened to. The narration export then reads it out with no warning,
// because there is nothing to warn about. Its rules look at structure -- a
// `writing` or `script` block, a wide table, a sight cue such as "look at" --
// and none of them reads an ordinary sentence for what it asks the learner to
// DO. So a warm-up that says
//
//     [PAUSE 2s] Draw the shape you just learned. One stroke, left to right.
//
// leaves the lesson drivable, and the audio edition tells someone at speed to
// draw (issue #12070, nine Chinese lessons in chapters 3-6).
//
// The corpus already has the right spelling for a writing task inside a
// listenable lesson: the activity cue.
//
//     [PAUSE 2s] [YOU WRITE: the shape you just learned — one stroke, left to right]
//
// `narration.ts` lists WRITE among its `MANUAL_CUE_ACTIONS`, so the cue is
// spoken as "[once you have stopped driving — write: …]", and the cue does not
// create a `writing` block, so it does not cost the lesson its drivability.
// The book prints it as "*Write it:* …". One authored form, two correct
// renderings. Bare prose gets neither: the renderer cannot tell "Write 是." from
// any other sentence, so it reads it out unhedged.
//
// ---------------------------------------------------------------------------
// What this module does
// ---------------------------------------------------------------------------
//
// It finds the bare form: an imperative writing verb that opens a sentence (or
// a "…, then write …" clause) in prose that is NOT inside a bracketed cue. The
// test beside it (`drivable-writing-cues.test.ts`) runs it over every
// drivable lesson in every track.
//
// "Not inside a cue" is decided by the narration renderer's own cue splitter,
// `splitNarrationCues`, not by a second bracket regex. The question being asked
// is "will the narration hedge this sentence?", and the only honest answer
// comes from the code that does the hedging. A sentence the splitter hands
// back as prose is a sentence the renderer will say plainly.
//
// ---------------------------------------------------------------------------
// Why the pattern is narrow, and where it is deliberately blind
// ---------------------------------------------------------------------------
//
// "write" is everywhere in a language course that is not an instruction: the
// word is *written* with œ, the etymology of English **write**, a gloss such as
// *Schreib!* — *Write!*. A detector that fired on the word would fire on
// hundreds of correct lessons and be switched off within a week. So it only
// fires where English puts an imperative, and every exclusion below was added
// for a real sentence in the corpus that is a mention, not an instruction:
//
//   shape                                   why it is not an instruction
//   --------------------------------------  ----------------------------------
//   **Write** is Old English *wrītan*       the verb is the SUBJECT ("is/was/means" follows)
//   *Schreib!* — *Write!*                   an italic gloss; italics are not allowed before the verb
//   Think: **विचार करणे**. Write: **लिहिणे**.  "Write:" + space is a vocabulary gloss line
//   *Apunta el número* — write the number   a dash only opens a clause with "then"
//   …; copy the sound rather than the …     copying a SOUND is speaking
//   *fijar* — draw, harden, fasten          a verb followed by a comma is a list of glosses
//   Write it down? (**ಬರೆದು ಕೊಡಿ**.)          a question answered in brackets is a recall prompt
//   Trace *nox* back to PIE                 "trace" is the etymologist's verb here; not matched
//
// Deliberately out of scope: `trace`, `mark`, `label`, `print` and `fill in`.
// In this corpus every sentence-initial "Trace" is etymology ("Trace it back
// far enough…"), and a finger-tracing task is authored as `[YOU TRACE: …]`
// already; the other four are overwhelmingly nouns or glosses. Adding them
// would buy a handful of real hits at the price of a pattern nobody could
// keep clean. That trade is the same one `SIGHT_CUES` in `src/modality.ts`
// makes: a false negative is a sentence a human reviewer still has to catch;
// a gate that cries wolf is a gate nobody reads.
//
// Also out of scope: lessons that are NOT drivable. A `pen` or `sight` lesson
// opens its narration with a spoken notice that it needs hands and eyes, so a
// bare "Write it" inside one has already been hedged at the lesson level.

import { splitNarrationCues } from "../src/narration.js";

/**
 * The writing verbs, as English imperatives.
 *
 * `copy` carries its own guard: "copy the sound", "copy the rhythm" and their
 * siblings ask the learner to imitate with the mouth, which a driver can do.
 */
const WRITING_VERB = String.raw`(?:write|rewrite|draw|copy(?!\s+(?:the\s+)?(?:sound|rhythm|melody|intonation|pronunciation|pitch|tone|stress)\b)|circle|underline|sketch|jot)\b`;

/**
 * Where an English imperative can begin.
 *
 *   ^                     the start of a paragraph, list item, or the prose after a cue
 *   . ! ? ; :  then space the start of a sentence or clause; a closing `)`, `*`
 *                         or quote may sit between the stop and the space, so
 *                         "**Writing.** Write all five" and "(**ai**.) Write" count
 *
 * After that: an optional list marker ("4. ", "- "), an optional bold label
 * opener ("**Write.**", "**Write:**"), and an optional connective ("Then
 * write", "Now write", "And write").
 */
const SENTENCE_START = String.raw`(?:^|(?<=[.!?;:][)*"”’]{0,3})\s+)(?:(?:[-*+]|\d+[.)])\s+)?(?:\*\*)?(?:(?:then|now|and|also)\s+)?`;

/**
 * A clause inside a sentence: "Say *desh*, then write **देश**", "Say one, then
 * write it". Only `then` opens one. A bare ", and write" or "— write" is too
 * often a description ("reference books attach it …, and draw in a wide
 * family") or a gloss ("*Apunta el número* — write the number down").
 */
const THEN_CLAUSE = String.raw`[,—–]\s+(?:and\s+)?then\s+`;

/**
 * Mentions that look like imperatives. See the table in the header.
 *
 *   (?!\*{0,2}\s+(?:is|was|means)\b)  "**Write** is Old English …"
 *   (?!,)                             "draw, harden, fasten" — a gloss list
 *   (?!:\s)                           "Write: **लिहिणे**" — a vocabulary line
 *                                     ("**Write:** hear …" is a bold label,
 *                                     ":" then "*", and still fires)
 *   (?![^.!?]*\?\s*\()                "Write it down? (**ಬರೆದು ಕೊಡಿ**.)"
 */
const NOT_A_MENTION = String.raw`(?!\*{0,2}\s+(?:is|was|means)\b)(?!,)(?!:\s)(?![^.!?]*\?\s*\()`;

/** The whole pattern. Case-insensitive, because "Write" and "then write" are the same verb. */
export const BARE_WRITING_IMPERATIVE = new RegExp(
  `(?:${SENTENCE_START}|${THEN_CLAUSE})${WRITING_VERB}${NOT_A_MENTION}`,
  "gi",
);

/**
 * Drop every HTML comment, the way the narrator does.
 *
 * A monotonic `indexOf` scan, not `replace(/<!--[\s\S]*?-->/g, "")`. That
 * regex is quadratic in the number of `<!--` openers when one is never closed
 * (see the same note in src/info-dump.ts and src/metalanguage.ts), and a single
 * pass of it is the "incomplete multi-character sanitization" shape code
 * scanning flags. Here each character is visited once, and an unterminated
 * `<!--` swallows the rest of the text — exactly what speech.ts does with its
 * `<!--[\s\S]*?(?:-->|$)` — so nothing the narrator would skip is checked.
 *
 *   "a <!-- x --> b"   → "a  b"
 *   "a <!-- x"         → "a "
 *   "a <!--<!-- x -->" → "a "
 */
export function withoutHtmlComments(markdown: string): string {
  let out = "";
  let index = 0;
  for (;;) {
    const start = markdown.indexOf("<!--", index);
    if (start === -1) break;
    out += markdown.slice(index, start);
    const end = markdown.indexOf("-->", start + 4);
    if (end === -1) return out;
    index = end + 3;
  }
  return out + markdown.slice(index);
}

/**
 * Break one block's Markdown into the spans a narrator would say as one run.
 *
 * Paragraphs are joined across their wrapped lines, because lessons wrap at
 * about eighty columns and "Draw the shape. Two bars … say\nwhat number" is one
 * sentence. Each list item, table row and heading starts a new span. HTML
 * comments and fenced code are dropped: neither is read aloud.
 *
 * Every span is then cut at its cues, and only the prose between them is kept.
 * A cue ends whatever came before it in the narration (it becomes its own
 * segment), so the prose after a cue is treated as starting fresh — which is
 * what lets "[PAUSE 2s] Draw the shape" be seen as opening with "Draw".
 */
export function narratedProseSpans(markdown: string): string[] {
  const text = withoutHtmlComments(markdown).replace(/^```[\s\S]*?^```/gm, "");
  const units: string[] = [];
  let current: string[] = [];
  const flush = (): void => {
    if (current.length > 0) units.push(current.join(" "));
    current = [];
  };
  for (const raw of text.split("\n")) {
    const line = raw.replace(/^\s*>\s?/, "");
    if (line.trim() === "") {
      flush();
      continue;
    }
    if (/^\s*(?:[-*+]|\d+[.)])\s+/.test(line) || /^\s*\|/.test(line) || /^#/.test(line)) flush();
    current.push(line.trim());
  }
  flush();

  const spans: string[] = [];
  for (const unit of units) {
    for (const part of splitNarrationCues(unit)) {
      if (!("text" in part)) continue;
      const prose = part.text.trim();
      if (prose !== "") spans.push(prose);
    }
  }
  return spans;
}

/**
 * Every span of `markdown` that a narrator would read as a bare instruction to
 * write. Returns the spans themselves, so a failure message can quote them.
 */
export function bareWritingImperatives(markdown: string): string[] {
  return narratedProseSpans(markdown).filter((span) => {
    BARE_WRITING_IMPERATIVE.lastIndex = 0;
    return BARE_WRITING_IMPERATIVE.test(span);
  });
}
