// Delivery cues — the one grammar both readers of a lesson agree on.
//
// HL00 asks lessons to be written as audio scripts, so the Markdown carries
// bracketed stage directions:
//
//     [PAUSE 2s]                  hold here
//     [PAUSE 1s each]             hold after every item of the list that follows
//     [REPEAT x2]                 say that again
//     [YOU SAY: hola]             the learner's turn
//     [YOU SAY (m.): boltā hūṁ]   the learner's turn, qualified
//     [YOU RUN the pattern: …]    the learner's turn, with a lower-case object
//
// Two views read those brackets:
//
//     lesson .md ──┬── narration.ts ── keeps each cue as a directive for the voice
//                  └── book.ts ─────── turns each cue into printed English, or ink-free
//
// They used to carry two grammars. The narration parser scanned brackets by depth
// and saw a cue wherever it sat in a paragraph; the book matched regular
// expressions one source line at a time. So a cue the author's editor had
// wrapped onto a second line —
//
//     Cover the word and write it from memory. [YOU WRITE: the word, then its
//     meaning]
//
// — was a perfectly good cue to the narrator and two halves of nothing to the
// book, which escaped it into `{[}YOU WRITE: …{]}` ON THE PRINTED PAGE. The same
// split let `[YOU SAY (m.): …]`, a pause in the middle of a sentence, and a
// second cue later in the same bullet all reach the PDF as raw brackets.
//
// This module is the shared half: where a bracket closes, and what (if
// anything) the text inside it means. Each view keeps its own *rendering*,
// because "what does a pause look like" has a different right answer for an ear
// and for a page. Neither view may decide on its own what *counts* as a cue.
//
// Everything here is a hand-written index scan. No regular expression is ever run
// across a whole paragraph, so no input can make the cost super-linear.

// ---------------------------------------------------------------------------
// Where does this bracket close?
// ---------------------------------------------------------------------------

/**
 * How far ahead {@link closingBracket} will look for a cue's closing bracket.
 *
 * 4,096 characters, against a corpus whose longest authored cue is a couple of
 * hundred. It exists to bound the scan, not to reject anything real. Without a
 * bound, a paragraph of nothing but `[[[[…` would cost O(n²): every `[` would scan
 * to the end of the text and fail. With it, each `[` costs at most a fixed number
 * of steps and the whole walk stays linear.
 */
export const MAX_CUE_LENGTH = 4096;

/**
 * The index of the `]` that closes the `[` at `open`, or -1.
 *
 * The scan tracks depth because cues really do nest brackets inside them —
 * `[YOU SAY: "mā ismuka?" — "what [is] your name?"]` — and stopping at the first
 * `]` would cut the cue in half. A backslash escapes the next character, so an
 * authored `\]` is never mistaken for a close.
 *
 * Newlines are ordinary characters here. That is the whole point of this file:
 * whether a cue may span lines is the CALLER's decision (the book only hands in
 * text that Markdown would join into one paragraph or one list item), not an
 * accident of matching one line at a time.
 */
export function closingBracket(text: string, open: number): number {
  let depth = 0;
  const limit = Math.min(text.length, open + MAX_CUE_LENGTH);
  let cursor = open;
  while (cursor < limit) {
    const scanned = text[cursor];
    if (scanned === "\\") {
      cursor += 2;
      continue;
    }
    if (scanned === "[") depth += 1;
    else if (scanned === "]") {
      depth -= 1;
      if (depth === 0) return cursor;
    }
    cursor += 1;
  }
  return -1;
}

/**
 * True when the `[` at `open` starts with a cue keyword: `YOU`, `PAUSE` or
 * `REPEAT`, followed by a blank (a space, a tab, or the newline of a wrapped cue).
 *
 * A constant-time filter to run BEFORE {@link closingBracket}. Most brackets in
 * a lesson are glosses and links, and without the filter each one would pay for
 * a bracket scan; with it, a line of a hundred thousand `[` costs a hundred
 * thousand steps rather than four hundred million.
 */
export function opensDeliveryCue(text: string, open: number): boolean {
  for (const keyword of CUE_KEYWORDS) {
    if (text.startsWith(keyword, open + 1) && isBlank(text[open + 1 + keyword.length])) {
      return true;
    }
  }
  return false;
}

const CUE_KEYWORDS = ["YOU", "PAUSE", "REPEAT"] as const;

// ---------------------------------------------------------------------------
// What does the text inside mean?
// ---------------------------------------------------------------------------

/** `[PAUSE 2s]`, `[PAUSE 1s each]`. */
export interface PauseCue {
  kind: "pause";
  seconds: number;
  /** `each`: the silence repeats after every item of the list it introduces. */
  perItem: boolean;
}

/** `[REPEAT x2]`. */
export interface RepeatCue {
  kind: "repeat";
  times: number;
}

/**
 * `[YOU <ACTION><qualifier>: <content>]`.
 *
 *     [YOU SAY: hola]                       action SAY,         qualifier ""
 *     [YOU SAY (m.): boltā hūṁ]             action SAY,         qualifier "(m.)"
 *     [YOU RUN the pattern: …]              action RUN,         qualifier "the pattern"
 *     [YOU READ ALOUD, gathering …: …]      action READ ALOUD,  qualifier ", gathering …"
 *     [YOU CHOOSE BY CONTEXT: …]            action CHOOSE BY CONTEXT
 *
 * The action is the run of upper-case words: that is what makes it a cue verb
 * rather than prose that happens to open with "YOU". The qualifier is whatever
 * the author wrote between the verb and the colon. It used to make the whole
 * bracket unrecognisable to BOTH views — 18 such cues reached the narration
 * script verbatim and 16 reached the printed books — although it is plainly part
 * of the instruction: "say it (masculine form)".
 */
export interface PromptCue {
  kind: "prompt";
  /** The upper-case cue verb, single-spaced: `SAY`, `READ ALOUD`, `CHOOSE BY CONTEXT`. */
  action: string;
  /**
   * The author's words between the verb and the colon, whitespace-collapsed and
   * trimmed: `(m.)`, `the pattern`, `, gathering the words one by one`. A leading
   * comma is kept, because it is how the author joined it to the verb — use
   * {@link joinQualifier} to put the two back together.
   */
  qualifier: string;
  /** Everything after the colon, whitespace-collapsed and trimmed. Still Markdown. */
  content: string;
}

export type DeliveryCue = PauseCue | RepeatCue | PromptCue;

function isDigit(character: string | undefined): boolean {
  return character !== undefined && character >= "0" && character <= "9";
}

function isUpper(character: string | undefined): boolean {
  return character !== undefined && character >= "A" && character <= "Z";
}

function isAsciiLetter(character: string | undefined): boolean {
  return isUpper(character) || (character !== undefined && character >= "a" && character <= "z");
}

function isBlank(character: string | undefined): boolean {
  return character === " " || character === "\t" || character === "\n" || character === "\r";
}

/**
 * Read a decimal number starting at `index`; null when there is none.
 *
 * `[PAUSE 1.5s]` is not in the corpus today but costs two lines to support and
 * would otherwise silently become literal prose.
 */
export function readNumber(text: string, index: number): { value: number; next: number } | null {
  let cursor = index;
  while (isDigit(text[cursor])) cursor += 1;
  if (cursor > index && text[cursor] === "." && isDigit(text[cursor + 1])) {
    cursor += 1;
    while (isDigit(text[cursor])) cursor += 1;
  }
  if (cursor === index) return null;
  return { value: Number(text.slice(index, cursor)), next: cursor };
}

/**
 * Join wrapped source lines the way Markdown does: every line trimmed, one space
 * between them. Inner runs of spaces on a single line are left alone — an author
 * who wrote `boltā  ·  boltī` meant those spaces, and renderMarkdown keeps them.
 *
 * Split-and-join rather than a `\s*\n\s*` replace: that pattern backtracks over a
 * long run of spaces with no newline in it, which is exactly the shape CodeQL's
 * polynomial-ReDoS query is written to find.
 */
export function joinWrappedLines(text: string): string {
  if (!text.includes("\n")) return text.trim();
  return text
    .split("\n")
    .map((line) => line.trim())
    .join(" ")
    .trim();
}

/**
 * Turn the inside of one `[…]` into a cue, or return null if it is not one.
 *
 * Returning null matters as much as returning a cue: lessons are full of ordinary
 * brackets — Markdown links, glosses like `[I am your friend]`, `[bonjour]` — and
 * reading one of those as a directive would delete real teaching content.
 */
export function parseDeliveryCue(inner: string): DeliveryCue | null {
  const text = joinWrappedLines(inner);

  const pause = afterKeyword(text, "PAUSE");
  if (pause !== -1) {
    const number = readNumber(text, pause);
    if (!number) return null;
    let cursor = number.next;
    if (text[cursor] === "s") cursor += 1;
    const rest = text.slice(cursor).trim().toLowerCase();
    if (rest !== "" && rest !== "each") return null;
    return { kind: "pause", seconds: number.value, perItem: rest === "each" };
  }

  const repeat = afterKeyword(text, "REPEAT");
  if (repeat !== -1) {
    let cursor = repeat;
    if (text[cursor] === "x" || text[cursor] === "X") cursor += 1;
    const number = readNumber(text, cursor);
    if (!number || text.slice(number.next).trim() !== "") return null;
    return { kind: "repeat", times: number.value };
  }

  const prompt = afterKeyword(text, "YOU");
  if (prompt !== -1) return parsePrompt(text, prompt);

  return null;
}

/**
 * Where the words after a cue keyword start, or -1 when `text` does not open
 * with that keyword followed by a blank.
 *
 * ANY blank: a space, a tab, or (before `joinWrappedLines` turned it into a
 * space) a newline. This is the same rule {@link opensDeliveryCue} applies and
 * the same rule the book's printed-cue gate applies. When they disagreed ---
 * this parser wanted exactly one space --- `[YOU\tSAY: hi]` was a cue to the
 * keyword filter, not a cue to the parser, and not a cue to the gate either,
 * so it reached the page as raw brackets with every check green.
 */
function afterKeyword(text: string, keyword: string): number {
  if (!text.startsWith(keyword) || !isBlank(text[keyword.length])) return -1;
  let cursor = keyword.length;
  while (isBlank(text[cursor])) cursor += 1;
  return cursor;
}

/**
 * `YOU <ACTION><qualifier>: <content>` — the scan behind {@link PromptCue}.
 *
 *     Y O U ␠ S A Y ␠ ( m . ) : ␠ b o l t ā …
 *             └─┬─┘ └──┬───┘ │ └────┬─────
 *            action qualifier │   content
 *                          first colon
 *
 * An action word is a maximal run of A–Z that is not glued to a lower-case
 * letter (`SAYx` is not a verb). Words keep joining the action while they are
 * separated by single blanks; the first word that is not all capitals ends it.
 */
function parsePrompt(text: string, start: number): PromptCue | null {
  const colon = text.indexOf(":", start);
  if (colon === -1) return null;
  const head = text.slice(start, colon);
  const words: string[] = [];
  let cursor = 0;
  let actionEnd = 0;
  while (cursor < head.length) {
    let end = cursor;
    while (isUpper(head[end])) end += 1;
    if (end === cursor || isAsciiLetter(head[end])) break;
    words.push(head.slice(cursor, end));
    actionEnd = end;
    // Step over the blanks to the next candidate word.
    let next = end;
    while (isBlank(head[next])) next += 1;
    if (next === end) break;
    cursor = next;
  }
  if (words.length === 0) return null;
  const qualifier = head.slice(actionEnd).trim();
  // The qualifier must hang off the verb with a blank or a comma. `[YOU SAY-IT: …]`
  // is not a cue anyone wrote, and guessing what it means would print nonsense.
  const separator = head[actionEnd];
  if (qualifier !== "" && !isBlank(separator) && separator !== ",") return null;
  return {
    kind: "prompt",
    action: words.join(" "),
    qualifier,
    content: text.slice(colon + 1).trim(),
  };
}

/**
 * Put a printed or spoken verb back together with its qualifier.
 *
 *     joinQualifier("Say it", "(m.)")             → "Say it (m.)"
 *     joinQualifier("Read aloud", ", gathering")  → "Read aloud, gathering"
 *     joinQualifier("Say it", "")                 → "Say it"
 */
export function joinQualifier(verb: string, qualifier: string): string {
  if (qualifier === "") return verb;
  return qualifier.startsWith(",") ? `${verb}${qualifier}` : `${verb} ${qualifier}`;
}
