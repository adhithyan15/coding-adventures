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
// One kind of cue is not safe either. `[YOU RECALL: write **ば** — **R1**]` is
// a cue, but RECALL is a spoken action, so the narration reads it to a driver
// as "recall: write ば". The last section of this module, "Inside a recall
// cue", reads those, and `drivableWritingInstructions` runs both. The two
// sections after it read cues for the other things a hand or an eye does on
// the page: pointing at a sign (inside a recall), and reading printed script
// (inside any cue the narration speaks unhedged).
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

import { parseDeliveryCue } from "../src/delivery-cue.js";
import { isManualCueAction, splitNarrationCues } from "../src/narration.js";

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
// once per cue (the nested one as one global pass), so a text of N characters
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
