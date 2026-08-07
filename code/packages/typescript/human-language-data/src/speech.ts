// Speech — turning learner-facing Markdown into something a mouth can say.
//
// This module answers one narrow question, asked twice by two different callers:
// **can this piece of the page be said out loud, and if so, what are the words?**
//
//   * `modality.ts` asks it to decide whether a lesson is drivable at all.
//   * `narration.ts` asks it to actually produce the script.
//
// Both need the same answer, and an answer that differed between them would be the
// worst possible bug in this feature: a lesson stamped 🚗 "safe in the car" whose
// narration then quietly skipped the table it could not read. So the judgement lives
// here, once, and both callers import it.
//
// ---------------------------------------------------------------------------
// Why tables are the whole problem
// ---------------------------------------------------------------------------
//
// HL08 measured it: of the 1,038 lessons that need neither a pen nor a script block,
// **322 contain a Markdown table** and that single feature is the biggest thing
// standing between the corpus and a hands-free course. Not the script. The table.
//
// And a table is not one thing. Two shapes, both real, from this corpus:
//
//   | Telugu                        | English            |
//   |---|---|
//   | నా పేరు మీరా. (*nā pēru Mira*)  | My name is Mira.   |
//
// That reads aloud perfectly: *"nā pēru Mira means: My name is Mira."* Two columns,
// one fact per row, no cross-referencing. A voice can carry it.
//
//   |   | numeral | word    | said     |
//   |---|---|---|---|
//   | 1 | ౧      | ఒకటి    | *okaṭi*  |
//
// That one cannot be said. Four columns, and the first has no header at all — its
// meaning comes from being *the leftmost column*, which is a fact about geometry,
// not about sound. Reading it aloud would either lie or lose something.
//
// So the rule is a width, and the width is a policy knob (`maxLinearisableTableColumns`
// in `core/chapter-policy.json`), not a constant buried in code.
//
// ---------------------------------------------------------------------------
// The safety rule that shapes every function below
// ---------------------------------------------------------------------------
//
// **A table this module refuses to linearise must make its lesson `sight`.** It is
// never dropped, never summarised away, never silently skipped. The learner may end
// up told "there is a five-column table here you will need to look at later" — that
// is fine. What is not fine is a learner who finishes a lesson unaware that a third
// of it never reached them. Every refusal below therefore carries a *reason* and the
// table's *headers and row count*, so even the refusal can be spoken.
//
// ---------------------------------------------------------------------------
// Regular expressions are avoided on purpose
// ---------------------------------------------------------------------------
//
// Every scan in this file is a hand-written left-to-right pass over the characters.
// That is not stylistic. These functions run over every block of all 1,096 lessons,
// on text an author controls, and the patterns we would otherwise want — nested
// emphasis, bracketed cues, alternating delimiter cells — are exactly the shapes that
// turn into catastrophic backtracking. A linear scan has no backtracking to catch
// fire. `modality.ts` made the same call for its cue list; this file keeps the habit.

// ---------------------------------------------------------------------------
// Part 1 — reading a row of a Markdown table
// ---------------------------------------------------------------------------

/**
 * Split one Markdown table row into its trimmed cells.
 *
 * A GFM row is cells fenced by pipes — `| a | b | c |` — so the cells are the
 * pipe-separated fields with the empty leading and trailing fence fields dropped. A
 * pipe written `\|` is *content*, not a fence, and is unescaped in place.
 *
 *   `| word | gloss |`          -> ["word", "gloss"]
 *   `| a \| b | c |`            -> ["a | b", "c"]
 *   `| yo | tú | él | ella |`   -> ["yo", "tú", "él", "ella"]
 */
export function splitTableRow(line: string): string[] {
  const fields: string[] = [];
  let current = "";
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (character === "\\" && line[index + 1] === "|") {
      current += "|";
      index += 1;
      continue;
    }
    if (character === "|") {
      fields.push(current);
      current = "";
      continue;
    }
    current += character;
  }
  fields.push(current);
  // A fenced row opens and closes with a pipe, producing an empty field at each end.
  // Those are the fence, not cells. An unfenced row (`a | b`) has neither.
  if (fields.length > 0 && (fields[0] ?? "").trim() === "") fields.shift();
  if (fields.length > 0 && (fields[fields.length - 1] ?? "").trim() === "") fields.pop();
  return fields.map((field) => field.trim());
}

/** True when a line opens a table row: at most three spaces of indent, then a pipe. */
export function isTableRowLine(line: string): boolean {
  let index = 0;
  while (index < line.length && index < 4 && (line[index] === " " || line[index] === "\t")) {
    index += 1;
  }
  return index < 4 && line[index] === "|";
}

/**
 * True for a GFM delimiter cell: `---`, `:--`, `--:`, `:-:` and friends.
 *
 * Hand-scanned rather than `/^:?-+:?$/` — see the header note. One optional colon,
 * one or more dashes, one optional colon, then the end. Nothing else.
 */
export function isDelimiterCell(cell: string): boolean {
  const text = cell.trim();
  let index = 0;
  if (text[index] === ":") index += 1;
  let dashes = 0;
  while (text[index] === "-") {
    index += 1;
    dashes += 1;
  }
  if (text[index] === ":") index += 1;
  return dashes > 0 && index === text.length;
}

// ---------------------------------------------------------------------------
// Part 2 — finding the tables in a lesson
// ---------------------------------------------------------------------------

/** A maximal run of consecutive table-row lines, split into cells. */
export interface MarkdownTable {
  /** 0-based index of the run's first line within the text it was found in. */
  startLine: number;
  /** Every row in the run, cells already trimmed. */
  rows: string[][];
  /**
   * True when row 1 is a GFM delimiter row as wide as row 0.
   *
   * Without it the run is not a table at all — Markdown renders it as literal text
   * with visible pipes — so we cannot know which row is the header, and refusing is
   * the only honest option.
   */
  delimited: boolean;
}

/**
 * Every run of table rows in a Markdown string, in document order.
 *
 * Deliberately structural and dumb: it does not care whether a run *is* a valid
 * table, only where the pipe rows are. Judgement happens in {@link linariseTable},
 * so that "this looks like a table but is malformed" stays a reportable refusal
 * instead of vanishing here.
 */
export function findMarkdownTables(text: string): MarkdownTable[] {
  const lines = text.split(/\r?\n/);
  const tables: MarkdownTable[] = [];
  let index = 0;
  while (index < lines.length) {
    if (!isTableRowLine(lines[index] ?? "")) {
      index += 1;
      continue;
    }
    const startLine = index;
    const rows: string[][] = [];
    while (index < lines.length && isTableRowLine(lines[index] ?? "")) {
      rows.push(splitTableRow(lines[index] ?? ""));
      index += 1;
    }
    const header = rows[0] ?? [];
    const separator = rows[1];
    const delimited =
      separator !== undefined &&
      separator.length === header.length &&
      header.length > 0 &&
      separator.every(isDelimiterCell);
    tables.push({ startLine, rows, delimited });
  }
  return tables;
}

// ---------------------------------------------------------------------------
// Part 3 — inline Markdown, spoken
// ---------------------------------------------------------------------------

/**
 * Symbols the corpus uses as shorthand, and the words a voice should say instead.
 *
 * `→` and `←` are the etymology arrows — *"aqua → ewe → eaue → eau"*, *"pēru ←
 * Dravidian pēr"* — and they are load-bearing: silently dropping them turns a
 * derivation into a list of unrelated words. `·` is the syllable separator
 * (*"na · ma · s · kā · raṁ"*), which becomes a comma so the voice puts a beat
 * between syllables rather than sliding them together.
 *
 * The em dash is left alone: every speech engine already reads it as a pause, which
 * is precisely what the author meant by it.
 */
const SPOKEN_SYMBOLS: ReadonlyArray<readonly [string, string]> = [
  ["←", " from "],
  ["↔", " corresponds to "],
  ["·", ", "],
];

/**
 * Turn one span of inline Markdown into words.
 *
 * What it removes, and why each would otherwise be *spoken aloud* by a TTS engine:
 *
 *   | Markdown            | Naive TTS says              | We produce      |
 *   |---------------------|-----------------------------|-----------------|
 *   | `**hola**`          | "asterisk asterisk hola…"   | `hola`          |
 *   | `` `silent-h` ``    | "backtick silent dash h…"   | `silent-h`      |
 *   | `[guide](x.md)`     | "guide bracket paren x…"    | `guide`         |
 *   | `\*pēr`             | "backslash asterisk pēr"    | `*pēr`          |
 *   | `aqua → ewe`        | "aqua ewe" (link lost!)     | `aqua becomes ewe` |
 *
 * Link *destinations* are dropped entirely rather than read out. A URL is not
 * teaching material and "h t t p s colon slash slash" in the middle of a Spanish
 * lesson is worse than useless; the book keeps the real hyperlink, and the narration
 * keeps only the words a listener can act on.
 *
 * The scan is one pass. `\` escapes are consumed before anything else looks at the
 * character they protect, which is what makes `*\*pēr*` come out as `*pēr` and not
 * as an unbalanced emphasis run.
 */
export function speakableInline(markdown: string, depth = 0): string {
  // A link's words are themselves inline Markdown, so resolving one recurses. Each
  // level consumes at least the `](…)` wrapper, which bounds the depth at roughly one
  // frame per five characters — but "roughly" is not a guarantee, and a lesson file is
  // just text in a pull request. Past a depth no real lesson approaches, stop
  // descending and keep the remaining text verbatim: slightly noisier speech is a much
  // better outcome than a build that dies on a crafted string.
  if (depth > 32) return collapseSpaces(markdown);
  let out = "";
  let index = 0;
  while (index < markdown.length) {
    const character = markdown[index] as string;

    // An escape protects exactly the next character, and the backslash is not spoken.
    //
    // With one exception: `\*`. In this corpus that is always the linguist's
    // reconstruction marker — *"Dravidian \*pēr"*, *"question-stem \*yā-"* — which is
    // a mark for the eye. A speech engine handed a bare `*` either says "asterisk" or
    // stumbles, and neither teaches anything, so the typography characters are
    // dropped whether or not they arrived escaped.
    if (character === "\\" && index + 1 < markdown.length) {
      const escaped = markdown[index + 1] as string;
      if (escaped !== "*" && escaped !== "`" && escaped !== "~") out += escaped;
      index += 2;
      continue;
    }

    // Emphasis and code fences are typography. They carry no sound.
    if (character === "*" || character === "`" || character === "~") {
      index += 1;
      continue;
    }

    // `[text](destination)` and `![alt](destination)` -> the words only. Brackets are
    // matched with a depth counter because the corpus really does nest them, e.g.
    // `[YOU SAY: the pattern — "[nā] [pēru]"]`.
    if (character === "[" || (character === "!" && markdown[index + 1] === "[")) {
      const open = character === "!" ? index + 1 : index;
      let depth = 0;
      let cursor = open;
      let close = -1;
      while (cursor < markdown.length) {
        const scanned = markdown[cursor];
        if (scanned === "\\") {
          cursor += 2;
          continue;
        }
        if (scanned === "[") depth += 1;
        else if (scanned === "]") {
          depth -= 1;
          if (depth === 0) {
            close = cursor;
            break;
          }
        }
        cursor += 1;
      }
      if (close !== -1 && markdown[close + 1] === "(") {
        const end = markdown.indexOf(")", close + 1);
        if (end !== -1) {
          out += speakableInline(markdown.slice(open + 1, close), depth + 1);
          index = end + 1;
          continue;
        }
      }
      // Not a link — an ordinary bracket. Keep it; the caller decides what it means.
      out += character;
      index += 1;
      continue;
    }

    // The rightward arrow means three different things depending on what follows it,
    // and getting that wrong produces the single worst-sounding line in the whole
    // export. All three readings are decided by a fixed lookahead — never a pattern.
    //
    //   `aqua → ewe`                    derivation      -> "aqua becomes ewe"
    //   `→ [pronunciation reference]`   cross-reference -> "see pronunciation reference"
    //   `= a-vu-nu →` (end of line)     points at what  -> "= a-vu-nu, which gives:"
    //                                   comes next
    if (character === "→" || character === "⇒") {
      let ahead = index + 1;
      while (markdown[ahead] === " " || markdown[ahead] === "\t" || markdown[ahead] === "\n") {
        ahead += 1;
      }
      if (ahead >= markdown.length) {
        out += ", which gives:";
        index = ahead;
        continue;
      }
      if (markdown[ahead] === "[") {
        out += " see ";
        index = ahead;
        continue;
      }
      out += " becomes ";
      index += 1;
      continue;
    }

    let replaced = false;
    for (const [symbol, spoken] of SPOKEN_SYMBOLS) {
      if (character === symbol) {
        out += spoken;
        replaced = true;
        break;
      }
    }
    if (replaced) {
      index += 1;
      continue;
    }

    out += character;
    index += 1;
  }
  return collapseSpaces(out);
}

/**
 * Squeeze runs of whitespace to one space and trim. Speech has no line breaks.
 *
 * It also drops a space that lands *before* punctuation. That is not cosmetic: the
 * symbol substitutions above insert their own leading punctuation — `·` becomes `", "`
 * — so `అ · వు` would otherwise come out as `అ , వు`, and a speech engine handed a
 * floating comma reads the pause in the wrong place.
 */
export function collapseSpaces(text: string): string {
  const characters = [...text];
  let out = "";
  let pendingSpace = false;
  for (let index = 0; index < characters.length; index += 1) {
    const character = characters[index] as string;
    if (character === " " || character === "\t" || character === "\n" || character === "\r") {
      pendingSpace = out.length > 0;
      continue;
    }
    if (pendingSpace && !",.;:!?".includes(character)) out += " ";
    pendingSpace = false;
    out += character;
  }
  return out;
}

/**
 * Add a full stop when a spoken sentence does not already end in one.
 *
 * The subtlety is closing punctuation. Lessons end sentences inside brackets and
 * quotes all the time — *`(No — a coincidence, a false friend.)`* — and looking only
 * at the very last character sees `)`, decides the sentence is unterminated, and
 * produces `.).`, which a speech engine reads as a stumble. So closers are peeled off
 * before the question is asked.
 */
export function endSentence(text: string): string {
  const trimmed = text.trim();
  if (trimmed === "") return "";
  let index = trimmed.length - 1;
  while (index >= 0 && ")]}\"'”’»".includes(trimmed[index] as string)) index -= 1;
  const last = index >= 0 ? (trimmed[index] as string) : "";
  return ".,;:!?…。".includes(last) ? trimmed : `${trimmed}.`;
}

// ---------------------------------------------------------------------------
// Part 4 — linearisation
// ---------------------------------------------------------------------------

/** Why a table could not be turned into speech. Every value is reportable to a learner. */
export type TableRefusalReason = "too-wide" | "ragged-row" | "no-rows";

/** Human sentences for the refusal codes, used in both the report and the spoken notice. */
export const TABLE_REFUSAL_MESSAGES: Readonly<Record<TableRefusalReason, string>> = {
  "too-wide": "it has more columns than can be held in the ear at once",
  "ragged-row": "its rows do not all have the same number of cells",
  "no-rows": "there is nothing under its heading row",
};

export interface LinearisedTable {
  ok: true;
  headers: string[];
  columns: number;
  rowCount: number;
  /** One spoken sentence per body row, in authored order. */
  utterances: string[];
}

export interface RefusedTable {
  ok: false;
  reason: TableRefusalReason;
  /** Widest row, in cells — the number a learner and an author both want to hear. */
  columns: number;
  /** Header cells when one could be identified; empty otherwise. */
  headers: string[];
  rowCount: number;
}

export type TableSpeech = LinearisedTable | RefusedTable;

export interface TableSpeechOptions {
  /** Widest table still considered speakable. */
  maxColumns?: number;
}

/**
 * Second-column headings that mean "and here is what it means".
 *
 * When a two-column table's right-hand heading is one of these, the row is spoken as
 * HL08's own example — *"X means Y"* — instead of the generic labelled form. It is a
 * small thing that makes the single most common table in the corpus sound like a
 * teacher rather than a database dump. Matched by exact lowercase equality against a
 * Set: no substring search, no pattern, no surprises from a heading like
 * "Meaningless".
 */
const GLOSS_HEADINGS: ReadonlySet<string> = new Set([
  "english",
  "meaning",
  "means",
  "gloss",
  "sense",
  "translation",
  "in english",
  "what it means",
]);

/**
 * The corpus's default speakable width, and the reasoning behind the number.
 *
 * **Three.** Measured over the 340 table-bearing lesson files: 99 have a widest row
 * of 2, 173 of 3, 60 of 4, and 8 of 5 or more. Two and three columns are a *labelled
 * fact* — "Language: Telugu. Hello: namaskāram. Source: Sanskrit." — which a listener
 * holds without effort. At four the table stops being a list of facts and becomes a
 * grid whose meaning lives in the *comparison between rows*, and the corpus's own
 * four-column tables prove it: `|   | numeral | word | said |` has an unlabelled
 * first column that only means something because of where it sits on the page.
 *
 * So 3 covers 272 of 340 tables (80%) and stops exactly where honesty would start
 * costing something.
 */
export const DEFAULT_LINEARISABLE_TABLE_COLUMNS = 3;

/**
 * Try to turn one table into a sequence of spoken sentences.
 *
 * Refusal order is deliberate — most actionable cause first — because the reason is
 * shown to an author who has to decide whether to reshape the table or accept the
 * `sight` mark on the lesson.
 */
export function linariseTable(
  table: MarkdownTable,
  options: TableSpeechOptions = {},
): TableSpeech {
  const maxColumns = options.maxColumns ?? DEFAULT_LINEARISABLE_TABLE_COLUMNS;
  const widest = table.rows.reduce((most, row) => Math.max(most, row.length), 0);

  // A run of pipe rows with no delimiter is not a Markdown table at all — the page
  // shows it with the pipes still visible. There is still nothing wrong with saying
  // it: every row is data, so it is read as an unlabelled sequence. This is not an
  // indulgence, it is the safety rule again — refusing would have sent lessons like
  // `| j'habite · tu habites | (all a-BEET) |` to `sight` over a missing `|---|`.
  const headers = table.delimited ? (table.rows[0] ?? []).map(speakableInline) : [];
  const body = table.delimited ? table.rows.slice(2) : table.rows;
  const columns = table.delimited ? headers.length : widest;
  const refuse = (reason: TableRefusalReason): RefusedTable => ({
    ok: false,
    reason,
    columns: widest,
    headers,
    rowCount: body.length,
  });

  if (columns > maxColumns || columns === 0) return refuse("too-wide");
  if (body.length === 0) return refuse("no-rows");
  if (body.some((row) => row.length !== columns)) return refuse("ragged-row");

  const glossShape = columns === 2 && GLOSS_HEADINGS.has((headers[1] ?? "").toLowerCase());

  const utterances = body.map((row) => {
    const cells = row.map(speakableInline);
    if (glossShape) {
      const term = cells[0] ?? "";
      const gloss = cells[1] ?? "";
      // A blank cell has no "means" to offer, so fall back to the labelled form
      // rather than saying "X means" and trailing off.
      if (term === "" || gloss === "") return endSentence(labelled(headers, cells, columns));
      return endSentence(`${term} means ${gloss}`);
    }
    return endSentence(labelled(headers, cells, columns));
  });

  return { ok: true, headers, columns, rowCount: body.length, utterances };
}

/**
 * `"Language: Telugu. Hello: namaskāram. Source: Sanskrit"` — one row, spoken.
 *
 * **A column with no heading is spoken as a bare value**, joined to the column before
 * it with a comma rather than a full stop. That case is not an edge case: the corpus's
 * commonest three-column shape is `| Read | | Meaning |`, where the unlabelled middle
 * column is the romanization sitting visually under the script. Refusing it would
 * have sent dozens of perfectly speakable practice tables to `sight` for the sake of
 * a heading that a sighted reader does not have either. Spoken, it comes out as:
 *
 *     Read: سلام, salām. Meaning: peace.
 *
 * An empty *cell* is different from an empty *heading*: it is spoken as "blank", so a
 * listener can tell a gap in the table from a gap in the narration.
 */
function labelled(
  headers: readonly string[],
  cells: readonly string[],
  columns: number,
): string {
  let out = "";
  for (let column = 0; column < columns; column += 1) {
    const heading = (headers[column] ?? "").trim();
    const raw = (cells[column] ?? "").trim();
    const value = raw === "" ? "blank" : raw;
    if (out === "") out = heading === "" ? value : `${heading}: ${value}`;
    else if (heading === "") out += `, ${value}`;
    // `endSentence` rather than a bare `". "`: a cell can itself be a finished
    // sentence — `| A sopa é boa. — It is a good soup. |` — and gluing a full stop
    // onto one produces `soup.. with estar:`, which a speech engine reads as a
    // stutter.
    else out = `${endSentence(out)} ${heading}: ${value}`;
  }
  return out;
}

/** Linearise every table in a Markdown string, keeping document order. */
export function linariseTables(text: string, options: TableSpeechOptions = {}): TableSpeech[] {
  return findMarkdownTables(text).map((table) => linariseTable(table, options));
}

/**
 * True when some table in this text cannot be spoken — the one question
 * `modality.ts` needs answered before it can call a lesson drivable.
 */
export function hasUnspeakableTable(text: string, options: TableSpeechOptions = {}): boolean {
  return linariseTables(text, options).some((result) => !result.ok);
}
