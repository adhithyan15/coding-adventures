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
// It finds the bare form: an imperative writing verb in prose that is NOT
// inside a bracketed cue, in one of four places.
//
//   shape                                        example from the corpus
//   -------------------------------------------  ----------------------------------------
//   opening a sentence                           "4. Write all five from dictation."
//   opening a "…, then write" clause             "Say *desh*, then write **देश**."
//   after a fronted phrase                       "Without looking back, write **look** …"
//   chained onto another step verb               "Say, read, and write each answer."
//                                                "Look, cover, wait five seconds, and write it."
//                                                "4. **Writing:** hear all six and write them."
//
// The first two are one regex, `BARE_WRITING_IMPERATIVE`. The last two came
// later: a hand review of the #16893/#16994 fixes found the same writing
// tasks hiding behind a fronted "Without looking back," and behind an earlier
// imperative ("Say and write …") where the verb opens nothing. Those two are
// judged one clause at a time by `opensChainedOrFrontedWriting`, because a
// single regex that let any amount of text sit between a step verb and "and
// write" would need a lazy `[^.]*?` across the whole sentence — the shape that
// goes quadratic and that code scanning flags. The test beside this module
// (`drivable-writing-cues.test.ts`) runs all four over every drivable lesson
// in every track.
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
//   look, listen, speak, write.             a chain with no object is a list of skills
//   Say "I read and write Spanish".         a quotation is the material being said
//   wine is what you buy, ship, tax and     a chain only counts after a step verb
//     write down                              ("Say", "Cover", "Hear" …) opens the clause
//   go to the desk, and write your name     "go" is not a step verb: this is a notice
//                                             in a reading passage, not the lesson
//   when it opens, write your name          a subordinate clause is not a fronted phrase
//   *bare*, write.                          a fronted phrase is plain words only
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
 * The verbs that open a non-writing step a writing verb is chained onto.
 *
 * "Say, read, and write each." "Cover it, wait five seconds, and write it."
 * "Hear all six and write them." In each the sentence opens with an
 * imperative the narrator can say safely, and the writing verb rides in on a
 * coordinating ", and" / " and" / ",". The opening verb is what makes the
 * whole chain an instruction: "wine is what you buy, ship, tax and write down"
 * opens with a subject, and "only then will you read and write the whole
 * word" with an adverb, so neither is a chain.
 *
 * The list is the set of step verbs the corpus actually opens chains with.
 * It is closed on purpose. "Go to the desk, and write your name" is what a
 * notice in a Japanese reading passage says, not what the lesson asks, and
 * "go" is not here; nor is any verb that is more often a noun in a lesson
 * ("Form", "Practice", "Watch").
 */
const CHAIN_OPENER = String.raw`(?:say|read|hear|listen|cover|uncover|hide|look|wait|pause|recall|repeat|repair|greet|identify|name|retrieve|check|speak|point|turn|count|ask|answer|picture|imagine|perform|run)\b`;

/**
 * A short phrase fronted before the imperative: "Without looking back, write",
 * "From sound alone, write", "With the page covered, write", "Beside each,
 * write", "Then, with the new word covered, write".
 *
 * It must open with one of a closed set of words — the adverbials and
 * prepositions lessons use to set up a recall step — and run, in plain words
 * only, to a comma. Plain words means no `*`, `(` or quote can sit in it, so
 * "*bare*, write" (a gloss pair) and "In Hindi, how do you say it, show me, I
 * could not hear, write it down" (a list of phrases) are not fronted phrases.
 * A subordinate clause ("when it opens, write your name at reception", which
 * is a reading passage quoting a notice) is deliberately not in the set.
 *
 * At most two may stack ("Then, with the new word covered,"), and each is at
 * most eight words. Both bounds keep the match linear: the words are separated
 * by a required run of spaces, which no word character can also match, so the
 * engine never has two ways to split the same text.
 */
const FRONTED_PHRASE = String.raw`(?:then|now|next|finally|first|again|afterwards|(?:without|from|with|beside|below|underneath|under)(?:\s+[a-z'’-]+){0,7}),\s+`;

/** A clause may open with a list marker, a bold label opener, and a connective, exactly as `SENTENCE_START` allows. */
const CLAUSE_LEAD = String.raw`^(?:(?:[-*+]|\d+[.)])\s+)?(?:\*\*)?(?:(?:then|now|and|also)\s+)?`;

/** "Without looking back, write …" — a writing verb straight after one or two fronted phrases. */
const FRONTED_WRITING = new RegExp(
  `${CLAUSE_LEAD}(?:${FRONTED_PHRASE}){1,2}${WRITING_VERB}${NOT_A_MENTION}`,
  "i",
);

/** A clause that opens (after any lead and fronted phrases) with a chain-opening step verb. */
const CHAIN_START = new RegExp(`${CLAUSE_LEAD}(?:${FRONTED_PHRASE}){0,2}${CHAIN_OPENER}`, "i");

/**
 * The link that carries a writing verb into a chain: ", and write", " and
 * write", ", write", optionally with "then" ("and then write").
 *
 * The verb must be followed by a word, not by the end of the sentence. That
 * is the guard for a list of skills — "look, listen, speak, write." — where
 * every item is a bare verb and nothing is being written. "Say, read, and
 * write each." has its object; "That closes the run: look, listen, speak,
 * write." does not.
 *
 * Every gap is ONE literal space, not `\s+`, because the clause has had its
 * whitespace runs collapsed (see `opensChainedOrFrontedWriting`). That is a
 * linearity fix, not a style choice: an unanchored `\s+and\s+` restarts at
 * every space of a long run and rescans the rest of it, so "say", 40,000
 * spaces and "x" took over a second.
 *
 * `CHAIN_NOT_A_MENTION` is `NOT_A_MENTION` without its recall-question
 * lookahead. That lookahead scans to the end of the sentence from every
 * candidate, which is quadratic in an unanchored search; the question case is
 * handled once per clause instead (a clause that ends in "?" is skipped).
 */
const CHAIN_NOT_A_MENTION = String.raw`(?!\*{0,2} (?:is|was|means)\b)(?!,)(?!: )`;

const CHAIN_LINK = new RegExp(
  String.raw`(?:, (?:and )?| and )(?:then )?${WRITING_VERB}${CHAIN_NOT_A_MENTION}(?= [^\s.,;:!?])`,
  "i",
);

/**
 * Where a clause ends. The same stops `SENTENCE_START` recognises, with the
 * same optional closer between the stop and the space, so "**Writing:** hear
 * all six and write them" splits into "**Writing:**" and "hear all six and
 * write them", and the second is judged as a clause of its own.
 */
const CLAUSE_BREAK = /[.!?;:][)*"”’]{0,3}\s+/g;

/**
 * The clauses of one span, in order, each keeping its stop character ("." "?"
 * …) so a question can be told from an instruction. A single forward pass over
 * the span's stops: no clause is re-scanned, so the total work is linear in the
 * span.
 */
export function clausesOf(span: string): string[] {
  const clauses: string[] = [];
  let from = 0;
  for (const stop of span.matchAll(CLAUSE_BREAK)) {
    const end = (stop.index ?? 0) + 1;
    clauses.push(span.slice(from, end));
    from = (stop.index ?? 0) + stop[0].length;
  }
  clauses.push(span.slice(from));
  return clauses.filter((clause) => clause.trim() !== "");
}

/**
 * Does this clause tell the listener to write, in one of the two shapes the
 * single regex cannot see?
 *
 *   fronted       "Without looking back, write **look**, **see** …"
 *   chained       "Say, read, and write each answer."
 *                 "Look, cover, wait five seconds, and write the word."
 *                 "hear all six and write them" (after a "**Writing:**" label)
 *
 * The chained test runs only when the clause opens with a step verb from
 * `CHAIN_OPENER`; the link itself is then looked for anywhere in the clause
 * outside quotation marks (see `withoutQuotations`).
 *
 * A clause that ends in "?" is a question, not an instruction: "Say it and
 * write it down? (**…**.)" is a recall prompt with its answer in brackets, the
 * same exclusion `NOT_A_MENTION` makes for the single regex.
 *
 * Whitespace runs are collapsed to one space first (one linear pass), so every
 * later pattern can name a gap as a single literal space. Each regex is then
 * applied once per clause, and none has a quantifier over a group that can
 * match the same text two ways, so the cost stays linear.
 */
export function opensChainedOrFrontedWriting(clause: string): boolean {
  const text = clause.replace(/\s+/g, " ").trim();
  if (text.endsWith("?")) return false;
  if (FRONTED_WRITING.test(text)) return true;
  if (!CHAIN_START.test(text)) return false;
  return CHAIN_LINK.test(withoutQuotations(text));
}

/**
 * The clause with its quoted stretches blanked out.
 *
 * `Say "I read and write Spanish".` opens with a step verb and contains " and
 * write", but that writing is inside the sentence being taught: the learner
 * is asked to SAY it. Whatever sits in double quotation marks is material, not
 * a further step, so it is removed before the chain is looked for. A quote
 * that is only a cue word ("Then hear “younger sister,” say **いもうと**, …
 * and write all three") loses its two words and keeps the chain around it.
 *
 * Each pattern is one opening quote, a run that contains NEITHER quote mark,
 * and the closing quote. Excluding the opener from the run is what keeps the
 * scan linear: with `“[^”]*”`, a run of unclosed `“` let every opener scan to
 * the end of the clause before failing (40,000 of them took over a second).
 * Here a failed attempt stops at the next `“`, which is where the next attempt
 * starts, so each character is read a bounded number of times. For the
 * straight quote the opener and closer are the same character, so `"[^"]*"`
 * already has that property. An unclosed quote is left as it is.
 */
function withoutQuotations(clause: string): string {
  return clause.replace(/"[^"]*"/g, "\"\"").replace(/“[^“”]*”/g, "“”");
}

/**
 * Every span of `markdown` that a narrator would read as a bare instruction to
 * write. Returns the spans themselves, so a failure message can quote them.
 */
export function bareWritingImperatives(markdown: string): string[] {
  return narratedProseSpans(markdown).filter((span) => {
    BARE_WRITING_IMPERATIVE.lastIndex = 0;
    if (BARE_WRITING_IMPERATIVE.test(span)) return true;
    return clausesOf(span).some(opensChainedOrFrontedWriting);
  });
}
