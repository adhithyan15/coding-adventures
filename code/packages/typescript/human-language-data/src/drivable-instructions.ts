// Finding the sentence that tells a driver to pick up a pen.
//
// (Or to read the page, point at it, cover it, handle a card or make a
// gesture.) This module began as a test helper,
// `tests/drivable-writing-imperatives.ts`, and the sections below still tell
// its history in order: each check was added when a pass over the corpus found
// a new shape. It lives in `src/` now because the checks became a modality
// rule — see "Promoted into the classifier" at the end, which is where to start
// if you want to know what decides whether a lesson is announced as drivable.
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
// One kind of cue is not safe either. `[YOU RECALL: write **ば** — **R1**]` is
// a cue, but RECALL is a spoken action, so the narration reads it to a driver
// as "recall: write ば". The last section of this module, "Inside a recall
// cue", reads those, and `drivableWritingInstructions` runs both. The sections
// after it read cues for the other things a hand or an eye does on the page:
// pointing at a sign (inside a recall), reading printed script and making a
// gesture (inside any cue the narration speaks unhedged). The last section
// returns to bare prose, for reading printed script, handling cards and
// covering the page.
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

import { parseDeliveryCue } from "./delivery-cue.js";
import { isManualCueAction, splitNarrationCues } from "./narration-cues.js";

/**
 * The writing verbs, as English imperatives.
 *
 * `copy` carries its own guard: "copy the sound", "copy the rhythm" and their
 * siblings ask the learner to imitate with the mouth, which a driver can do.
 * The guard stops at a written MARK: "copy the tone marks", "copy the stress
 * mark onto the right vowel", "copy the pitch accent marks" ask for a pen, and a
 * security review found the guard excusing them because "tone" and "stress"
 * matched as sound words. So a sound word followed by "mark(s)" (or "accent
 * mark(s)") is not a sound, and `copy` counts.
 *
 * The verb ends at the next letter or digit rather than at `\b`, because `_` is a
 * word character to `\b`: "_write_ it" (italic, said "write it") never matched.
 */
const WRITING_VERB = String.raw`(?:write|rewrite|draw|copy(?!\s+(?:the\s+)?(?:sound|rhythm|melody|intonation|pronunciation|pitch|tone|stress)\b(?!\s+(?:accents?\s+)?marks?\b))|circle|underline|sketch|jot)(?![a-z0-9])`;

/**
 * Where an English imperative can begin.
 *
 *   ^                     the start of a paragraph, list item, or the prose after a cue
 *   . ! ? ; :  then space the start of a sentence or clause; a closing `)`, `*`
 *                         or quote may sit between the stop and the space, so
 *                         "**Writing.** Write all five" and "(**ai**.) Write" count
 *
 * After that: an optional list marker ("4. ", "- "), an optional emphasis
 * opener ("**Write.**", "**Write:**", and the italic "*Write*" or "_Write_",
 * which the narrator strips just the same), and an optional connective ("Then
 * write", "Now write", "And write").
 *
 * The opener was bold-only until a security review: "*Read* the line" and
 * "_write_ it" are heard exactly as "Read the line" and "write it", and every
 * detector built on these links let them through. Accepting them changed no
 * verdict in the corpus when it was made; it closes the shape for the next one.
 */
const SENTENCE_START = String.raw`(?:^|(?<=[.!?;:][)*"”’]{0,3})\s+)(?:(?:[-*+]|\d+[.)])\s+)?(?:\*{1,2}|_{1,2})?(?:(?:then|now|and|also)\s+)?`;

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
 *   (?![*_]{0,2}\s+(?:is|was|means)\b)  "**Write** is Old English …" (or `_Write_ is`)
 *   (?!,)                             "draw, harden, fasten" — a gloss list
 *   (?!:\s)                           "Write: **लिहिणे**" — a vocabulary line
 *                                     ("**Write:** hear …" is a bold label,
 *                                     ":" then "*", and still fires)
 *   (?![^.!?]*\?\s*\()                "Write it down? (**ಬರೆದು ಕೊಡಿ**.)"
 */
const NOT_A_MENTION = String.raw`(?![*_]{0,2}\s+(?:is|was|means)\b)(?!,)(?!:\s)(?![^.!?]*\?\s*\()`;

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
  const spans: string[] = [];
  for (const part of narratedParts(markdown)) {
    if (!("text" in part)) continue;
    const prose = part.text.trim();
    if (prose !== "") spans.push(prose);
  }
  return spans;
}

/**
 * The same walk as {@link narratedProseSpans}, keeping the cues as well as the
 * prose between them: each paragraph, list item, table row and heading, cut by
 * the narration renderer's own splitter. `narratedProseSpans` keeps the prose;
 * {@link writingRecallCues} keeps the cues.
 */
function narratedParts(markdown: string): ReturnType<typeof splitNarrationCues> {
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
  return units.flatMap((unit) => splitNarrationCues(unit));
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
const CLAUSE_LEAD = String.raw`^(?:(?:[-*+]|\d+[.)])\s+)?(?:\*{1,2}|_{1,2})?(?:(?:then|now|and|also)\s+)?`;

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
const CHAIN_NOT_A_MENTION = String.raw`(?![*_]{0,2} (?:is|was|means)\b)(?!,)(?!: )`;

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
 *
 * A quotation that ENDS a sentence keeps its final stop, and only that:
 *
 *   Say “I do not understand.” Read **もうすこし**.
 *   Say “.” Read **もうすこし**.          the "." still ends the sentence
 *
 * Without it the blanked text read `Say “” Read **もうすこし**.`, one sentence
 * with "Read" in its middle, and the reading step after a quoted sentence was
 * invisible to every prose check (found when the modality rule's non-vacuity
 * test undid a READ cue in JA-C10-wakarimashita). The stop is the last
 * character of the run the pattern already matched, so this costs nothing.
 */
function withoutQuotations(clause: string): string {
  return clause
    .replace(/"[^"]*"/g, (quote) => `"${finalStop(quote.slice(1, -1))}"`)
    .replace(/“[^“”]*”/g, (quote) => `“${finalStop(quote.slice(1, -1))}”`);
}

/** The sentence stop a quoted run ends on, or nothing: "I do not understand." -> ".". */
function finalStop(quoted: string): string {
  const last = quoted.slice(-1);
  return last === "." || last === "!" || last === "?" ? last : "";
}

/**
 * Every span of `markdown` that a narrator would read as a bare instruction to
 * write. Returns the spans themselves, so a failure message can quote them.
 */
export function bareWritingImperatives(markdown: string): string[] {
  return narratedProseSpans(markdown).filter(proseAsksToWrite);
}

/**
 * Does one narrated prose span tell the listener to write? The predicate
 * behind {@link bareWritingImperatives}, exported so {@link eyesOrHandsSteps}
 * can judge every span in a single walk of the section.
 */
export function proseAsksToWrite(span: string): boolean {
  BARE_WRITING_IMPERATIVE.lastIndex = 0;
  if (BARE_WRITING_IMPERATIVE.test(span)) return true;
  return clausesOf(span).some(opensChainedOrFrontedWriting);
}

// ---------------------------------------------------------------------------
// Inside a recall cue
// ---------------------------------------------------------------------------
//
// Everything above judges prose, and treats a cue as already safe. For
// `[YOU WRITE: …]` that is right: WRITE is one of the narration's
// `MANUAL_CUE_ACTIONS`, so the cue is deferred ("once you have stopped
// driving — write: …"). It is not right for every cue. RECALL is not a manual
// action — recalling is something a driver can do — so the narration reads a
// recall cue out as an ordinary turn:
//
//     authored                                  narrated
//     ---------------------------------------   ----------------------------------------
//     [YOU RECALL: write **ば** — **R1**]        "your turn — recall: write ば — R1"
//     [YOU WRITE: **ば** from memory — **R1**]   "once you have stopped driving —
//                                                 write: ば from memory — R1"
//
// The first tells a driver to write, with nothing to say "not now". About a
// hundred and fifty spaced-recall cues across the drivable lessons had that
// shape — the spacing plan names a sign to retrieve, and the retrieval it asks
// for is a written one. The cue verb is the only thing the narration reads to
// decide whether to defer, so the fix is the verb: `[YOU WRITE: … from
// memory]`, keeping the spacing tag and saying in words what RECALL said in
// its name.
//
// Only RECALL is read inside. Its content is an instruction ("write **ば**",
// "say the Japanese for to write, then …"), so the prose rules apply to it
// as written. The content of `[YOU SAY: …]` and `[YOU ANSWER: …]` is not an
// instruction but the material to be spoken — "[YOU SAY: I write letters]" in
// a target language — and reading a writing verb in it would flag what the
// learner is meant to say.
//
// The content is judged three ways:
//
//   shape                                           example from the corpus
//   ---------------------------------------------   --------------------------------------------
//   it opens with a writing verb (the prose regex,  [YOU RECALL: write **け** — **R4**, eighty …]
//     anchored at the start of the content)         [YOU RECALL: draw the **।** and say what …]
//   a clause of it does (fronted or chained, the    [YOU RECALL: …; with the page covered, write …]
//     prose clause test)
//   a writing verb is chained on anywhere           [YOU RECALL: ask *kitthe?* and write it — **R2**]
//                                                   [YOU RECALL: answer *kuṭhe?* with *ithe*, then
//                                                     *tithe*, and write **तिथे** once]
//
// The third is the one prose does not get. A chain in prose counts only when a
// step verb from `CHAIN_OPENER` opens its clause, because prose is full of
// subjects ("wine is what you buy, ship, tax and write down"). A recall cue
// has no subject: RECALL is itself the step verb, and every clause inside the
// cue hangs off it. So ", and write …" anywhere in the cue is a step. The
// last example above is why that matters — the "?" inside *kuṭhe?* ends a
// clause for `clausesOf`, and the clause after it opens with "with", which
// the prose test rightly cannot treat as an instruction.
//
// The controls, all corpus cues that mention writing and ask for none:
//
//   [YOU RECALL: say the Japanese for to write, then the Japanese for to speak, …]
//                                       "to write" is a gloss: the verb is the
//                                       word being recalled, not the task
//   [YOU RECALL: point to the sign in **これ** you can already write, and the one you cannot]
//                                       "you can already write" describes the
//                                       learner; a chain link must PRECEDE the verb
//   [YOU RECALL: say how much space a written answer needs on a form — **R4**]
//                                       "written" is not an imperative
//   [YOU RECALL: *ek* and the letter **ए** you had to learn to write it]
//                                       a memory of writing, not a request for it
//   [YOU RECALL: the first letters you wrote — **р с н б д е т** — …]
//                                       the past tense never matches

/** Cue verbs whose content is an instruction the narration speaks unhedged. */
const INSTRUCTION_CUE_ACTIONS: ReadonlySet<string> = new Set(["RECALL"]);

/**
 * Does the content of a recall cue (the Markdown after its colon) ask for
 * writing? See the table above for the three shapes and the controls.
 *
 * Whitespace runs are collapsed in one pass and quotations are blanked with
 * the opener-excluding patterns of `withoutQuotations`. One step is NOT
 * linear on its own: `BARE_WRITING_IMPERATIVE` carries the prose test's
 * `NOT_A_MENTION` lookahead, which scans to the end of the sentence from every
 * candidate, so a sentence of N writing verbs with no stop costs O(N²) (the
 * prose path, `bareWritingImperatives`, has the same shape). What keeps it
 * cheap here is the cue bound: `closingBracket` gives up after
 * `MAX_CUE_LENGTH` (4,096) characters, so no cue content is longer than that,
 * and a maximal adversarial cue costs well under a millisecond.
 */
export function recallCueAsksForWriting(content: string): boolean {
  const text = content.replace(/\s+/g, " ").trim();
  BARE_WRITING_IMPERATIVE.lastIndex = 0;
  if (BARE_WRITING_IMPERATIVE.test(text)) return true;
  if (clausesOf(text).some(opensChainedOrFrontedWriting)) return true;
  return CHAIN_LINK.test(withoutQuotations(text));
}

/**
 * Every `[YOU RECALL: …]` cue in `markdown` that a narrator would read as a
 * spoken instruction to write. Returns the cues as authored (`source`), so a
 * failure message can quote them.
 */
export function writingRecallCues(markdown: string): string[] {
  const cues: string[] = [];
  for (const part of narratedParts(markdown)) {
    if (!("cue" in part) || part.cue.kind !== "prompt") continue;
    if (!INSTRUCTION_CUE_ACTIONS.has(part.cue.action)) continue;
    // The narration cue keeps only the Markdown-stripped instruction; the
    // patterns need the Markdown (`**`, `*`) they were written against, so the
    // authored cue is parsed again for its raw content.
    const raw = parseDeliveryCue(part.cue.source.slice(1, -1));
    if (raw?.kind !== "prompt") continue;
    if (recallCueAsksForWriting(raw.content)) cues.push(part.cue.source);
  }
  return cues;
}

/**
 * Everything in a drivable lesson that the narration would read to a driver as
 * an instruction to write: bare prose imperatives, then recall cues that ask
 * for writing.
 */
export function drivableWritingInstructions(markdown: string): string[] {
  return [...bareWritingImperatives(markdown), ...writingRecallCues(markdown)];
}

// ---------------------------------------------------------------------------
// Pointing at the page, inside a recall cue
// ---------------------------------------------------------------------------
//
// Writing is not the only thing a recall cue can ask of a hand. Twenty-four
// drivable Japanese recalls and one Hindi one asked the learner to put a
// finger on a printed sign:
//
//     [YOU RECALL: point to the sign in **でんわ** that carries the two-stroke
//       mark, and name the sign under it]
//     [YOU RECALL: say *mulāqāt*, then read **प्रणाम** and point to its **ण**]
//
// POINT is one of the narration's `MANUAL_CUE_ACTIONS`, so `[YOU POINT: …]` is
// deferred; inside RECALL the same request was read to a driver as an
// ordinary turn. The fix is the one the writing recalls got — say the same
// thing in a form the ear can do ("say which sign in **でんわ** carries the
// two-stroke mark, and name the sign under it") — and this is the check that
// keeps it fixed.
//
// It fires on "point to" or "point at" where a recall cue puts a step: at the
// start of the content, or straight after a step link (", ", ", and ", "; ",
// " and ", " then ", each optionally followed by "then "). Exactly "point":
// "points at something near" and "its old pointing stem" describe a word, and
// "point out" is a gloss ("the French for to point out"). A gloss of "to point
// at" is not caught either, because "to " is not a step link. Quotations are
// blanked first, as for the writing chain, so the material being recalled
// cannot fire it.
//
// "read" is not this check's business. A recall that says "then read **आँख**"
// asks for eyes too, and has its own check below ("Reading the page, inside
// any spoken cue"), because its fix is different: a reading step cannot be said
// for the ear, so it moves out into a `[YOU READ: …]` cue.
//
// Linear: whitespace runs are collapsed in one pass and quotations blanked
// with the opener-excluding patterns of `withoutQuotations`. The pattern is a
// choice of fixed literals, an optional fixed literal and two fixed words, so
// each start position does a bounded amount of work and no quantifier can
// match the same text two ways.

const POINTING_STEP = /(?:^|, (?:and )?|; | and | then )(?:then )?point (?:to|at)\b/i;

/** Does the content of a recall cue ask the learner to point at the page? */
export function recallCueAsksToPoint(content: string): boolean {
  const text = withoutQuotations(content.replace(/\s+/g, " ").trim());
  return POINTING_STEP.test(text);
}

/**
 * Every `[YOU RECALL: …]` cue in `markdown` that a narrator would read as a
 * spoken instruction to point at the page, quoted as authored.
 */
export function pointingRecallCues(markdown: string): string[] {
  const cues: string[] = [];
  for (const part of narratedParts(markdown)) {
    if (!("cue" in part) || part.cue.kind !== "prompt") continue;
    if (!INSTRUCTION_CUE_ACTIONS.has(part.cue.action)) continue;
    const raw = parseDeliveryCue(part.cue.source.slice(1, -1));
    if (raw?.kind !== "prompt") continue;
    if (recallCueAsksToPoint(raw.content)) cues.push(part.cue.source);
  }
  return cues;
}

// ---------------------------------------------------------------------------
// Reading the page, inside any spoken cue
// ---------------------------------------------------------------------------
//
// The spaced script-recognition recalls in the Indic tracks were authored as
// one spoken cue holding a reading step:
//
//     authored                                    narrated
//     -----------------------------------------   ------------------------------------------
//     [YOU RECALL: say *dīyā*, then read **कान**]  "your turn — recall: say dīyā, then read कान"
//
// A driver can say *dīyā*. They cannot read कान: reading printed script is the
// eyes' job, whatever the mouth does next, which is why READ is one of the
// narration's `MANUAL_CUE_ACTIONS`. Unlike pointing, there is no way to say a
// reading step for the ear — "say the word for ear" is a different exercise
// (production), not the recognition the recall was spaced to revisit. So the
// fix keeps the reading and moves it into a READ cue of its own, in the order
// the author gave, and leaves the spoken steps in the recall:
//
//     - [YOU RECALL: say *dīyā*]          your turn — recall: say dīyā
//     - [YOU READ: **कान**]               once you have stopped driving — read: कान
//
// The book prints the pair as "*Recall:* say *dīyā*" and "*Read it:* **कान**".
//
// Which cues are read
// -------------------
//
// Not only RECALL. The narration defers a cue when `isManualCueAction` says
// so and reads every other cue out as an ordinary turn, so a reading step
// inside ANY of them reaches a driver. The first pass of this check read
// RECALL only; the corpus then still had two other spoken verbs doing it:
//
//     [YOU RETURN TO: read **இன்று**, say *tuṭaippam* and say *kūrai* — three
//       distances back — then join two of them with -உம்]       (Tamil, 12 reviews)
//     [YOU SAY: *chha, sāt* — then read **સાત** sign by sign]    (Gujarati, 2)
//     [YOU RUN: both voices once from the romanization, then read the closing
//       line off the script alone]                              (Urdu, 1)
//
// So the check reads every cue the narration does not defer — in practice
// every cue whose head verb is in `SPOKEN_CUE_ACTIONS`, since
// `cue-action-classification.test.ts` fails on a head verb in neither set.
//
// SAY and ANSWER are the cues to be careful with, because their content is
// often the material to be spoken rather than an instruction — "[YOU SAY:
// "Leo un libro" — I read a book]". Three things keep that material quiet,
// and the controls in the test hold each one:
//
//   - "read" must sit where a STEP starts (the links below). In material it
//     follows a subject ("I read", "he always reads"), "to " ("to read", a
//     gloss), or nothing a link names.
//   - its object must be on the page: bold script, or something printed. A
//     gloss in italics or plain words ("*reception*", "read it — AH-weh")
//     is something said, not something seen.
//   - quotations are blanked first, so a quoted sentence that happens to say
//     "then read **…**" is the material, not the task.
//
// Measured over every spoken cue in the corpus that contains "read" (248
// cues: 123 RECALL, 108 SAY, 14 RETURN TO, one each of RUN, LIST and
// CONTRAST), the check fired, before the fix, on exactly 15 drivable cues —
// the RETURN TO, SAY and RUN cues above. In lessons that are not drivable it
// fires only on real reading steps: the reading recalls, five Bengali digit
// drills ("[YOU SAY: **এক**, then read **১**, **১০**, **১৩**]"), a Hindi
// stem ("then read the stem **बोल**") and two Tamil reviews. No SAY or
// ANSWER material fires: "I read Telugu", "to read", "read it — na · ma ·
// s · te" and the rest stay quiet.
//
// Where a step starts
// -------------------
//
// At the start of the content, or after a step link, each optionally followed
// by "then " or "now ":
//
//   link     example
//   -------  -------------------------------------------------
//   (start)  read **ऋ** — **R1**, one lesson back
//   ", "     say *ek*, read **एक**           (", and " too)
//   "; "     say *tīn*; read **त**
//   " and "  say *āṉāl* and read **எப்போது**
//   " then " say *dūdh* then read **आँख**
//   " — "    say *a* — read **क**            an em dash between two steps
//   ": "     say *a*: read **क**             a colon introducing the step
//   (+now)   say *a*, now read **क**
//
// "read" itself may carry a colon ("then read: **क**").
//
// What is read
// ------------
//
// "On the page" means one of three objects, each a shape the corpus had:
//
//   object                                     example from the corpus
//   -----------------------------------------  ----------------------------------------------
//   script, in bold, straight after the verb   [YOU RECALL: read **ऋ** — **R1**, one lesson back]
//     (or after "aloud", "out" or "it",        [YOU RECALL: say *ghās*, then read **कुआँ** and say …]
//     a colon allowed on the last of them)     … then read it: **क**
//   a short noun phrase that reaches script:   [YOU RECALL: read the sign **ೇ**, and say what it …]
//     a/an/the, at most four plain words, bold [YOU RECALL: read the form label **आवडती कृती** and …]
//                                              … read the very long sign **क**
//   something printed, or the script itself:  [YOU RECALL: read a printed ticket and say the figure …]
//     "printed" or "script" in that place      [YOU RUN: …, then read the closing line off the script alone]
//
// The controls, all corpus cues that say "read" and ask nobody to look:
//
//   [YOU RECALL: say the Japanese for to read, then …]
//                                       a gloss: "to " is not a step link
//   [YOU RECALL: say the line of your message that means *I read Marathi*]
//                                       material being recalled, after "I "
//   [YOU RECALL: read *reception* on a sign — **R1**, one lesson back]
//                                       an ITALIC English meaning, not printed
//                                       script: nothing is on the page, so the
//                                       learner retrieves the sign's word from
//                                       memory and says it, which an ear can do
//   [YOU RECALL: read *open*, then *not yet open*, and say which one lets you in]
//                                       the same, two meanings in a row
//   [YOU SAY: read it — AH-weh]         "it" is the word just heard; what
//                                       follows the dash is a pronunciation
//   [YOU SAY: "legō" — I read, hard g]  material: "I read", after a dash
//   [YOU SAY: **khândan** — to read]    a gloss after a dash
//
// Linear, and with no nested quantifier: whitespace runs are collapsed in one
// pass and quotations blanked as before. `READING_STEP` is a choice of fixed
// literals, an optional choice of two fixed literals and a fixed word, so each
// start position does a bounded amount of work; it ends in a one-character
// lookahead rather than consuming what follows "read", so "read then read **X**"
// still sees its second step. Each match then looks at most
// `READ_OBJECT_WINDOW` characters ahead, in plain code with a fixed number of
// words (`readsTheObjectOnThePage`), so a text of N characters costs O(N)
// however many "read"s it holds.

const READING_STEP = /(?:^|, (?:and )?|; | and | then | — |: )(?:then |now )?read(?=[ :])/gi;

/** How far past "read" the object test looks. Every corpus object fits well inside it. */
const READ_OBJECT_WINDOW = 80;

/** The words that may open a noun phrase whose head is printed script. */
const ARTICLES: ReadonlySet<string> = new Set(["a", "an", "the"]);

/** Words that may stand between the verb and its object: "read aloud **X**", "read it: **X**". */
const READ_PARTICLES: ReadonlySet<string> = new Set(["aloud", "out", "it"]);

/** At most this many particles ("read it aloud: **X**"). */
const MAX_READ_PARTICLES = 2;

/** At most this many plain words between the article and the script ("the very long sign **X**"). */
const MAX_NOUN_PHRASE_WORDS = 4;

/**
 * Words that put the object on the page by themselves: "a printed ticket",
 * "the closing line off the script".
 */
const PAGE_WORDS: ReadonlySet<string> = new Set(["printed", "script"]);

/** Is `word` one of the `PAGE_WORDS`, allowing one closing punctuation mark? */
function isPageWord(word: string): boolean {
  return PAGE_WORDS.has(word.toLowerCase().replace(/[,.;:]$/, ""));
}

/** A plain lower-case word, optionally closed by a colon ("sign:"). */
const PLAIN_WORD = /^[a-z]+:?$/;

/**
 * Is the text just after "read" something on the page? `ahead` starts at the
 * character after "read" (a space or a colon) and is at most
 * `READ_OBJECT_WINDOW` characters, and the loops below visit a fixed number of
 * words, so this is constant work per call.
 *
 *   " **कान**"                          yes  script
 *   ": **कान**"                         yes  script, after "read:"
 *   " aloud **कान**"                    yes  script, read aloud
 *   " it: **कान**"                      yes  script, after "read it:"
 *   " the sign **ೇ**, and say …"       yes  article, one plain word, script
 *   " the very long sign **ೇ**"        yes  article, three plain words, script
 *   " the sign: **ೇ**"                 yes  a colon closing the phrase, then script
 *   " a printed ticket and say …"       yes  printed
 *   " *reception* on a sign"            no   an italic meaning
 *   " it — AH-weh"                      no   a pronunciation, not script
 *   " out the number you heard"         no   no script, nothing printed
 *   " the whole line, then say **X**"   no   a comma ends the phrase before the script
 *   " the a b c d e **X**"              no   five plain words: past the bound
 */
function readsTheObjectOnThePage(ahead: string): boolean {
  const start = ahead.startsWith(":") ? 1 : 0;
  if (ahead[start] !== " ") return false;
  const words = ahead.slice(start + 1).split(" ");
  let index = 0;
  for (let particles = 0; particles < MAX_READ_PARTICLES; particles += 1) {
    const word = (words[index] ?? "").toLowerCase();
    const bare = word.endsWith(":") ? word.slice(0, -1) : word;
    if (!READ_PARTICLES.has(bare)) break;
    index += 1;
    if (bare !== word) break; // "it:" closes the run of particles
  }
  const first = words[index] ?? "";
  if (first.startsWith("**") || isPageWord(first)) return true;
  if (!ARTICLES.has(first.toLowerCase())) return false;
  // Up to four plain lower-case words, then bold script or a page word. A
  // colon on a plain word closes the phrase, so the next word must be bold.
  for (let step = 1; step <= MAX_NOUN_PHRASE_WORDS + 1; step += 1) {
    const word = words[index + step] ?? "";
    if (word.startsWith("**") || isPageWord(word)) return true;
    if (step > MAX_NOUN_PHRASE_WORDS || !PLAIN_WORD.test(word)) return false;
    if (word.endsWith(":")) return (words[index + step + 1] ?? "").startsWith("**");
  }
  return false;
}

/** Does the content of a spoken cue ask the learner to read script on the page? */
export function spokenCueAsksToReadScript(content: string): boolean {
  const text = withoutQuotations(content.replace(/\s+/g, " ").trim());
  for (const step of text.matchAll(READING_STEP)) {
    const from = (step.index ?? 0) + step[0].length;
    if (readsTheObjectOnThePage(text.slice(from, from + READ_OBJECT_WINDOW))) return true;
  }
  return false;
}

/**
 * Every cue in `markdown` that the narration reads out as an ordinary turn
 * (any action `isManualCueAction` does not defer) and that asks for printed
 * script to be read, quoted as authored.
 */
export function readingSpokenCues(markdown: string): string[] {
  const cues: string[] = [];
  for (const part of narratedParts(markdown)) {
    if (!("cue" in part) || part.cue.kind !== "prompt") continue;
    if (isManualCueAction(part.cue.action)) continue;
    const raw = parseDeliveryCue(part.cue.source.slice(1, -1));
    if (raw?.kind !== "prompt") continue;
    if (spokenCueAsksToReadScript(raw.content)) cues.push(part.cue.source);
  }
  return cues;
}

// ---------------------------------------------------------------------------
// A gesture, inside any spoken cue
// ---------------------------------------------------------------------------
//
// The last two sections read a cue for the page. This one reads it for the
// body. A spoken cue is read to a driver as an ordinary turn, so any hand
// movement folded into it is asked of someone holding a steering wheel:
//
//     authored                                            narrated
//     --------------------------------------------------  ----------------------------------------
//     [YOU HEAR: *añcŭ*; YOU SHOW: 5]                     "your turn — hear: añcŭ; YOU SHOW: 5"
//     [YOU SAY: *denwa*, clapping three beats]            "your turn — say: denwa, clapping …"
//     [YOU SAY: "இங்கே" three times, pointing at …]        "your turn — say: இங்கே three times, …"
//
// The corpus had 135 of them in drivable lessons (issue #12070, ninth pass):
// 66 demonstrative drills in six Indic tracks ("three times, pointing at
// something different each time"), 24 Japanese mora drills ("clapping three
// beats"), finger counting in the Tamil and Malayalam number lessons (four
// nested `YOU SHOW: N` cues among them), touching a body part as it is named,
// raising each hand for right and left, a hand-wobble, a small bow, and
// labels that name a gesture ("[YOU SAY: pointing at them — *ei bhāirā*]"),
// which a listener at speed cannot tell from a request for one.
//
// Unlike reading, almost every one of these has an ear-and-voice form that
// keeps the learning goal, so the fix is a rewrite rather than a split:
// counting the beats aloud instead of clapping them ("then count its beats
// aloud — three"), picturing the thing a demonstrative lands on instead of
// pointing at it, saying the number heard instead of showing it, "say
// *right* or *left* after each" instead of raising each hand.
//
// Which cues are read
// -------------------
//
// Every cue the narration does not defer, exactly as for reading: a gesture
// inside a `[YOU POINT: …]` or `[YOU WRITE: …]` cue is already deferred.
//
// What fires
// ----------
//
// Three shapes, each a fixed vocabulary:
//
//   shape                                         example from the corpus
//   --------------------------------------------  -----------------------------------------------
//   a gesture verb where a step starts            [YOU SAY: *eki*, clapping two beats]
//     (the step links of the reading check,       [YOU SAY: *kandhā*, and touch it]
//     plus " while ", " as you " and " by ")      [YOU SAY: … *añcŭ* while raising one more finger]
//                                                 [YOU SAY: point to one person, ask **¿Quién?** …]
//                                                 [YOU SAY: *mo | o* and tap twice]
//   a hand or body manner phrase, anywhere        [YOU SAY: *tohfā*, offered with both hands]
//                                                 [YOU SAY: "vaṇakkam" with a small bow]
//                                                 [YOU SAY: hello / goodbye, palms together — …]
//   a nested cue whose verb is manual             [YOU HEAR: *mūnnŭ*; YOU SHOW: 3]
//
// The gesture verbs, each only in the forms that are always a gesture:
// "point"/"pointing" (not "point out", a gloss, and not "points", which
// describes a word), "clap", "tap" only with "twice", "once", "out" or a count
// of beats, groups, syllables or morae (a bare "tap on the *r*" is the tongue's
// tap consonant), "touch" with a pronoun or an indefinite object (not
// "touching the sound once", which is a Malayalam single consonant), "raise"
// with a finger or a hand (not "raise the pitch"), "hold up", "show" with a
// number of fingers, "gesture"/"gesturing", "wave goodbye"/"wave your hand"
// (not "wave away an apology", a speech act), "handing over" and "handing
// something over" (only the participle, the manner of saying a word; "hand
// over a gift" and "hands over" are English meanings to put into the
// language), "nod" and "shake your head".
//
// A nested cue is read with the narration's own `isManualCueAction`, so the
// day a lesson nests `YOU WRITE:` (or a longer action such as
// `YOU WRITE FROM MEMORY:`) inside a `[YOU SAY: …]`, it is caught by the
// same list that decides deferral. SHOW and CLAP joined `MANUAL_CUE_ACTIONS`
// for this: no cue heads with either now, but SHOW is the verb the nested
// finger-counting cues used.
//
// The controls, all corpus cues that mention a hand or a gesture and ask for
// none:
//
//   [YOU RECALL: say the Tamil for to touch, then …]   a gloss: "for to " is no link
//   [YOU SAY: a heel, a fist, a palm, the liver, a lung] vocabulary being taught
//   [YOU SAY: "kai" — hand]                            a word and its meaning
//   [YOU SAY: wave away an apology — *paravāgilla*]    a speech act, not a wave
//   [YOU SAY: "gracias" — *GRAH-syahs*, one soft tap on the *r*]
//                                                      the tongue's tap, a noun
//   [YOU SAY: *kuṭi*, touching the sound once]         a single consonant
//   [YOU SAY: "comme ci, comme ça" — with a little hand-wobble in the voice]
//                                                      the voice does the wobble
//   [YOU SAY: the three pointing and person words you now own — …]
//                                                      "pointing" names the words
//   [YOU SAY: *koṭu*, then *vāṅgu* — and say which way each hand is moving]
//                                                      a thing to say about hands
//   [YOU RECALL: the plain *ch* you tested with a hand at your mouth]
//                                                      NOT a control: a memory,
//                                                      but "hand at your mouth"
//                                                      is a manner phrase and
//                                                      fires; the one corpus cue
//                                                      of this shape was reworded
//
// Measured over every spoken cue in the corpus before the fix, the check fired
// on exactly the 135 drivable cues that were rewritten and, in lessons that
// are not drivable, on 29 cues that are real gesture work there (Kannada digit
// lessons that point at a printed figure, aspiration drills with a hand in
// front of the mouth, Bengali and Urdu demonstrative reviews, two Japanese
// writing recalls that clap). One drivable cue was rewritten without the check
// seeing it: "[YOU SAY: all five in order, then say *namaste* and name what
// your hands are doing]" presupposes the gesture without naming a movement,
// and a pattern for "what your hands are doing" would be a pattern for one
// sentence.
//
// Linear, with no nested quantifier: whitespace runs are collapsed and
// quotations blanked as before. `GESTURE_STEP` is a fixed-literal link, an
// optional fixed-literal connective and a choice of fixed phrases, each with
// at most optional single characters or optional fixed words, so each start
// position does a bounded amount of work. `GESTURE_MANNER` is a choice of
// fixed phrases. `NESTED_CUE` is a fixed literal and one run of capitals,
// which no other part of the pattern can also match. Each regex is applied
// once per cue (the nested one as one whole-text pass), so a text of N characters
// costs O(N).

/** Where a step starts in a spoken cue: the reading check's links, plus a participle's "while", "as you" and "by". */
const GESTURE_LINK = String.raw`(?:^|, |; | — |: | and | then | while | as you | by )(?:and then |and |then |now |also )?`;

/** The gesture verbs, each only in a form that is always a movement of the body. See the header. */
const GESTURE_VERB = String.raw`(?:point(?:ing)?(?! out\b)(?=[ ,.;:]|$)|clap(?:s|ping)?\b|tap(?:ping)? (?:twice|once|out)\b|tap(?:ping)? (?:the |its )?(?:two |three |four |five |six |even )?(?:beats?|groups?|syllables?|morae)\b|touch(?:ing)? (?:it|them|each|your|yourself|one|a|an|something)\b|rais(?:e|ing) (?:one more|one|a|each|your|both) (?:finger|hand)s?\b|hold(?:ing)? (?:up|it up|them up)\b|show(?:ing)? (?:\d|(?:one|two|three|four|five|your) fingers?\b)|gestur(?:e|ing)\b|wav(?:e|ing) (?:goodbye|your hand|a hand)\b|handing (?:it |them |something |things )?over\b|nod(?:ding)?\b|shak(?:e|ing) your head\b)`;

const GESTURE_STEP = new RegExp(GESTURE_LINK + GESTURE_VERB, "i");

/** A hand or a bow as the manner of saying something, wherever it sits in the cue. */
const GESTURE_MANNER =
  /with (?:a|one|both|your|each) (?:small )?(?:hands?|bow|head-bow)\b|\b(?:palms|hands) together\b|\bpressed palms\b|\bmatching gesture\b|\bhand (?:at|in front of) (?:your|the) mouth\b/i;

/**
 * A cue nested inside another cue's content: "[YOU HEAR: *añcŭ*; YOU SHOW: 5]".
 * Case-sensitive, as the cue grammar is. The action may be several words
 * ("YOU WRITE FROM MEMORY:"), but at most five: a fixed bound, so a long run
 * of "YOU YOU YOU …" with no colon costs a bounded scan from each start rather
 * than one to the end of the text.
 */
const NESTED_CUE = /\bYOU ([A-Z]+(?: [A-Z]+){0,4}):/g;

/** Does the content of a spoken cue ask the learner to make a gesture? */
export function spokenCueAsksForGesture(content: string): boolean {
  const text = withoutQuotations(content.replace(/\s+/g, " ").trim());
  if (GESTURE_STEP.test(text) || GESTURE_MANNER.test(text)) return true;
  for (const nested of text.matchAll(NESTED_CUE)) {
    if (isManualCueAction(nested[1] ?? "")) return true;
  }
  return false;
}

/**
 * Every cue in `markdown` that the narration reads out as an ordinary turn
 * (any action `isManualCueAction` does not defer) and that asks for a gesture,
 * quoted as authored.
 */
export function gestureSpokenCues(markdown: string): string[] {
  const cues: string[] = [];
  for (const part of narratedParts(markdown)) {
    if (!("cue" in part) || part.cue.kind !== "prompt") continue;
    if (isManualCueAction(part.cue.action)) continue;
    const raw = parseDeliveryCue(part.cue.source.slice(1, -1));
    if (raw?.kind !== "prompt") continue;
    if (spokenCueAsksForGesture(raw.content)) cues.push(part.cue.source);
  }
  return cues;
}

// ---------------------------------------------------------------------------
// Reading the page, or handling cards, in prose
// ---------------------------------------------------------------------------
//
// Everything since "Inside a recall cue" reads cues. Bare prose is read aloud
// just as it is written, so a prose instruction to read printed script reaches
// a driver too (issue #12070, tenth pass):
//
//     authored                                       narrated
//     ---------------------------------------------  --------------------------------------
//     Hear, picture the part, say, and read **は**.   "Hear, picture the part, say, and read は."
//     Then take six meaning cards, say each word,    "Then take six meaning cards, …"
//       and read the six character cards.
//     Look, cover, and wait five seconds.            "Look, cover, and wait five seconds."
//
// The fix was the one the reading cues got: the reading or card step moves into
// a cue the narration defers ("Hear, picture the part, and say **は**. [YOU
// READ: **は**]"), and a step that has an ear form gets it ("Then, from six
// English meanings, say each word", "Say that again and listen for the verb").
// This check keeps the narrow, mechanical part of that fixed.
//
// What fires
// ----------
//
// Four shapes, each a closed vocabulary, judged on the narrated prose spans the
// writing check reads (`narratedProseSpans`, so a cue is never prose):
//
//   shape                                          example from the corpus
//   ---------------------------------------------  --------------------------------------------
//   "read" where a step starts, with its object    Read **いちど**. Say *sumimasen* …
//     on the page: script (bold, or any non-Latin  3. **Reading:** read all six character cards
//     word), "printed", "script", a card, or       Read the five printed shuffled …
//     "down"/"across" a list                       **Read.** 我是中国人 — 你是中国人吗
//     (see `proseReadsThePage`)                    Read down once, without stopping.
//   a card-handling verb where a step starts,      Shuffle five unpointed cards: …
//     with "card"/"cards" before the clause ends   Then take six meaning cards …
//   a card named by what is printed on it,         produce all six from meaning cards
//     anywhere: meaning, character, printed or
//     unpointed card(s)
//   cover, uncover or hide where a step starts,    Look, cover, and wait five seconds.
//     with an object or a following link          Hide the pinyin. Read 书 …
//
// The step links are the reading cue check's, minus the em dash and the colon
// (in prose an em dash opens a gloss — "*Cubre la olla* — cover the pot" — and
// a colon is already a sentence start), plus the sentence starts of the
// writing check (". ", "! ", "? ", "; ", ": " with an optional closing `)`,
// `*` or quote between the stop and the space, so "**Reading:** read" counts).
//
// The controls, all corpus prose that says "read", "card" or "cover" and asks
// nobody to look:
//
//   Read it literally and it says "it is two hours"     interpretation: "it" is not on the page
//   Read it in Kannada order: **ಅದು ನಿಜ** (the thought)   a particle, then "in": no object
//   the same stack you have written in **नमस्ते** and     a participle: "read in" is not an object
//     read in **स्टेशन**
//   Why is its **ட** read as a soft *ḍ*?               no step link before "read"
//   **من فضلك** — read right to left, **min faḍlik**    a gloss after an em dash
//   Read a notice like this and act on it               advice for the street: nothing bold
//   say the Bengali for an identity card               a gloss: "for" is not a step link
//   Name what the card hanging on it says              "name" is not a handling verb
//   English lets *sit* cover both                      no step link before "cover"
//   *Cubre la olla* — cover the pot                    a gloss after an em dash
//
// Measured over the corpus as it stood before the fix, the check fired on 418
// prose spans in 391 drivable lessons (Marwadi 120, Japanese 53, Chinese 48,
// Tamil 28, and the rest in every other track but Spanish), each of which this
// change rewrote; afterwards, on none. In lessons that are not drivable it
// fires on 740 spans in 603 lessons, and a sample of them is all real reading,
// card and look-cover-write work, which those lessons' narration already
// hedges with its hands-and-eyes notice.
//
// A review follow-up widened three things, each still a closed literal: " or "
// is a step link ("turn the page or cover every Arabic model"), "cover up"
// covers, and a bare "cover" may end its step with " and wait", " and write"
// or " and say" as well as ", and " ("Look, cover and wait"). Over the
// pre-fix corpus that changed nothing in drivable lessons (still 418 spans) and
// added one span elsewhere (AR-W00-full-greeting-recall, a real
// turn-the-page-or-cover step).
//
// Two widenings were measured and left out, because they were not precise:
// a bare "read," in a chain ("hear, say, read, and write") is as often a list
// of the skills a lesson teaches, heading included, as a step; and "cover,
// then" also matches a stroke the Chinese writing lessons call "cover" ("top
// slant, cover, then the middle"). Those, like the rest of this list, were
// inventoried by hand instead.
//
// Deliberately out of scope, because no closed vocabulary separates them from
// description: an object that is a passage rather than script ("Read it once
// without stopping", fixed by inventory), a look at a table ("Look down the two
// columns"), and the conditional narrative "Look up **पहिला** in Molesworth's
// dictionary and it is there". Those were inventoried by reading every
// drivable lesson that says read, look, see, watch, card, cover, page or
// printed, and are recorded in each track's changelog.
//
// Linear: whitespace runs are collapsed and quotations blanked as for the cue
// checks. Each step regex is a fixed-literal link, an optional fixed literal and
// a fixed verb, ending in a lookahead; each match then looks at most
// `READ_OBJECT_WINDOW` characters ahead in plain code over a fixed number of
// words. `PRINTED_CARD` is fixed literals. So a span of N characters costs O(N).

/**
 * Where a step starts in prose: a sentence start, or a step link. No em dash: in
 * prose it opens a gloss.
 *
 * The start of the span may carry a list marker ("- ", "* ", "3. ", "2) "), as
 * the writing check's `SENTENCE_START` always allowed. `narratedProseSpans`
 * starts a new span at every list item but keeps its marker, so without this a
 * list item "- Read **নাম**." was not a step at all — found when the
 * modality rule's non-vacuity test undid the READ cues in real lessons and the
 * list-item ones went unseen. The marker is a fixed literal and optional, and
 * only at `^`, so the link stays linear.
 */
const PROSE_LINK = String.raw`(?:^(?:(?:[-*+]|\d+[.)]) )?|[.!?;:]["”’)*]{0,3} |, (?:and |or )?|; | and | or | then )(?:\*{1,2}|_{1,2})?(?:then |now |and |also |first )?`;

/**
 * "read" where a step starts. A closing emphasis marker after it is consumed
 * ("*Read* the line", "_read_ it"), so what follows reaches the object check as
 * it does after a plain "read"; a closing "**" is left for that check, which has
 * always read past it.
 */
const PROSE_READING_STEP = new RegExp(`${PROSE_LINK}read(?=[ :.]|\\*\\*)|${PROSE_LINK}read(?:\\*|_{1,2})(?=[ :.])`, "gi");

/** Card-handling verbs; the card itself must follow before the clause ends. */
const CARD_STEP = new RegExp(
  `${PROSE_LINK}(?:turn|shuffle|take|place|put|pick up|reverse|sort|deal|lay out|hold up|match|read)\\b`,
  "gi",
);

/** A card named by what is printed on it is always a card on the table. */
const PRINTED_CARD = /\b(?:meaning|character|printed|unpointed) cards?\b/i;

/** Covering the page: "Look, cover, and wait", "Hide the pinyin", "Cover it". The object is judged by `coversThePage`. */
const COVER_STEP = new RegExp(`${PROSE_LINK}(?:cover|uncover|hide)(?=[ ,.])`, "gi");

/** What may follow a bare "cover" that ends its step: "Look, cover, and wait", "Look, cover and wait". */
const BARE_STEP_ENDS: readonly string[] = [", and ", " and wait", " and write", " and say", "."];

/** What a hand covers on the page, after a determiner and at most three plain words. */
const PAGE_NOUNS: ReadonlySet<string> = new Set([
  "page", "pages", "model", "models", "line", "lines", "word", "words", "text", "answer", "answers",
  "romanization", "romanizations", "pinyin", "english", "figures", "card", "cards", "side", "left", "right",
]);

/**
 * Is the text just after "cover", "uncover" or "hide" the page? Constant work
 * over at most `READ_OBJECT_WINDOW` characters.
 *
 *   ", and wait five seconds"      yes  a bare step in a chain ("Look, cover, and wait")
 *   "."                            yes  a bare step ending its sentence
 *   " it and wait five seconds"    yes  a pronoun: the thing just read
 *   " **कांई**, and wait"          yes  script
 *   " the pinyin."                 yes  a page noun
 *   " the right-hand side and"     yes  a page noun within three words
 *   " the pattern that makes it"   no   not a thing on the page
 *   " both; French makes you"      no   "both" alone is not on the page
 */
function coversThePage(ahead: string): boolean {
  if (BARE_STEP_ENDS.some((end) => ahead.startsWith(end))) return true;
  const words = ahead.trim().split(" ");
  if ((words[0] ?? "").toLowerCase() === "up") words.shift();
  const first = (words[0] ?? "").toLowerCase().replace(/[,.;:]$/, "");
  if (first === "it" || first === "them" || first === "everything" || first.startsWith("**")) return true;
  if (!PROSE_DETERMINERS.has(first) && first !== "every" && first !== "your") return false;
  for (let step = 1; step <= 3; step += 1) {
    const word = (words[step] ?? "").toLowerCase().replace(/[,.;:]$/, "");
    if (PAGE_NOUNS.has(word) || PAGE_NOUNS.has(word.replace(/^(?:right|left)-hand$/, "side"))) return true;
    if (!/^[a-z-]+$/.test(word)) return false;
  }
  return false;
}

/** Words that may open a noun phrase in prose: articles, possessives, quantities. */
const PROSE_DETERMINERS: ReadonlySet<string> = new Set([
  "a", "an", "the", "its", "each", "every", "both", "all", "this", "these", "that", "those",
  "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
]);

/** Words that put the object on the page by themselves, in prose. */
const PROSE_PAGE_WORDS: ReadonlySet<string> = new Set(["printed", "script", "card", "cards"]);

/** Directions that only a list on the page has: "Read down once", "read across each line". */
const PAGE_DIRECTIONS: ReadonlySet<string> = new Set(["down", "across"]);

function isProsePageWord(word: string): boolean {
  return PROSE_PAGE_WORDS.has(word.toLowerCase().replace(/[,.;:]$/, ""));
}

/**
 * Is `word` written in a script other than the Latin alphabet ("书", "我是中国人")?
 * Prose often leaves a short Chinese or Japanese form unbolded, and no English
 * word starts with one, so a step that reads it is reading the page.
 */
function isNonLatinScript(word: string): boolean {
  return /^\p{L}/u.test(word) && !/^\p{Script=Latin}/u.test(word);
}

/**
 * Nouns that only a page has, for "read the …" in prose: "Read these shuffled
 * numerals", "Read the visible row once". A listener can hear a word or a
 * sentence; a row, a column or a numeral is somewhere on the page.
 * "Word", "sentence" and "notice" are deliberately absent: "Read a notice like
 * this and act on it" is advice for the street, and "read the words" is as often
 * a figure of speech as a step. So is "line": a line of a dialogue is heard as
 * readily as it is read, and "then read the whole line" is pinned as nothing
 * printed. A line read ONCE is still caught, by the manner ("Read the three
 * lines once"), not by the noun.
 */
const READ_PAGE_NOUNS: ReadonlySet<string> = new Set([
  "row", "rows", "column", "columns", "numeral", "numerals",
  "digit", "digits", "label", "labels", "bank",
]);

/**
 * What the learner has written, after "your": "Read your repaired word", "Read
 * your copy", "read your own column back". The writing is on the page by
 * definition, so reading it back needs eyes.
 */
const YOUR_WRITING_NOUNS: ReadonlySet<string> = new Set([
  "copy", "word", "words", "answer", "answers", "greeting", "column", "digits",
  "line", "lines", "sentence", "handwriting", "writing", "page",
]);

/**
 * A pronoun or a count, then the manner of a reading pass. Case-sensitive on
 * purpose, so "the" plus one or two words cannot be "the German" or "the
 * Sanskrit" (a language named, not a thing printed). Every quantifier is
 * bounded and every alternative opens with a distinct literal, so a match
 * attempt does a fixed amount of work: this is applied once, anchored, to a
 * window of at most `READ_OBJECT_WINDOW` characters.
 *
 *   object                       manner
 *   ---------------------------  -------------------------------------------
 *   it, them, both, each (one),  once, twice, again, back; sign by sign;
 *   all <count>, the <1-2 words>   one (small) piece at a time; ", and say",
 *                                  ", then answer"; or aloud (not "aloud as",
 *                                  "aloud in", "aloud and", which describe)
 *
 * There is no bare form ("Read once before checking"): in prose, "taught once
 * and read once" is a description, and the bare form cannot tell them apart.
 */
const READ_OBJECT_THEN_MANNER = new RegExp(
  String.raw`^[.:]?(?:\*\*)? (?:(?:it|them|both|each(?: one)?|all [a-z]+|the(?: [a-z]+){1,2})(?: aloud)?(?:,? (?:and |then ){1,2}(?:say|answer)\b| (?:once|twice|again|back)\b| (?:sign|piece|letter) by (?:sign|piece|letter)\b| one (?:small )?piece at a time\b)|(?:it|them|both|each(?: one)?|all [a-z]+) aloud\b(?! (?:as|in|and)\b))`,
);

/** The learner's own writing as the object: "Read what you wrote". */
const WHAT_YOU_WROTE = /^ what you (?:wrote|have written|copied)\b/;

/**
 * A source on the page anywhere in the reading step's clause: "Read the
 * question aloud from the page", "Read the answer once from your own
 * handwriting". The run before it stops at the clause's end and is bounded,
 * and the lazy quantifier can only extend over that bounded run, so each call
 * is constant work.
 */
const FROM_THE_PAGE = /^[^.;!?]{0,60}? (?:from|off) (?:the|your(?: own)?) (?:page|handwriting|script|screen)\b/;

/**
 * The four objects of a reading step that carry no script themselves, but put
 * the step on the page anyway. See the second table on {@link proseReadsThePage}.
 */
function readsAPageThingWithoutScript(ahead: string): boolean {
  if (READ_OBJECT_THEN_MANNER.test(ahead)) return true;
  if (WHAT_YOU_WROTE.test(ahead) || FROM_THE_PAGE.test(ahead)) return true;
  const words = ahead.trim().split(" ");
  const first = (words[0] ?? "").toLowerCase();
  const nouns = first === "your" ? YOUR_WRITING_NOUNS : PROSE_DETERMINERS.has(first) ? READ_PAGE_NOUNS : null;
  if (nouns === null) return false;
  // Up to four plain words after the determiner, the noun among them. Plain
  // means lower-case letters with at most one closing comma, colon or stop, so
  // "the German" or "the **…**" end the phrase.
  for (let step = 1; step <= MAX_NOUN_PHRASE_WORDS; step += 1) {
    const word = words[step] ?? "";
    if (!/^[a-z]+[,.;:]?$/.test(word)) return false;
    if (nouns.has(word.replace(/[,.;:]$/, ""))) return true;
    if (word !== word.replace(/[,.;:]$/, "")) return false;
  }
  return false;
}

/**
 * Is the text just after a prose "read" something on the page? Like
 * `readsTheObjectOnThePage`, constant work per call over at most
 * `READ_OBJECT_WINDOW` characters, with three prose additions: a bold label's
 * closing ("**Read.** 我是…", "read:"), a determiner that is a quantity or a
 * possessive ("read its unpointed card", "Read twelve printed figures"), and
 * the directions only a printed list has ("Read down once").
 *
 *   " **いちど**."                   yes  script
 *   ".** 我是中国人 — …"              yes  a bold label, then script
 *   " the six character cards"       yes  a card
 *   " its unpointed card."            yes  a possessive, then a card
 *   " twelve printed figures"         yes  a quantity, then printed
 *   " 书, hold the level first tone"   yes  script that is not bold
 *   " down once, without stopping."   yes  a list on the page
 *   " it literally and it says"       no   a particle, then nothing printed
 *   " it in Kannada order: **…**"     no   "in" is not an object
 *   " in **स्टेशन**"                  no   a participle's place, not an object
 *   " a notice like this and act"     no   no script within four plain words
 *
 * Four more objects joined when the check began to decide modality as well as
 * gate prose (see "Promoted into the classifier" below). Each is a shape the
 * drivable cores of the Indic and Urdu tracks had, and each still needs the
 * READ to sit where a step starts:
 *
 *   " these shuffled numerals:"        yes  a determiner, then a thing only a page
 *   " the visible row once"            yes    has: rows, columns, numerals, digits,
 *                                             labels, a bank
 *   " your repaired word as *sa-LU*"   yes  "your", then something the learner
 *                                             wrote: copy, word, answer, column …
 *   " what you wrote once."            yes  the learner's own writing
 *   " the question aloud from the page" yes a source on the page: from/off the
 *                                             page, the script, your handwriting
 *   " the three lines once"            yes  a pronoun, a count or "the" and up to
 *   " it, and say which plain letter"  yes    two words, then the manner
 *   " each one and say what it means"  yes    of a reading step: once, twice,
 *   " it one small piece at a time."   yes    again, back, aloud, sign by sign, or
 *   " both, then answer."              yes    "and say" / "then answer" after it
 *   " it as a sum now rather than"     no   "read X as Y" is interpretation
 *   " once. Say each aloud"            no   a bare "once" ("taught once and read
 *                                             once") describes as often as it asks
 *   " them for now."                   no   no manner, no page
 *   " back together at a distant"      no   "back" needs an object before it;
 *                                             bare, it is a participle's adverb
 *   " it aloud in the order you"       no   "aloud in …" describes a misreading
 *   " the frame as three beats:"       no   "frame" is not a page noun
 */
export function proseReadsThePage(ahead: string): boolean {
  // "Read **は** as *ha* inside a word" is NOT excused as interpretation the way
  // "read it as a sum" is (the object above is no script, so it never fires).
  // With script in the object the two readings cannot be told apart: "Read
  // **は | な** as *ha-na*" is a reading step with a pronunciation attached. The
  // asymmetry of this module decides it: a rule about reading is rewritten for
  // the ear ("は is said *ha* inside a word"), and the step stays caught.
  if (readsAPageThingWithoutScript(ahead)) return true;
  let start = 0;
  if (ahead[start] === "." || ahead[start] === ":") start += 1;
  if (ahead.startsWith("**", start)) start += 2;
  if (ahead[start] !== " ") return false;
  const words = ahead.slice(start + 1).split(" ");
  let index = 0;
  for (let particles = 0; particles < MAX_READ_PARTICLES; particles += 1) {
    const word = (words[index] ?? "").toLowerCase();
    const bare = word.endsWith(":") ? word.slice(0, -1) : word;
    if (!READ_PARTICLES.has(bare)) break;
    index += 1;
    if (bare !== word) break;
  }
  const first = words[index] ?? "";
  if (first.startsWith("**") || isProsePageWord(first) || isNonLatinScript(first)) return true;
  if (index === 0 && PAGE_DIRECTIONS.has(first.toLowerCase())) return true;
  if (!PROSE_DETERMINERS.has(first.toLowerCase())) return false;
  for (let step = 1; step <= MAX_NOUN_PHRASE_WORDS + 1; step += 1) {
    const word = words[index + step] ?? "";
    if (word.startsWith("**") || isProsePageWord(word)) return true;
    if (step > MAX_NOUN_PHRASE_WORDS || !PLAIN_WORD.test(word)) return false;
    if (word.endsWith(":")) return (words[index + step + 1] ?? "").startsWith("**");
  }
  return false;
}

/**
 * Does the clause after a card-handling verb name a card before it ends?
 * `ahead` is at most `READ_OBJECT_WINDOW` characters, so this is constant work.
 */
function handlesACard(ahead: string): boolean {
  const end = ahead.search(/[.;!?]/);
  return /\bcards?\b/i.test(end === -1 ? ahead : ahead.slice(0, end));
}

/** Does one narrated prose span ask the listener to read printed script, handle cards, or cover the page? */
export function proseAsksToReadOrHandleCards(span: string): boolean {
  const text = withoutQuotations(span.replace(/\s+/g, " ").trim());
  if (PRINTED_CARD.test(text)) return true;
  for (const step of text.matchAll(COVER_STEP)) {
    const from = (step.index ?? 0) + step[0].length;
    if (coversThePage(text.slice(from, from + READ_OBJECT_WINDOW))) return true;
  }
  for (const step of text.matchAll(PROSE_READING_STEP)) {
    const from = (step.index ?? 0) + step[0].length;
    if (proseReadsThePage(text.slice(from, from + READ_OBJECT_WINDOW))) return true;
  }
  for (const step of text.matchAll(CARD_STEP)) {
    const from = (step.index ?? 0) + step[0].length;
    if (handlesACard(text.slice(from, from + READ_OBJECT_WINDOW))) return true;
  }
  return false;
}

/** Every narrated prose span of `markdown` that asks to read the page, handle cards, or cover the page. */
export function readingOrCardProse(markdown: string): string[] {
  return narratedProseSpans(markdown).filter(proseAsksToReadOrHandleCards);
}

// ---------------------------------------------------------------------------
// Pointing, or a gesture, in prose
// ---------------------------------------------------------------------------
//
// The cue checks above read "point to" inside a recall and gestures inside any
// spoken cue. Bare prose asks for both too, and it is read aloud as written:
//
//     authored                                        narrated
//     ----------------------------------------------  -----------------------------------
//     Point to **ش · ک · ر** and say *sh · k · r*.     "Point to ش · ک · ر and say sh k r."
//     Give its meaning and point to the final verb    "… and point to the final verb in
//       in **મારું નામ મીરા છે**.                         મારું નામ મીરા છે."
//     Say **namaskāra** with a small bow.             "Say namaskāra with a small bow."
//
// What fires
// ----------
//
//   shape                                          why it is not a mention
//   ---------------------------------------------  ------------------------------------------
//   "point to" / "point at" where a step starts    the prose step links of the reading check
//     (`PROSE_LINK`), not before a wh-word           ("point at what it means" names a job a
//                                                    word does, in a recall's answer)
//   a hand or a bow as the manner of a clause      the clause must open with a speech verb
//     (`GESTURE_MANNER`) whose verb is spoken:       (say, greet, answer, …): "*Vaṇakkam* is
//     "Say **namaskāra** with a small bow"           said with pressed palms" describes the
//                                                    custom, and "Imagine meeting someone
//                                                    with your palms together" asks for a
//                                                    picture, not a movement
//
// Measured over the whole corpus when this arrived, the two rules fired on 162
// prose spans (157 pointing, 5 manner), every one of them in a lesson that
// needs eyes in full (writing lessons pointing at a model, form drills
// pointing at a label), so their precision is the cue checks'. The five in
// drivable cores — three pointing steps and two "Say **namaskāra** with a
// small bow" — were real.
//
// Linear: whitespace runs are collapsed and quotations blanked as for the other
// prose checks; `PROSE_POINTING_STEP` is a fixed-literal link, an optional fixed
// literal, two fixed words and a lookahead over fixed words. The manner check
// splits the span into clauses once (`clausesOf`) and runs two anchored or
// fixed-literal patterns per clause.

const PROSE_POINTING_STEP = new RegExp(
  `${PROSE_LINK}point (?:to|at)\\b(?! (?:what|how|why|which|where|when|who|whether)\\b)`,
  "i",
);

/** A clause whose verb is a spoken act, so a manner phrase in it is how to say it. */
const SPOKEN_CLAUSE_START = new RegExp(
  `${CLAUSE_LEAD}(?:${FRONTED_PHRASE}){0,2}(?:say|greet|answer|reply|ask|repeat|offer)\\b`,
  "i",
);

/** Does one narrated prose span ask the listener to point at something, or to make a gesture as they speak? */
export function proseAsksToPointOrGesture(span: string): boolean {
  const text = withoutQuotations(span.replace(/\s+/g, " ").trim());
  if (PROSE_POINTING_STEP.test(text)) return true;
  return clausesOf(text).some(
    (clause) => SPOKEN_CLAUSE_START.test(clause) && GESTURE_MANNER.test(clause),
  );
}

/** Every narrated prose span of `markdown` that asks to point or gesture. */
export function pointingOrGestureProse(markdown: string): string[] {
  return narratedProseSpans(markdown).filter(proseAsksToPointOrGesture);
}

// ---------------------------------------------------------------------------
// Promoted into the classifier
// ---------------------------------------------------------------------------
//
// Every check in this file began life as a TEST: `drivable-writing-cues.test.ts`
// ran them over the lessons the modality manifest calls `drivable` and demanded
// zero. That left a hole the shape of the amendment in `modality.ts` that
// split a lesson into a core and its detachable sections.
//
//   drivable       modality === "voice"      what the tests scanned
//   coreDrivable   coreModality === "voice"  what the narration announces as
//                                            "you can do this one in the car"
//
// A lesson with a `## Writing — …` or `## The letters in this word` section is
// `sight` or `pen` in full and `voice` in its core. It was never `drivable`, so
// no test read it — and its narration still told the driver it could be done
// in the car, then read out its wrap-up recall:
//
//     [PAUSE 3s] Read **नमस्ते**. What does the conjunct do? …
//
// with no stop guard, because only the detachable sections get one. Measured
// when this was found, the checks as they then stood saw 158 such steps in the
// cores of 128 core-drivable lessons across 15 tracks. With the additions in
// this file ("Read these shuffled numerals", "Point to **ش · ک · ر**", "Read
// the question aloud from the page", a list item "- Read **নাম**.", a reading
// step after a quoted sentence) and with detachable sections left out, the
// count was 166 steps in 132 lessons: 98 reading steps (32 of them inside
// spoken recalls), 30 writing, 28 cover or uncover, 4 pointing, 6 gestures.
//
// The fix is not a wider test. A test that scans `coreDrivable` lessons would
// have needed to know which sections are detachable, which is a modality rule,
// and would still leave the CLASSIFIER claiming the lesson is drivable: every
// consumer of the manifest, the book's "Hands-free start" line, the chapter
// narration's drivable prefix, all would keep the wrong answer. So the checks
// moved here, into `src/`, and `modality.ts` asks {@link eyesOrHandsSteps} of
// every section. A section with a step a driver would hear unhedged is `sight`,
// with the reason `eyes-or-hands-step`; when that section is in the core, the
// core is `sight` too, and the lesson stops being announced as drivable.
//
// What this does NOT judge, deliberately:
//
//   - a deferred cue: `[YOU READ: **नमस्ते**]` is spoken as "once you have
//     stopped driving — read: नमस्ते". That is the authored way to keep a
//     reading step inside a drivable lesson, and it stays drivable.
//   - a heading: modality reads the body of each section, not its title. A
//     title names the section ("Practice — hear, say, read, and write water"
//     is a list of skills); the step itself is in the body. The NARRATION
//     still has to say that title to a driver, and a label spoken aloud
//     sounds exactly like an instruction, so the renderer reads headings with
//     their own check — see "Headings, spoken" at the end of this file.
//   - a scored activity: its prompt lives in an `<!-- hl-activity: … -->`
//     comment, which `narratedParts` drops with every other comment. The
//     narration speaks that prompt as a question, so it has its own check
//     too, and the renderer defers a prompt that needs eyes or hands exactly
//     as it defers a `[YOU WRITE: …]` cue — see "Scored activity prompts".
//   - a reading lesson, a writing lesson, a script section: those are already
//     `sight` or `pen` by type or block, before any sentence is read.

/** One narrated part of a section: what {@link eyesOrHandsSteps} judges it by. */
function partAsksForEyesOrHands(part: ReturnType<typeof splitNarrationCues>[number]): string | null {
  if ("text" in part) {
    const span = part.text.trim();
    if (span === "" || !mayAskForEyesOrHands(span)) return null;
    return proseAsksToWrite(span) ||
      proseAsksToReadOrHandleCards(span) ||
      proseAsksToPointOrGesture(span)
      ? span
      : null;
  }
  if (part.cue.kind !== "prompt") return null;
  // The cue's own "YOU SAY:" would pass the nested-cue test, so only what follows
  // its colon is screened (a superset of the parsed content, which follows it too).
  const source = part.cue.source;
  if (!mayAskForEyesOrHands(source.slice(source.indexOf(":") + 1))) return null;
  // A cue the narration defers is already hedged ("once you have stopped
  // driving — read: …"), whatever its content. Only spoken cues are judged.
  if (isManualCueAction(part.cue.action)) return null;
  const raw = parseDeliveryCue(part.cue.source.slice(1, -1));
  if (raw?.kind !== "prompt") return null;
  const recall = INSTRUCTION_CUE_ACTIONS.has(part.cue.action);
  const asks =
    (recall && (recallCueAsksForWriting(raw.content) || recallCueAsksToPoint(raw.content))) ||
    spokenCueAsksToReadScript(raw.content) ||
    spokenCueAsksForGesture(raw.content);
  return asks ? part.cue.source : null;
}

/**
 * Every word any check above can start from, as a substring.
 *
 * `eyesOrHandsSteps` runs on every section of every lesson each time modality
 * is derived (about thirty thousand lessons, several times over in a full test
 * run), and nine sections in ten contain none of these. Walking such a section
 * through the cue splitter and a dozen patterns costs about three times as much
 * as deriving the rest of the lesson's modality, so a section that cannot match
 * is answered by this one linear scan instead.
 *
 * It must be a SUPERSET of what the checks can see, never a subset: a word
 * missing here would silently make a check unreachable. Every check anchors
 * its verb at the START of a word (after a step link, a space, a bold marker
 * or the start of the text), so the vocabulary is matched as word-start
 * prefixes, case-insensitively — "pointing" is found by "point", "Hands" by
 * "hand", "uncover" by "(un)cover" — and NOT as bare substrings, which would
 * let "already" and "bread" through on "read" and filter almost nothing. It
 * lists the anchor of every pattern:
 *
 *   writing        (re)write draw copy circle underline sketch jot
 *   reading        read
 *   cards, cover   card (un)cover hid(e)
 *   pointing       point
 *   gestures       clap tap touch rais hold show gestur wav nod shak bow palm hand
 *
 * plus `YOU ` followed by a capital (case-sensitive, so not the pronoun), which
 * is where a nested cue such as "[YOU HEAR: *añcŭ*; YOU SHOW: 5]" starts. The
 * test beside this module holds the claim: on the corpus, the filtered and
 * unfiltered walks agree.
 */
const STEP_VOCABULARY =
  /\b(?:(?:re)?write|draw|copy|circle|underline|sketch|jot|read|card|(?:un)?cover|hid|point|clap|tap|touch|rais|hold|show|gestur|wav|nod|shak|bow|palm|hand)/i;
const NESTED_CUE_START = /YOU [A-Z]/;

/** Could any check in this file fire on `markdown`? False means certainly not. */
export function mayAskForEyesOrHands(markdown: string): boolean {
  return STEP_VOCABULARY.test(markdown) || NESTED_CUE_START.test(markdown);
}

/**
 * Every step in `markdown` that the narration would read to a driver plainly
 * and that needs eyes or hands: the union of every check in this file, judged
 * in one walk of the section. Returns the prose spans and cues as authored, so
 * a report can quote them.
 *
 * This is the rule behind the `eyes-or-hands-step` modality reason. It is the
 * same set of findings the separate checks return, in body order, each part
 * at most once: the corpus test in `tests/drivable-instructions.test.ts` holds
 * the two equal.
 */
export function eyesOrHandsSteps(markdown: string): string[] {
  const steps: string[] = [];
  if (!mayAskForEyesOrHands(markdown)) return steps;
  for (const part of narratedParts(markdown)) {
    const step = partAsksForEyesOrHands(part);
    if (step !== null) steps.push(step);
  }
  return steps;
}

// ---------------------------------------------------------------------------
// Scored activity prompts
// ---------------------------------------------------------------------------
//
// Everything above reads a section's BODY, as `narratedParts` sees it, and that
// walk drops every HTML comment — which is where a scored activity lives:
//
//     <!-- hl-activity: {"id":"ZH-C12-erzi-write","kind":"text",
//        "prompt":"Write son in Chinese characters.","answer":"儿子", …} -->
//
// The narration does not drop it. It compiles the contract and says the prompt
// as a question (`narration.ts`, `activityLines`):
//
//     [question — say your answer, then pause 12 seconds]
//     Write son in Chinese characters.
//
// which tells a driver to write, inside a lesson announced as one they can do in
// the car. When this was measured (issue #12070, the follow-up to the classifier
// change above), the drivable parts of core-drivable lessons spoke 3,103 activity
// prompts, and the checks in this file — run over the prompt text — fired on 222
// of them: 164 that open "Write …" (Marwadi 69, Japanese 45, Chinese 28, Marathi
// 9, Spanish 8 …), the rest reading unromanized script ("Read 汉字 without
// pinyin, then give its meaning"), a chained "then write", a picture cue.
//
// The activity schema is no help in telling them apart: `kind` is always `text`
// (`activity.ts` accepts nothing else), because the app shows every prompt
// beside a text box. So what the prompt ASKS is the only signal there is, and it
// is read with the same detectors as prose, plus a stricter check of its own
// (the prose detectors are tuned to under-fire, and a prompt cannot afford
// that), plus two shapes that only a prompt has, because only a prompt is
// displayed next to the script it is about:
//
//   shape                                     example from the corpus
//   ----------------------------------------  ---------------------------------------------
//   every check above, on the prompt text     Write son in Chinese characters.
//                                             Read 汉字 without pinyin, then give its meaning.
//   any hand verb as a whole word, outside    From bachchā, write the taught word.
//     a closed list of excused shapes (the    Write is that so, without pinyin.
//     fail-safe check: see the next section)
//   typing, with a script as the answer's     Type 'a car' in hiragana.
//     form (in hiragana / in characters /     Type the Mandarin greeting in characters.
//     the character for …)                    Type the Mandarin character for person.
//   reading, where a step starts, with a      Read this shuffled pair and give each meaning:
//     determiner or "and" for its object,       આજ — રાત.
//     in a prompt that holds unromanized      Read and distinguish 中文 and 中国 without pinyin.
//     script                                  In ਪੜ੍ਹਨਾ, identify the small ha … and then read
//                                               the whole known word aloud.
//
// The second shape is why "Type" is not simply a writing verb. 1,268 of those
// 3,103 prompts open "Type the Japanese for 'a stick'", "Type the Persian word
// for 'tea'" — and their answers are words, accepted in romanization, which a
// driver can SAY. Those are not deferred; the renderer says them for the ear
// ("Say the Japanese for 'a stick'", see `for-the-ear.ts`). Only a typing step
// whose answer is the script itself is a writing task in disguise.
//
// The third is narrower than the prose reading check on purpose. In a prompt the
// script is printed right there, so "Read this shuffled pair" can only mean read
// it off the screen — but "Read in the falling fourth tone, 好 stops meaning
// 'good'" is a participle ("in" is no object), and "Read se compran libros as if
// se still meant themselves" holds no unromanized script, so neither fires.
//
// What fires is deferred, not forbidden: the renderer speaks it as
// "[once you have stopped driving — question: …]", the JSON marks it
// `spoken: false` exactly as it marks a `[YOU WRITE: …]` cue, and the section
// that holds it is named in the notice and guarded like one. That is the same
// promise a deferred cue keeps, and it keeps the lesson's core drivable for the
// same reason: nothing is asked at the wheel.

/** "type" where a step starts: "Type the …", "…, then type the …". Never the noun ("its borrowing type", "type of"). */
const TYPING_STEP = new RegExp(`${PROSE_LINK}type(?= )(?! of\\b)`, "i");

/**
 * The answer's form is a script: "in hiragana", "in Chinese characters", "as
 * Devanagari signs", "in Urdu figures", "the character for …". Fixed words
 * after a fixed preposition and at most one capitalised word, so constant work
 * per position.
 */
const SCRIPT_FORM =
  /\b(?:in|as|using|into) (?:[A-Z][a-z]+ )?(?:hiragana|katakana|kana|kanji|characters?|script|signs|figures|digits|numerals)\b|\bcharacters? for\b/;

/** Does a prompt ask for its answer to be TYPED in a script? (A typed word is said; a typed script is written.) */
export function promptTypesInAScript(prompt: string): boolean {
  const text = prompt.replace(/\s+/g, " ").trim();
  return TYPING_STEP.test(text) && SCRIPT_FORM.test(text);
}

/** "read" where a step starts in a prompt, captured with the word after it. */
const PROMPT_READING_STEP = new RegExp(`${PROSE_LINK}read (\\S+)`, "gi");

/** Words that make what follows "read" its object in a prompt: "this", "the", "both", "and" (two objects to come). */
function opensAPromptObject(word: string): boolean {
  const bare = word.toLowerCase().replace(/[,.;:]$/, "");
  return PROSE_DETERMINERS.has(bare) || READ_PARTICLES.has(bare) || bare === "and";
}

/** Any letter of a script other than Latin, anywhere in the text. One linear scan. */
const NON_LATIN_LETTER = /(?=\p{L})\P{Script=Latin}/u;

/**
 * Does a prompt ask the learner to read the unromanized script it displays?
 * See the third row of the table above.
 *
 *   "Read this shuffled pair and give each meaning: આજ — રાત."   yes
 *   "Read and distinguish 中文 and 中国 without pinyin."          yes
 *   "Read in the falling fourth tone, 好 stops meaning 'good'."  no   ("in")
 *   "Read se compran libros as if se still meant themselves."   no   (no script)
 *   "How is the character 日 read inside 日本語?"                 no   (no step link)
 */
export function promptReadsTheShownScript(prompt: string): boolean {
  const text = withoutQuotations(prompt.replace(/\s+/g, " ").trim());
  if (!NON_LATIN_LETTER.test(text)) return false;
  for (const step of text.matchAll(PROMPT_READING_STEP)) {
    if (opensAPromptObject(step[1] ?? "")) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// A prompt that names a hand verb: the fail-safe check
// ---------------------------------------------------------------------------
//
// The prose checks above were built for lesson PROSE, and they are tuned to
// under-fire: in prose most writing verbs are mentions ("*fijar* — draw,
// harden, fasten", "**Write** is Old English", "when it opens, write your
// name" in a reading passage), and a gate that cries wolf is a gate nobody
// reads. So they count a verb only at a recognised step start: after a short
// list of opening verbs, or after a fronted phrase of plain ASCII words.
//
// A scored PROMPT is the opposite case. It is one short instruction shown above
// a text box, and a writing verb in it is a request. Reusing the prose tuning
// let these through, each spoken as a live question in a kept section of a
// core-drivable lesson (found by a security review of the first version):
//
//   prompt                                                   why the prose check missed it
//   -------------------------------------------------------  ----------------------------------------
//   From bachchā, write the taught word.                     "bachchā" is not plain ASCII, so the
//                                                              fronted phrase is not one
//   After hiding the model, write kāĩ and give its meaning.  "after" opens no fronted phrase
//   After all four checks pass, write home, person, …        (the same)
//   Write is that so, without pinyin.                        the copula guard read "Write" as the
//                                                              subject of "is"
//   Hear dhanyavad; give its meaning and write it in …       "give" is not a chain opener
//   From the heard morae i | chi | do, write once in …       "|" is not a fronted-phrase word
//
// About fifty of them: forty-one Marwadi dictations, five Chinese, two Marathi,
// one Japanese. The two possible mistakes do not cost the same. A question
// deferred that a driver could have answered costs one rehearsal; a question
// asked that needs a pen asks a driver to pick one up. So a prompt gets a check
// that fails SAFE: any hand verb, as a whole word, anywhere in the prompt,
// defers it — unless that occurrence is one of a small, closed list of shapes
// that cannot be a request:
//
//   shape                            example                                        why it is not a request
//   -------------------------------  ---------------------------------------------  -------------------------------
//   a quoted gloss: an opening       Say the Japanese for 'to write'.               the quote is the MEANING asked
//     quote, then "to" or a          Say the Urdu verb meaning 'to write'.            for; the verb is what it
//     pronoun, the verb, and the                                                      says, not what to do
//     closing quote within four
//     unpunctuated words; the quote
//     does not follow a colon, and
//     the closing quote ends a word
//   an infinitive gloss that ends    … before it meant to write?                    "means / meant / meaning to V"
//     its sentence                                                                    is a definition
//   a noun: a determiner right       … a letter-for-letter copy of tenuī …          an imperative never follows
//     before it, or "of" after it    What is the point of …                           "the" or "a", nor takes "of"
//   a third-person QUESTION: do /    Does it point to my or your?                   the subject is the WORD, not the
//     does / did / can … opening     Does یہ (yih) point at what is near you?         learner: "Do you write …?"
//     the sentence (or after a       What does one speaker write after midday?        and "Can I point …?" still fire
//     question word that does), a
//     subject of at most three
//     words, none of them you, I,
//     we or "please", and the
//     sentence ends in "?"
//   copy + a sound word, not a       Copy the rhythm of the line.                   imitation with the mouth: the
//     mark                                                                            guard WRITING_VERB carries
//
// A second security review tightened four of those excuses, each because a real
// request slipped through it:
//
//   request that was excused                                  what now refuses the excuse
//   --------------------------------------------------------  -------------------------------------------
//   Follow this: 'You trace the letter's outline …'           a quote after a colon is a quoted
//                                                               instruction, and the apostrophe in
//                                                               "letter's" is not a closing quote
//   Remember what it means to write it: do it in Devanagari.  "means to write" must end its sentence
//   Do it again and copy the sign in Devanagari.              the auxiliary must open the sentence,
//   If you can now write it in hiragana, do so.                 and the sentence must end in "?"
//   Copy the tone marks onto the vowels.                      a sound word followed by "mark(s)" is a
//                                                               written mark
//
// Everything else fires, including shapes the prose checks excuse on purpose: a
// verb after any fronted phrase, a verb followed by "is" ("Write is that so"
// asks for 是吗 to be written), a bare verb with no object, a verb in the middle
// of a list ("give its meaning and write it"), "Do you write …?". A bare quoted
// verb — "Say the word for 'write'" — fires too: an opening quote right before
// an imperative is also how a prompt quotes an instruction to carry out, and
// telling the two apart is the guessing this check exists to avoid.
//
// The verbs: writing (write, rewrite, jot, copy, trace, draw, sketch, record,
// note down, transcribe), marking a page or a screen (circle, underline, tick,
// check off, cross out, highlight, fill in, fill out, select), and the hand or
// the eye on its own (point, look at, tap, clap). Base forms only: a request is
// an imperative, and "written accents", "the pointing word" and "writing
// evidence" are descriptions. "record", "note down" and "transcribe" came with
// the second review: "Record the retrieval string: e, five, mouth, …" accepts
// only kana and kanji. ("note" alone is not here: "Note how the vowel
// lengthens" asks the learner to notice, which is done by ear.)
//
// Three verbs are deliberately NOT here:
//
//   - "type" and "read", whose shapes are the other two rows of the table in
//     the section above: 1,268 "Type the Japanese for …" prompts are said for
//     the ear, and "How is 日 read inside 日本語?" asks about a sound.
//   - "mark" (and "label"), for the reason the prose header gives: in prompts
//     they are nouns ("Name the mark that is written left of its letter",
//     "which mark each one takes") far more often than verbs, and the verb
//     shapes in the corpus are spoken tasks ("Label a big room, claim that your
//     house is big").
//
// Emphasis markers (`*`, `_`, backticks) are dropped first, so `*Write*`,
// `_write_` and `**Write**` are all the bare word.

/** The hand verbs a prompt may not name, as whole words. See the section header. */
const PROMPT_HAND_VERB =
  /(?<![\p{L}\p{M}\p{N}])(?:re-?write|write|jot|copy|trace|draw|sketch|record|note down|transcribe|circle|underline|tick|check off|cross out|highlight|fill in|fill out|select|point|look at|tap|clap)(?![\p{L}\p{M}\p{N}])/giu;

/**
 * How far either side of a verb the excuses look. Every excuse is a few words
 * long, so a fixed window keeps the whole scan linear in the prompt.
 */
const HAND_VERB_WINDOW = 60;

/**
 * An opening quote, then "to" or a subject pronoun, right before the verb: the
 * start of `'to write'`. Not after a colon: "Follow this: 'You trace …'" quotes
 * an instruction to carry out, not a meaning.
 */
const QUOTED_GLOSS_BEFORE = /(?:^|(?<!:) |[^\p{L}\p{N}\s:])['‘"“](?:to|i|you|we|they|he|she|it) $/iu;
/**
 * …and the closing quote within four more unpunctuated words: the end of `'to
 * write'`, `'I write letters'`. The closing quote must end a word, so the
 * apostrophe of "letter's" is not one.
 */
const QUOTED_GLOSS_AFTER = /^(?: [^\s'’"”,.;:!?—–]+){0,4}['’"”](?![\p{L}\p{M}\p{N}])/u;
/** "means to write", "meant to write", "meaning to write": a definition… */
const INFINITIVE_GLOSS_BEFORE = /\b(?:means?|meant|meaning) to $/i;
/** …that ends its sentence: "meant to write?", not "means to write it: do it …". */
const INFINITIVE_GLOSS_AFTER = /^['’"”)]*(?:[.?!]|$)/u;
/** A determiner right before the word makes it a noun: "the point", "a copy", "which circle". */
const NOUN_BEFORE =
  /\b(?:a|an|the|this|that|these|those|each|every|its|their|his|her|our|my|your|which|what|whose|no|another) $/i;
/** "copy of", "point of": a noun with its complement. */
const NOUN_AFTER = /^ of\b/i;
/**
 * A question whose subject is not the learner: "Does it point", "Does یہ (yih)
 * point", "What does one speaker write after midday?". The auxiliary OPENS the
 * sentence (the start of the prompt, or after a stop and a space), or follows a
 * question word that does ("What does", "Which mora does"), then at most three
 * subject words, none punctuated, none of them you, I, we or "please". The
 * caller also requires the sentence to end in "?": "Do it again and copy the
 * sign" opens with "Do" too, and is an imperative.
 *
 * `before` is the window before the verb; when the window was cut short, the
 * caller puts a NUL in front of it, so `^` is only the start of the prompt.
 */
const THIRD_PERSON_QUESTION_BEFORE =
  /(?:^|[.!?;:] )(?:(?:what|which|where|when|why|how|who|whom|whose)(?: [^\s.!?;:,]+)? )?(?:does|do|did|can|could|will|would|should) (?:(?!(?:you|i|we|please) )[^\s.!?;:,]+ ){1,3}$/iu;
/**
 * "copy the sound", "copy the rhythm": imitation with the mouth, as in
 * `WRITING_VERB` — but not "copy the tone marks", "copy the stress mark", "copy
 * the pitch accent marks", which are written.
 */
const COPY_BY_MOUTH_AFTER =
  /^ (?:the )?(?:sound|rhythm|melody|intonation|pronunciation|pitch|tone|stress)\b(?! (?:accents? )?marks?\b)/i;

/**
 * Is this occurrence of a hand verb one of the excused shapes? `before` and
 * `after` are the windows either side; `endsInQuestion` says whether the sentence
 * holding the verb ends in "?".
 */
function handVerbIsExcused(verb: string, before: string, after: string, endsInQuestion: boolean): boolean {
  if (QUOTED_GLOSS_BEFORE.test(before) && QUOTED_GLOSS_AFTER.test(after)) return true;
  if (INFINITIVE_GLOSS_BEFORE.test(before) && INFINITIVE_GLOSS_AFTER.test(after)) return true;
  if (NOUN_BEFORE.test(before) || NOUN_AFTER.test(after)) return true;
  if (endsInQuestion && THIRD_PERSON_QUESTION_BEFORE.test(before)) return true;
  return verb.toLowerCase() === "copy" && COPY_BY_MOUTH_AFTER.test(after);
}

/**
 * For each index of `text`, the index of the next sentence stop (".", "!" or
 * "?") at or after it, or `text.length` when there is none. One pass from the
 * end, so asking "does this verb's sentence end in '?'" costs O(1) per verb and
 * the whole scan stays linear however many verbs a prompt holds.
 */
function nextSentenceStops(text: string): Int32Array {
  const next = new Int32Array(text.length + 1);
  next[text.length] = text.length;
  for (let index = text.length - 1; index >= 0; index -= 1) {
    const char = text[index];
    next[index] = char === "." || char === "!" || char === "?" ? index : (next[index + 1] ?? text.length);
  }
  return next;
}

/**
 * Every hand verb a prompt names outside the excused shapes, lower-cased, in
 * order; empty when there is none. See the section header.
 *
 *   "From bachchā, write the taught word."                 ["write"]
 *   "Write is that so, without pinyin."                    ["write"]
 *   "Look at the sign, then tap each mora."                ["look at", "tap"]
 *   "Say the Japanese for 'to write'."                     []   (a quoted gloss)
 *   "Does it point to my or your?"                         []   (a third-person question)
 *   "… a letter-for-letter copy of tenuī …"                []   (a noun)
 *
 * Linear: one whole-text scan for the verbs (fixed alternatives behind a one-
 * character lookbehind), and each excuse reads a fixed-size window either side.
 */
export function handVerbsInPrompt(prompt: string): string[] {
  const text = prompt.replace(/[*_`]+/g, "").replace(/\s+/g, " ").trim();
  const verbs: string[] = [];
  let stops: Int32Array | null = null;
  for (const match of text.matchAll(PROMPT_HAND_VERB)) {
    const at = match.index;
    const end = at + match[0].length;
    // A window cut short starts with NUL, so no excuse mistakes the cut for the
    // start of the prompt (see THIRD_PERSON_QUESTION_BEFORE).
    const cut = at > HAND_VERB_WINDOW ? "\u0000" : "";
    const before = cut + text.slice(Math.max(0, at - HAND_VERB_WINDOW), at);
    const after = text.slice(end, end + HAND_VERB_WINDOW);
    stops ??= nextSentenceStops(text);
    const endsInQuestion = text[stops[end] ?? text.length] === "?";
    if (!handVerbIsExcused(match[0], before, after, endsInQuestion)) verbs.push(match[0].toLowerCase());
  }
  return verbs;
}

// ---------------------------------------------------------------------------
// A prompt whose ANSWER is written: the form it asks for, and what it accepts
// ---------------------------------------------------------------------------
//
// The checks above look for a verb. A second security review found questions
// that name no hand verb at all and still cannot be answered aloud, because the
// FORM of the answer is writing — each one spoken as a live question in a kept
// section of a core-drivable lesson:
//
//   prompt                                                        what it accepts
//   ------------------------------------------------------------  ------------------------------
//   Refuse an offer and then leave, in two written lines.         कोनी। पाछे मिलसू। …
//   Refuse the offer, then leave the door open — two written      (the same)
//     lines.
//   Someone says das karo. Refuse in one word, in Devanagari.     कोनी, … (assesses a SCRIPT atom)
//   Record the retrieval string: e, five, mouth, please say it,   え・五・口・… (kana and kanji only)
//     hair, eye.
//   Which already-taught Gurmukhi consonant is na: ਨ or ਸ?        ਨ, "ਨ na", "na is ਨ"
//   Which taught character is the Level 1 word female or woman?   女, "女 nǚ"
//
// So there are three more prompt shapes, and two rules that read the rest of the
// activity rather than its prompt:
//
//   shape or rule                              example
//   -----------------------------------------  -----------------------------------------------
//   a written answer form: "in writing", "on   … in two written lines.
//     paper", "written line(s) / form /
//     answer(s) / sentence(s) / response(s)
//     / reply / version"
//   a script as the answer's form: in / into   Refuse in one word, in Devanagari.
//     / using / as, an optional "the" and one  … in Chinese characters, in kana, into hiragana
//     more word, then a script's name
//   a choice between two unromanized runs of   … is na: ਨ or ਸ?
//     script ("X or Y", "X / Y", "X vs Y")
//   every accepted answer holds a letter of    Which taught character is … woman?  (女, 女 nǚ)
//     a script that is not Latin: no answer
//     a driver can SAY would be accepted
//   it assesses a `-SCRIPT-` atom AND its      Someone says das karo. Refuse in one word …
//     canonical answer is in script: the         (assesses MW-SCRIPT-KONI-01; answer कोनी)
//     item scores producing or recognising
//     the glyphs
//
// The script-name check fires on a spoken FACT too — "In Devanagari, ক is …" —
// and that is deliberate: the costs are the ones the hand-verb check weighs, and a
// prompt about how a script spells a word is one a driver can do without.
//
// The `-SCRIPT-` rule stops short of every script atom on purpose. Thirty-five
// spoken questions assessed one when it was added, and most of them are knowledge
// ABOUT a script asked and answered by ear: "Say the four velar letters in order"
// (ka kha ga gha), "In the cluster rrha, where is the h written relative to the
// letter before it?" (underneath), "How many beats does いいえ (iie) hold?"
// (three). Their canonical answers are words, so a driver can give them; deferring
// them would empty the letter-recall drills of every Marathi review lesson for no
// safety gain. When the canonical answer — the first accepted response, the one
// the app displays — is itself script, the item is grading the glyphs, and it is
// deferred even when a romanized variant is also accepted ("koni" for कोनी): the
// prompt asked for Devanagari.

/** "in writing", "two written lines", "the written form": the answer is to be written. */
const WRITTEN_ANSWER =
  /(?<![\p{L}\p{M}])(?:in writing|on paper|written (?:lines?|form|answers?|sentences?|responses?|reply|version))(?![\p{L}\p{M}])/iu;

/**
 * The name of a script, or of a script's units, as the form of an answer. A
 * language name alone is not one ("in Gujarati" is a language); a language name
 * followed by script / letters / characters / signs is.
 */
const SCRIPT_NAME = String.raw`(?:devanagari|gurmukhi|shahmukhi|nastaliq|naskh|perso-arabic|cyrillic|hangul|kana|hiragana|katakana|kanji|hanzi|characters|script)`;

/**
 * "in Devanagari", "in Chinese characters", "into hiragana", "using the Arabic
 * script", "in Gujarati script": a preposition, an optional "the", at most one
 * more word, then a script's name. Whole words; the one free word is a run of
 * letters, so each position does bounded work beyond that run.
 */
const SCRIPT_ANSWER_FORM = new RegExp(
  String.raw`(?<![\p{L}\p{M}])(?:in|into|using|as) (?:the )?(?:[\p{L}\p{M}-]+ )?${SCRIPT_NAME}(?![\p{L}\p{M}])`,
  "iu",
);

/** Does a prompt ask for its answer in a written form or in a script? See the table above. */
export function promptAsksForAWrittenAnswer(prompt: string): boolean {
  const text = prompt.replace(/[*_`]+/g, "").replace(/\s+/g, " ").trim();
  return WRITTEN_ANSWER.test(text) || SCRIPT_ANSWER_FORM.test(text);
}

/**
 * One letter (with its marks) of a script that is neither Latin nor shared:
 * Common covers digits, punctuation and the Japanese length bar, Inherited the
 * combining marks, and neither is a script anyone has to read.
 */
const SCRIPT_RUN_CHAR = String.raw`(?![\p{Script=Latin}\p{Script=Common}\p{Script=Inherited}])\p{L}\p{M}*`;

/**
 * A choice between two runs of unromanized script: "ਨ or ਸ", "中文 / 中国",
 * "আজ vs রাত". A romanization in brackets after the first run ("ਨ (na) or ਸ")
 * breaks the pattern, because then the choice can be heard. The first run starts
 * where no letter or mark precedes it, so each run is tried from its start only.
 * That lookbehind is ONE character wide on purpose: an earlier `(?<!\p{L}\p{M}*)`
 * meant the same thing for real text but made the engine walk back over a whole
 * run of combining marks at every start position — quadratic, 53 s on 40,000
 * bare accents. A mark is only ever preceded by its letter or another mark, so
 * "no letter and no mark immediately before" is the same boundary.
 */
const GLYPH_CHOICE = new RegExp(
  String.raw`(?<![\p{L}\p{M}])(?:${SCRIPT_RUN_CHAR})+['’"”]?(?:,? or | ?\/ ?| vs\.? | versus )['‘"“]?(?:${SCRIPT_RUN_CHAR})+`,
  "u",
);

/** Does a prompt offer a choice between shown glyphs that only an eye can tell apart? */
export function promptChoosesAmongShownGlyphs(prompt: string): boolean {
  return GLYPH_CHOICE.test(prompt.replace(/[*_`]+/g, "").replace(/\s+/g, " "));
}

/** A letter of a script that is not Latin (nor Common or Inherited): see `SCRIPT_RUN_CHAR`. */
const SCRIPT_LETTER = /(?![\p{Script=Latin}\p{Script=Common}\p{Script=Inherited}])\p{L}/u;

/** An atom that assesses a script: `MW-SCRIPT-KONI-01`, `JA-SCRIPT-E-01`. */
const SCRIPT_ATOM = /-SCRIPT-/;

/** What the structural rules read of a scored activity: its prompt, its atoms, its accepted answers. */
export interface ScoredActivityShape {
  prompt: string;
  assesses: readonly string[];
  /** The canonical answer first, then every authored variant (as `CompiledLessonActivity` holds them). */
  acceptedResponses: readonly string[];
}

/**
 * Is every accepted answer written in a script? True only when there is at least
 * one, and each holds a letter of a non-Latin script: then nothing a driver can
 * SAY is accepted. "女" and "女 nǚ" both hold 女; "koni" holds none.
 */
export function acceptsOnlyScript(acceptedResponses: readonly string[]): boolean {
  return acceptedResponses.length > 0 && acceptedResponses.every((response) => SCRIPT_LETTER.test(response));
}

/** Does the activity assess a script atom with a canonical answer that is itself script? See the header. */
export function gradesScriptGlyphs(activity: Pick<ScoredActivityShape, "assesses" | "acceptedResponses">): boolean {
  const canonical = activity.acceptedResponses[0] ?? "";
  return activity.assesses.some((atom) => SCRIPT_ATOM.test(atom)) && SCRIPT_LETTER.test(canonical);
}

/** Why a scored activity's prompt cannot be asked at the wheel; one code per shape. */
export type ActivityEyesOrHandsReason =
  | "eyes-or-hands-step"
  | "hand-verb"
  | "typing-in-a-script"
  | "reading-the-shown-script"
  | "written-answer"
  | "choice-among-shown-glyphs"
  | "accepts-only-script"
  | "grades-script-glyphs";

/**
 * Every reason a scored activity's prompt needs eyes or hands — empty when a
 * driver can answer it aloud. This is the narration's test for deferring the
 * question; see the two section headers above.
 *
 *   "Write son in Chinese characters."               ["eyes-or-hands-step", "hand-verb"]
 *   "From bachchā, write the taught word."           ["hand-verb"]
 *   "Type 'a car' in hiragana."                      ["typing-in-a-script"]
 *   "Read this shuffled pair …: આજ — રાત."           ["reading-the-shown-script"]
 *   "Type the Japanese for 'a stick, a pole'."       []   (said for the ear instead)
 *   "Which one points at a day we both know?"        []
 *
 * Each check reads the prompt as authored AND with its emphasis markers dropped,
 * and either firing counts: `*Type* 'a car' in hiragana` and `Read **this** pair
 * …` are the same requests as their plain spellings, and reading both can only
 * defer more, never less.
 */
export function activityPromptNeedsEyesOrHands(prompt: string): ActivityEyesOrHandsReason[] {
  const plain = prompt.replace(/[*_`]+/g, "");
  const readings = plain === prompt ? [prompt] : [prompt, plain];
  const either = (check: (text: string) => boolean): boolean => readings.some(check);
  const reasons: ActivityEyesOrHandsReason[] = [];
  if (either((text) => eyesOrHandsSteps(text).length > 0)) reasons.push("eyes-or-hands-step");
  if (handVerbsInPrompt(prompt).length > 0) reasons.push("hand-verb");
  if (either(promptTypesInAScript)) reasons.push("typing-in-a-script");
  if (either(promptReadsTheShownScript)) reasons.push("reading-the-shown-script");
  if (promptAsksForAWrittenAnswer(prompt)) reasons.push("written-answer");
  if (promptChoosesAmongShownGlyphs(prompt)) reasons.push("choice-among-shown-glyphs");
  return reasons;
}

/**
 * Every reason a scored ACTIVITY cannot be asked at the wheel: the prompt's
 * reasons ({@link activityPromptNeedsEyesOrHands}) and the two structural rules
 * that read what it assesses and what it accepts (see "A prompt whose ANSWER is
 * written"). This is what the narration defers on; the prompt-only check remains
 * for a caller that holds nothing but the prompt.
 *
 *   { prompt: "Which taught character is … woman?", accepted 女, 女 nǚ }     ["accepts-only-script"]
 *   { prompt: "Refuse in one word, in Devanagari.", assesses MW-SCRIPT-… }  ["written-answer", "grades-script-glyphs"]
 *   { prompt: "Say the four velar letters in order.", assesses MR-SCRIPT-…,
 *     answer "ka kha ga gha" }                                               []
 */
export function activityNeedsEyesOrHands(activity: ScoredActivityShape): ActivityEyesOrHandsReason[] {
  const reasons = activityPromptNeedsEyesOrHands(activity.prompt);
  if (acceptsOnlyScript(activity.acceptedResponses)) reasons.push("accepts-only-script");
  if (gradesScriptGlyphs(activity)) reasons.push("grades-script-glyphs");
  return reasons;
}

/** Does `text` still ask for typing where a step starts? What a spoken prompt must never do. */
export function asksToType(text: string): boolean {
  return TYPING_STEP.test(text.replace(/\s+/g, " ").trim());
}

// ---------------------------------------------------------------------------
// Headings, spoken
// ---------------------------------------------------------------------------
//
// The classifier does not read headings (see "Promoted into the classifier"): a
// title names its section, and the step it introduces is in the body, where it
// is deferred, said for the ear, or counted. That is right for deciding what a
// lesson needs. It is not enough for what the narration SAYS, because the
// narration speaks every title, and a spoken label is indistinguishable from an
// instruction:
//
//     Guided Practice — read, copy, recall.
//     [once you have stopped driving — read: に]
//
// The body defers the reading; the title, one line earlier, has already asked
// for it. Measured over the drivable parts of core-drivable lessons when this was
// added: 103 titles opened a step with an eyes-or-hands verb — 31 Japanese
// "Guided Practice — read, copy, recall", 38 Spanish lesson titles of the shape
// "(el mensaje del vecino) — read it, then answer", "Guided Practice — write",
// "Guided Practice: Read them back", "Sounds you'll need: Read the cedilla" —
// and the prose checks above found only two of them, because they were built
// for sentences with objects, not for labels.
//
// So a heading gets a check of its own, deliberately simpler and stricter than
// the prose one. A heading has no room for the shapes the prose checks excuse
// ("Read it literally and it says …" is interpretation; a heading never
// interprets), so an eyes-or-hands verb at a step start counts with or without
// an object. The step starts are the heading's:
//
//   link            example
//   --------------  ---------------------------------------------------------
//   (start)         Read two, say one
//   ": "            Sounds you'll need: Read the cedilla
//   " — " " – "     Guided Practice — write
//   ", " ", and "   Practice — hear, say, read, and write water
//   " then " …      Practice — sort the evening words, then read the Latin
//
// The em dash IS a link here, unlike in prose, where it opens a gloss
// ("*Cubre la olla* — cover the pot"): in a section title it separates the
// label from what the section does. A lesson title is the exception, and the
// renderer handles it (`for-the-ear.ts`): "ಬರೆ (bare) — write" is the headword
// and its gloss, and the gloss is the word's meaning, not a request.
//
// The controls:
//
//   You'll want to know: tapēte, tapētis      a Latin word that starts "tap"
//   Guided Practice: point every verb at …    "point" with an object other than
//                                             to/at/and is not a gesture (a bare
//                                             "point" ending a step still is)
//   **Write** is Old English *wrītan*         the verb is a subject ("is" follows)
//   Hear *kànshū* — to read a book            "to " is not a step link
//
// One shape the prose checks excuse is NOT excused here: "read" followed by an
// italic pronunciation. "Grammar Lens — the sign は, read *wa*" means "は is
// pronounced wa" on the page, but the italics are gone once it is spoken, and
// "the sign は, read wa" is heard as an instruction. So it fires, and the
// narration says "Grammar Lens"; the body, which explains the reading, follows.
//
// What the renderer does with a match is in `for-the-ear.ts`: it keeps the
// label ("Guided Practice") and drops the part that asks. A section the notice
// sets aside keeps its whole title, because the stop guard is now spoken BEFORE
// it.

/**
 * Where a step starts in a heading. See the table above. Any emphasis opener may
 * sit between the link and the verb — `**Write**`, `*Read*`, `_copy_` — because
 * the narration strips all of them, and an italic verb is heard exactly as a bold
 * one is.
 */
const HEADING_LINK = String.raw`(?:^|: | [—–] |, (?:and |or )?| then | and | or )(?:then |now |also |first )?(?:\*{1,2}|_{1,2})?`;

/**
 * The eyes-or-hands verbs a heading can open a step with. `copy` keeps the
 * writing check's guard for imitating a sound, marks included ("copy the tone
 * marks" still counts).
 *
 * `point` counts with a gesture object ("point to", "point at", "point and …")
 * or BARE, as the last word of a step: the end of the heading or a list item
 * ("Retrieval 6 — greet, repair, point", "point, then say"). A security review
 * found that title spoken as a lesson's opening line, because only the gesture
 * objects were matched. "point every verb at somebody" is a grammar drill, so a
 * `point` followed by any other word still does not count.
 */
const HEADING_VERB = String.raw`(?:re-?read|read|rewrite|write|type|draw|copy(?! (?:the )?(?:sound|rhythm|melody|intonation|pronunciation|pitch|tone|stress)\b(?! (?:accents? )?marks?\b))|trace|circle|underline|uncover|cover|hide|tap|clap|point (?:to|at|and)|point(?=[*_]{0,2}(?:[,.;:!?]|$)))`;

/**
 * The verb must end the word (a Unicode-aware boundary, so "tapēte" is not
 * "tap"), and must not be followed by a copula (the verb as a subject: "Writing
 * is …" never matches anyway, "**Write** is Old English" would).
 */
const HEADING_STEP = new RegExp(
  `${HEADING_LINK}${HEADING_VERB}(?![\\p{L}\\p{M}'’-])(?![*_]{0,2} (?:is|was|means)\\b)`,
  "iu",
);

/**
 * Where the first eyes-or-hands step in a heading starts — the index of its
 * link (", ", " — ", the start …) — or -1 when the heading asks for none.
 *
 *   "Guided Practice — read, copy, recall"   15   (the " — ")
 *   "Read two, say one"                      0
 *   "Guided Practice: point every verb at"   -1
 *
 * Whitespace runs are collapsed first (one linear pass), so every gap is one
 * literal space; the pattern is a fixed-literal link, optional fixed literals
 * and a choice of fixed verbs, so each start position does bounded work.
 */
export function headingStepIndex(heading: string): number {
  const match = HEADING_STEP.exec(heading.replace(/\s+/g, " "));
  return match === null ? -1 : match.index;
}
