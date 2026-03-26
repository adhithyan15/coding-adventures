/**
 * Block-Level Parser
 *
 * Phase 1 of CommonMark parsing: split the input into block-level tokens
 * and build the structural skeleton of the document.
 *
 * === Two-Phase Overview ===
 *
 * CommonMark parsing is inherently two-phase:
 *
 *   Phase 1 (this file): Block structure
 *     Input text → lines → block tree with raw inline content strings
 *
 *   Phase 2 (inline-parser.ts): Inline content
 *     Each block's raw content → inline nodes (emphasis, links, etc.)
 *
 * The phases cannot be merged because block structure determines where
 * inline content lives. A `*` that starts a list item is structural;
 * a `*` inside paragraph text may be emphasis.
 *
 * === ModalStateMachine Usage ===
 *
 * The parser uses a ModalStateMachine from @coding-adventures/state-machine
 * to track multi-line block state. The key insight is that most of the
 * parser runs in a "normal" scanning mode, but certain block types require
 * the parser to stay in a distinct mode across multiple lines:
 *
 *   NORMAL mode    — scanning for block starters line by line
 *   FENCED_CODE    — inside ``` or ~~~ block; accumulate raw lines
 *   HTML_BLOCK_1   — inside <script/pre/textarea/style>; ends on </tag>
 *   HTML_BLOCK_2   — inside <!-- comment -->; ends on -->
 *   HTML_BLOCK_3   — inside <? processing ?>; ends on ?>
 *   HTML_BLOCK_4   — inside <!DECLARATION>; ends on >
 *   HTML_BLOCK_5   — inside <![CDATA[...]]>; ends on ]]>
 *   HTML_BLOCK_6   — block-level open/close tag; ends on blank line
 *   HTML_BLOCK_7   — complete tag; ends on blank line
 *
 * The DFA within each mode processes the line's first characters to
 * determine whether the mode continues or terminates.
 *
 * === Block Tree Construction ===
 *
 * Container blocks (document, blockquote, list items) form a stack.
 * When a new line arrives, we walk down the stack checking continuations,
 * then add the line's content to the appropriate block.
 *
 * @module block-parser
 */

import { ModalStateMachine, DFA, transitionKey } from "@coding-adventures/state-machine";
import type {
  DocumentNode, BlockNode, HeadingNode, ParagraphNode,
  CodeBlockNode, BlockquoteNode, ListNode, ListItemNode, TaskItemNode,
  TableNode, TableRowNode, TableCellNode, TableAlignment,
  ThematicBreakNode, RawBlockNode,
} from "@coding-adventures/document-ast";
import type { LinkRefMap, LinkReference } from "./types.js";
import { normalizeLinkLabel, normalizeUrl, isAsciiPunctuation, isAsciiWhitespace } from "./scanner.js";
import { decodeEntity, decodeEntities } from "./entities.js";

/** Apply backslash escapes — only for ASCII punctuation characters. */
function applyBackslashEscapes(s: string): string {
  return s.replace(/\\(.)/g, (match, ch: string) =>
    isAsciiPunctuation(ch) ? ch : match
  );
}

// ─── Internal Block Representations ──────────────────────────────────────────
//
// During parsing we use mutable intermediate representations, then freeze
// them into the readonly AST types at the end.

type MutableBlock =
  | MutableDocument
  | MutableBlockquote
  | MutableList
  | MutableListItem
  | MutableParagraph
  | MutableFencedCode
  | MutableIndentedCode
  | MutableHtmlBlock
  | MutableHeading
  | MutableThematicBreak
  | MutableLinkDef;

interface MutableDocument {
  kind: "document";
  children: MutableBlock[];
}

interface MutableBlockquote {
  kind: "blockquote";
  children: MutableBlock[];
}

interface MutableList {
  kind: "list";
  ordered: boolean;
  marker: string; // the marker character: - * + or ) .
  start: number;
  tight: boolean;
  items: MutableListItem[];
  hadBlankLine: boolean; // track blank lines between items
}

interface MutableListItem {
  kind: "list_item";
  marker: string;
  markerIndent: number;  // indentation of the marker
  contentIndent: number; // how many spaces of indentation the content needs
  children: MutableBlock[];
  hadBlankLine: boolean; // blank line inside this item
}

interface MutableParagraph {
  kind: "paragraph";
  lines: string[]; // raw lines, joined with \n before inline parsing
}

interface MutableFencedCode {
  kind: "fenced_code";
  fence: string;     // the opening fence characters (``` or ~~~)
  fenceLen: number;  // length of the fence (>= 3)
  baseIndent: number; // indentation of opening fence (0-3), stripped from content lines
  infoString: string;
  lines: string[];
  closed: boolean;
}

interface MutableIndentedCode {
  kind: "indented_code";
  lines: string[]; // each line with 4 leading spaces stripped
}

interface MutableHtmlBlock {
  kind: "html_block";
  htmlType: 1 | 2 | 3 | 4 | 5 | 6 | 7;
  lines: string[];
  closed: boolean;
}

interface MutableHeading {
  kind: "heading";
  level: 1 | 2 | 3 | 4 | 5 | 6;
  content: string; // raw inline content
}

interface MutableThematicBreak {
  kind: "thematic_break";
}

interface MutableLinkDef {
  kind: "link_def";
  label: string;
  destination: string;
  title: string | null;
}

// ─── ModalStateMachine Setup ──────────────────────────────────────────────────
//
// We build a ModalStateMachine with modes corresponding to the different
// multi-line block parsing contexts. Within each mode, a DFA processes
// "events" that represent classified line prefixes.
//
// Events sent to the DFA:
//   "blank"    — the line is all whitespace
//   "content"  — the line has non-whitespace content
//   "fence"    — in FENCED_CODE mode, line starts closing fence
//   "end_tag"  — in HTML_BLOCK_x modes, line matches the end condition
//
// The DFA states within each mode represent parsing progress.

function buildNormalModeDFA(): DFA {
  // In NORMAL mode the DFA just has two states:
  //   "scanning" → always stays in "scanning"
  // All the interesting logic is procedural in processLine().
  // The DFA acts as a placeholder; we use mode-switching for transitions.
  const states = new Set(["scanning"]);
  const alphabet = new Set(["blank", "content"]);
  const transitions = new Map<string, string>([
    [transitionKey("scanning", "blank"), "scanning"],
    [transitionKey("scanning", "content"), "scanning"],
  ]);
  return new DFA(states, alphabet, transitions, "scanning", new Set(["scanning"]));
}

function buildFencedCodeDFA(): DFA {
  // FENCED_CODE mode stays "open" until a closing fence is seen.
  //   "open" --content--> "open"   (accumulate)
  //   "open" --blank-->   "open"   (blank lines inside code block are kept)
  //   "open" --fence-->   "closed" (closing fence)
  const states = new Set(["open", "closed"]);
  const alphabet = new Set(["blank", "content", "fence"]);
  const transitions = new Map<string, string>([
    [transitionKey("open", "blank"),   "open"],
    [transitionKey("open", "content"), "open"],
    [transitionKey("open", "fence"),   "closed"],
    [transitionKey("closed", "blank"),   "closed"],
    [transitionKey("closed", "content"), "closed"],
    [transitionKey("closed", "fence"),   "closed"],
  ]);
  return new DFA(states, alphabet, transitions, "open", new Set(["open", "closed"]));
}

function buildHtmlBlockDFA(): DFA {
  // HTML block mode stays "open" until the end condition is met.
  const states = new Set(["open", "closed"]);
  const alphabet = new Set(["blank", "content", "end_tag"]);
  const transitions = new Map<string, string>([
    [transitionKey("open", "blank"),   "open"],
    [transitionKey("open", "content"), "open"],
    [transitionKey("open", "end_tag"), "closed"],
    [transitionKey("closed", "blank"),   "closed"],
    [transitionKey("closed", "content"), "closed"],
    [transitionKey("closed", "end_tag"), "closed"],
  ]);
  return new DFA(states, alphabet, transitions, "open", new Set(["open", "closed"]));
}

// Build the ModalStateMachine. Each multi-line block type is a mode.
// The mode transitions are driven by the block parser's processLine() method
// via the machine's switchMode() calls.

const MODES = new Map([
  ["normal",     buildNormalModeDFA()],
  ["fenced",     buildFencedCodeDFA()],
  ["html_block", buildHtmlBlockDFA()],
]);

const MODE_TRANSITIONS = new Map<string, string>([
  // normal → fenced when we open a code fence
  [transitionKey("normal", "enter_fenced"), "fenced"],
  // fenced → normal when we close the fence OR force-close at EOF
  [transitionKey("fenced", "exit_fenced"), "normal"],
  // normal → html_block when we open an HTML block
  [transitionKey("normal", "enter_html"), "html_block"],
  // html_block → normal when the end condition is met
  [transitionKey("html_block", "exit_html"), "normal"],
]);

// ─── HTML Block Pattern Helpers ───────────────────────────────────────────────

// CommonMark defines 7 types of HTML blocks. Each has different opening
// and closing conditions. Types 1-5 end on specific closing tags/markers.
// Types 6-7 end on a blank line.

const HTML_BLOCK_1_OPEN  = /^<(?:script|pre|textarea|style)(?:\s|>|$)/i;
const HTML_BLOCK_1_CLOSE = /<\/(?:script|pre|textarea|style)>/i;
const HTML_BLOCK_2_OPEN  = /^<!--/;
// CommonMark spec §4.6: type-2 HTML blocks end at --> (standard comment close).
// Also accept --!> per HTML5 §13.2.6.2, which some parsers treat as comment end.
const HTML_BLOCK_2_CLOSE = /--!?>/;
const HTML_BLOCK_3_OPEN  = /^<\?/;
const HTML_BLOCK_3_CLOSE = /\?>/;
const HTML_BLOCK_4_OPEN  = /^<![A-Z]/;
const HTML_BLOCK_4_CLOSE = />/;
const HTML_BLOCK_5_OPEN  = /^<!\[CDATA\[/;
const HTML_BLOCK_5_CLOSE = /\]\]>/;

// Type 6: open/close tag for block-level HTML elements
const HTML_BLOCK_6_TAGS = new Set([
  "address","article","aside","base","basefont","blockquote","body",
  "caption","center","col","colgroup","dd","details","dialog","dir",
  "div","dl","dt","fieldset","figcaption","figure","footer","form",
  "frame","frameset","h1","h2","h3","h4","h5","h6","head","header",
  "hr","html","iframe","legend","li","link","main","menu","menuitem",
  "meta","nav","noframes","ol","optgroup","option","p","param",
  "search","section","summary","table","tbody","td","tfoot","th",
  "thead","title","tr","track","ul",
]);

const HTML_BLOCK_6_OPEN = new RegExp(
  `^</?(?:${[...HTML_BLOCK_6_TAGS].join("|")})(?:\\s|>|/>|$)`, "i"
);

// Type 7: a complete open tag, closing tag, or processing instruction
// that is NOT in the type 6 list. Ends on blank line.
// (We detect type 7 after ruling out types 1-6)
//
// Pre-compiled at module load (not inside the function) to avoid ReDoS
// amplification: constructing a RegExp on every call means V8 re-analyses
// the pattern for potential catastrophic backtracking on every line.
// The attribute group `(ATTR7)*` has repetition over a group with alternatives,
// which historically triggered backtracking engines. Module-level compilation
// gives V8 the chance to JIT-compile the pattern once and detect problematic
// paths early.
const _HTML_BLOCK_7_ATTR = String.raw`(?:\s+[a-zA-Z_:][a-zA-Z0-9_:.\-]*(?:\s*=\s*(?:[^\s"'=<>\x60]+|'[^'\n]*'|"[^"\n]*"))?)`;
const HTML_BLOCK_7_OPEN_TAG  = new RegExp(String.raw`^<[A-Za-z][A-Za-z0-9\-]*(${_HTML_BLOCK_7_ATTR})*\s*/?>$`);
const HTML_BLOCK_7_CLOSE_TAG = /^<\/[A-Za-z][A-Za-z0-9\-]*\s*>$/;

function detectHtmlBlockType(line: string): 1 | 2 | 3 | 4 | 5 | 6 | 7 | null {
  const stripped = line.trimStart();
  if (HTML_BLOCK_1_OPEN.test(stripped)) return 1;
  if (HTML_BLOCK_2_OPEN.test(stripped)) return 2;
  if (HTML_BLOCK_3_OPEN.test(stripped)) return 3;
  if (HTML_BLOCK_4_OPEN.test(stripped)) return 4;
  if (HTML_BLOCK_5_OPEN.test(stripped)) return 5;
  if (HTML_BLOCK_6_OPEN.test(stripped)) return 6;

  // Type 7: complete open or close tag with valid attribute syntax (spec §4.6).
  if (HTML_BLOCK_7_OPEN_TAG.test(stripped) || HTML_BLOCK_7_CLOSE_TAG.test(stripped)) {
    return 7;
  }
  return null;
}

function htmlBlockEnds(line: string, htmlType: number): boolean {
  switch (htmlType) {
    case 1: return HTML_BLOCK_1_CLOSE.test(line);
    case 2: return HTML_BLOCK_2_CLOSE.test(line);
    case 3: return HTML_BLOCK_3_CLOSE.test(line);
    case 4: return HTML_BLOCK_4_CLOSE.test(line);
    case 5: return HTML_BLOCK_5_CLOSE.test(line);
    case 6: case 7: return /^\s*$/.test(line); // blank line ends types 6 and 7
    default: return false;
  }
}

// ─── Line Classification Helpers ─────────────────────────────────────────────

/** True if the line is blank (empty or only whitespace). */
function isBlank(line: string): boolean {
  return /^\s*$/.test(line);
}

/**
 * Count leading virtual spaces (expanding tabs to the next 4-column tab stop).
 *
 * `baseCol` is the virtual column of `line[0]` in the original document —
 * necessary after partial-tab stripping, where the string may start mid-tab.
 * Returns the number of virtual indentation spaces (relative to `baseCol`).
 *
 * Example: indentOf("  \tbar", 2) — the two spaces are at cols 2-3, the tab
 * starts at col 4 and expands to col 8 (adds 4 virtual spaces). Returns 6.
 */
function indentOf(line: string, baseCol = 0): number {
  let col = baseCol;
  for (const ch of line) {
    if (ch === " ") col++;
    else if (ch === "\t") col += 4 - (col % 4);
    else break;
  }
  return col - baseCol;
}

/**
 * Strip exactly `n` virtual spaces of leading indentation, expanding tabs
 * correctly relative to `baseCol` (the virtual column of `line[0]`).
 *
 * Returns `[strippedLine, nextBaseCol]` where `nextBaseCol` is the virtual
 * column of `strippedLine[0]` — which is `baseCol` after advancing past the
 * stripped indentation.
 *
 * **Partial-tab handling**: When a tab spans the strip boundary (the tab
 * would expand past `n` virtual spaces), we consume the tab character and
 * prepend the leftover expansion spaces to the result. This correctly
 * represents the remaining part of the tab as explicit spaces.
 *
 * Example: stripIndent("\t\tbar", 2, 0)
 *   First tab at col 0 → expands 4 spaces; we need 2 → leftover 2 spaces.
 *   Returns ["  \tbar", 2]  (two spaces + second tab, base col = 2).
 */
function stripIndent(line: string, n: number, baseCol = 0): [string, number] {
  let remaining = n;
  let col = baseCol;
  let i = 0;
  while (remaining > 0 && i < line.length) {
    if (line[i] === " ") { i++; remaining--; col++; }
    else if (line[i] === "\t") {
      const w = 4 - (col % 4);
      if (w <= remaining) { i++; remaining -= w; col += w; }
      else {
        // Partial tab: consume the tab character but prepend the leftover
        // virtual spaces to the result so they are visible to the next pass.
        const leftover = w - remaining;
        return [" ".repeat(leftover) + line.slice(i + 1), col + remaining];
      }
    } else break;
  }
  return [line.slice(i), col];
}

/**
 * Compute the virtual column reached after consuming `charCount` characters
 * from `line`, starting at virtual column `startCol`.
 *
 * Used to find the virtual column of the list-marker separator character so
 * that a tab separator can be partially consumed (1 virtual space used as the
 * required separator space, remainder prepended to the item content).
 */
function virtualColAfter(line: string, charCount: number, startCol = 0): number {
  let col = startCol;
  for (let i = 0; i < charCount && i < line.length; i++) {
    if (line[i] === "\t") col += 4 - (col % 4);
    else col++;
  }
  return col;
}

/** Extract the info string from a fenced code block opening line. */
function extractInfoString(line: string): string {
  // Strip the fence characters and leading whitespace from the info string
  const m = line.match(/^[`~]+\s*(.*)$/);
  if (!m) return "";
  // Per CommonMark: only the first word is the language
  const raw = m[1]!.trim().split(/\s+/)[0] ?? "";
  // Apply backslash escapes (punctuation only) and entity decoding to the info string
  return decodeEntities(applyBackslashEscapes(raw));
}

// ─── ATX Heading Detection ────────────────────────────────────────────────────

interface AtxHeading {
  level: 1 | 2 | 3 | 4 | 5 | 6;
  content: string;
}

function parseAtxHeading(line: string): AtxHeading | null {
  // Up to 3 spaces of indentation, then 1-6 # chars, then space or end-of-line
  const m = line.match(/^ {0,3}(#{1,6})([ \t]|$)(.*)/);
  if (!m) return null;

  const hashes = m[1]!;
  // m[3] is everything after the first space (or empty if heading was just hashes)
  let content = (m[3] ?? "").trimEnd();

  // Remove closing hash sequence: space/tab + one or more hashes + optional spaces
  content = content.replace(/[ \t]+#+[ \t]*$/, "");
  // If content is now purely hashes (e.g. `### ###` → content becomes `###`), it was
  // the closing sequence and the heading is empty.
  if (/^#+[ \t]*$/.test(content)) content = "";

  return {
    level: hashes.length as 1 | 2 | 3 | 4 | 5 | 6,
    content: content.trim(),
  };
}

// ─── Thematic Break Detection ─────────────────────────────────────────────────

function isThematicBreak(line: string): boolean {
  // 0-3 spaces, then 3+ of *, -, or _ optionally separated by spaces/tabs
  return /^ {0,3}((?:\*[ \t]*){3,}|(?:-[ \t]*){3,}|(?:_[ \t]*){3,})\s*$/.test(line);
}

// ─── List Item Detection ──────────────────────────────────────────────────────

interface ListMarker {
  ordered: boolean;
  start: number;
  marker: string;     // the delimiter character: - * + . )
  markerLen: number;  // total characters consumed by marker + space
  spaceAfter: number; // spaces/tab after marker (0 for end-of-line items)
  indent: number;     // spaces before marker
}

function parseListMarker(line: string): ListMarker | null {
  // Unordered: up to 3 spaces + (- * +) + (space, tab, or end-of-line)
  const unordered = line.match(/^( {0,3})([-*+])( +|\t|$)/);
  if (unordered) {
    const indent = unordered[1]!.length;
    const marker = unordered[2]!;
    const space = unordered[3]!;
    return {
      ordered: false,
      start: 1,
      marker,
      markerLen: indent + 1 + space.length,
      spaceAfter: space.length,
      indent,
    };
  }

  // Ordered: up to 3 spaces + 1-9 digits + (. or )) + (space, tab, or end-of-line)
  const ordered = line.match(/^( {0,3})(\d{1,9})([.)])( +|\t|$)/);
  if (ordered) {
    const indent = ordered[1]!.length;
    const num = parseInt(ordered[2]!, 10);
    const delim = ordered[3]!;
    const space = ordered[4]!;
    const markerWidth = ordered[2]!.length + 1; // digits + delimiter
    return {
      ordered: true,
      start: num,
      marker: delim,
      markerLen: indent + markerWidth + space.length,
      spaceAfter: space.length,
      indent,
    };
  }

  return null;
}

// ─── Setext Heading Detection ─────────────────────────────────────────────────

function isSetextUnderline(line: string): 1 | 2 | null {
  if (/^ {0,3}=+\s*$/.test(line)) return 1;
  if (/^ {0,3}-+\s*$/.test(line)) return 2;
  return null;
}

// ─── Link Reference Definition Parsing ───────────────────────────────────────
//
// Link reference definitions can span multiple lines and appear anywhere
// in the document. We attempt to parse them greedily when we see `[` at
// the start of a line.

interface ParsedLinkDef {
  label: string;
  destination: string;
  title: string | null;
  charsConsumed: number; // total characters consumed (including newlines)
}

function parseLinkDefinition(text: string): ParsedLinkDef | null {
  // Link label: up to 3 leading spaces + [...]:
  // Labels may NOT contain unescaped `[` (spec §4.7). The `\[` in the
  // character class excludes bare `[` while still allowing `\[` escapes.
  const labelMatch = text.match(/^ {0,3}\[([^\]\\\[]*(?:\\.[^\]\\\[]*)*)\]:/);
  if (!labelMatch) return null;

  const rawLabel = labelMatch[1]!;
  if (rawLabel.trim() === "") return null; // empty label not allowed
  // CommonMark §4.7: no backslash escaping is performed on link labels.
  // The only role of `\` is to allow brackets (`\[`, `\]`) in the label.
  // We normalize the raw label as-is (backslashes included).
  const label = normalizeLinkLabel(rawLabel);
  let pos = labelMatch[0].length;

  // Skip whitespace (including one newline)
  const wsMatch = text.slice(pos).match(/^[ \t]*\n?[ \t]*/);
  if (wsMatch) pos += wsMatch[0].length;

  // Destination: either <...> or non-whitespace non-control chars
  let destination = "";
  if (text[pos] === "<") {
    const angleMatch = text.slice(pos).match(/^<([^<>\n\\]*(?:\\.[^<>\n\\]*)*)>/);
    if (!angleMatch) return null;
    destination = normalizeUrl(decodeEntities(applyBackslashEscapes(angleMatch[1]!)));
    pos += angleMatch[0].length;
  } else {
    // Non-angle-bracket destination: no spaces, no control chars, balanced parens
    let depth = 0;
    const start = pos;
    while (pos < text.length) {
      const ch = text[pos]!;
      if (ch === "(") { depth++; pos++; }
      else if (ch === ")") {
        if (depth === 0) break;
        depth--;
        pos++;
      } else if (/[\s\x00-\x1f]/.test(ch)) {
        break;
      } else if (ch === "\\") {
        pos += 2; // skip `\X` pair (we extract range later)
      } else {
        pos++;
      }
    }
    if (pos === start) return null; // empty destination
    destination = normalizeUrl(decodeEntities(applyBackslashEscapes(text.slice(start, pos))));
  }

  // Optional title
  let title: string | null = null;
  const beforeTitle = pos;
  const spacesMatch = text.slice(pos).match(/^[ \t]*\n?[ \t]*/);
  if (spacesMatch && spacesMatch[0].length > 0) {
    pos += spacesMatch[0].length;
    const titleChar = text[pos];
    let closeChar = "";
    if (titleChar === '"') closeChar = '"';
    else if (titleChar === "'") closeChar = "'";
    else if (titleChar === "(") closeChar = ")";

    if (closeChar !== "") {
      pos++; // skip open char
      const titleStart = pos;
      let escaped = false;
      while (pos < text.length) {
        const ch = text[pos]!;
        if (escaped) { escaped = false; pos++; continue; }
        if (ch === "\\") { escaped = true; pos++; continue; }
        if (ch === closeChar) { pos++; break; }
        if (ch === "\n" && closeChar === ")") break; // parens don't allow newlines
        pos++;
      }
      if (text[pos - 1] === closeChar) {
        title = decodeEntities(applyBackslashEscapes(text.slice(titleStart, pos - 1)));
      } else {
        // Failed to parse title — restore position
        pos = beforeTitle;
        title = null;
      }
    } else {
      pos = beforeTitle;
    }
  }

  // Must be followed by only whitespace on the rest of the line
  const eolMatch = text.slice(pos).match(/^[ \t]*(?:\n|$)/);
  if (!eolMatch) {
    // If we had a title parse attempt, maybe the title was not present
    if (title !== null) {
      pos = beforeTitle;
      title = null;
      const eolMatch2 = text.slice(pos).match(/^[ \t]*(?:\n|$)/);
      if (!eolMatch2) return null;
      pos += eolMatch2[0].length;
    } else {
      return null;
    }
  } else {
    pos += eolMatch[0].length;
  }

  return { label, destination, title, charsConsumed: pos };
}

// ─── Main Block Parser ────────────────────────────────────────────────────────

/**
 * Parse a CommonMark document into a block-level tree (Phase 1).
 *
 * Returns both the tree and the link reference map, which Phase 2
 * (inline parsing) uses to resolve `[text][label]` links.
 */
export function parseBlocks(input: string): {
  document: MutableDocument;
  linkRefs: LinkRefMap;
} {
  // Normalize line endings to LF, then split into lines.
  // We keep the newlines by splitting on \n and processing each line.
  const normalized = input.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const rawLines = normalized.split("\n");
  // The trailing newline at end of input produces a spurious empty string after
  // splitting. Remove it — it is not a real line to process.
  if (rawLines.length > 0 && rawLines[rawLines.length - 1] === "") {
    rawLines.pop();
  }

  // The ModalStateMachine tracks which multi-line block context we're in
  const modal = new ModalStateMachine(MODES, MODE_TRANSITIONS, "normal");

  const linkRefs: LinkRefMap = new Map();
  const root: MutableDocument = { kind: "document", children: [] };

  // Container block stack. The innermost open container is at the end.
  // We always start with the document.
  let openContainers: MutableBlock[] = [root];

  // Track the current open leaf block (paragraph, code block, etc.)
  let currentLeaf: MutableBlock | null = null;

  // Track blank lines for list tightness
  let lastLineWasBlank = false;
  // Track the innermost container during the last blank line, so we can
  // distinguish inter-item blanks (at list/list_item level) from blanks
  // inside nested sub-containers (which should NOT make the outer list loose).
  let lastBlankInnerContainer: MutableBlock = root;

  for (let lineIdx = 0; lineIdx < rawLines.length; lineIdx++) {
    const rawLine = rawLines[lineIdx]!;
    const origBlank = isBlank(rawLine);

    // ── Container continuation ────────────────────────────────────────────
    //
    // Always walk the container stack FIRST so that:
    //   - fenced/html code blocks inside containers get lineContent with
    //     container markers already stripped
    //   - prevInnerContainer tracking lets us detect when a container is left
    //     and finalize any open leaf block (preventing setext heading confusion)

    let lineContent = rawLine;
    // lineBaseCol tracks the virtual column of lineContent[0] in the original
    // line after container markers have been stripped. This is needed for
    // correct tab-stop calculations (tabs expand to the next multiple of 4
    // relative to the column where they appear in the source, not relative to
    // the start of the stripped string).
    let lineBaseCol = 0;
    const newContainers: MutableBlock[] = [root];
    let lazyParagraphContinuation = false;

    let containerIdx = 1;
    while (containerIdx < openContainers.length) {
      const container = openContainers[containerIdx]!;

      if (container.kind === "blockquote") {
        // Strip the blockquote marker `> ` (up to 3 leading spaces, `>`, then
        // optionally 1 space). A tab after `>` is treated as the separator but
        // only 1 virtual space of it is consumed; the remaining virtual spaces
        // from the tab expansion are prepended to the content (CommonMark §2.1).
        let bqI = 0;
        let bqCol = lineBaseCol;
        // Strip 0–3 leading spaces
        while (bqI < 3 && bqI < lineContent.length && lineContent[bqI] === " ") {
          bqI++; bqCol++;
        }
        if (bqI < lineContent.length && lineContent[bqI] === ">") {
          bqI++; bqCol++;
          if (bqI < lineContent.length) {
            if (lineContent[bqI] === " ") {
              bqI++; bqCol++;
            } else if (lineContent[bqI] === "\t") {
              // Tab at bqCol: expand to next tab stop, consume 1 virtual space
              // as the required separator, prepend the rest to the content.
              const w = 4 - (bqCol % 4);
              bqI++; // consume the tab character
              if (w > 1) {
                // Partial tab: leftover (w-1) virtual spaces become content prefix
                lineContent = " ".repeat(w - 1) + lineContent.slice(bqI);
                lineBaseCol = bqCol + 1;
                newContainers.push(container);
                containerIdx++;
                continue;
              }
              bqCol += w; // w === 1: tab exactly consumed as separator
            }
          }
          lineContent = lineContent.slice(bqI);
          lineBaseCol = bqCol;
          newContainers.push(container);
          containerIdx++;
        } else if (currentLeaf?.kind === "paragraph" && !origBlank
            && !isThematicBreak(lineContent)
            && !(indentOf(lineContent, lineBaseCol) < 4 && lineContent.trimStart().match(/^(`{3,}|~{3,})/))
            && !parseAtxHeading(lineContent)) {
          // Lazy continuation of paragraph inside blockquote — but NOT when a
          // line would start a new block construct (list marker, thematic break,
          // ATX heading, fenced code opener).
          const lm = parseListMarker(lineContent);
          const lmBlankStart = lm ? isBlank(lineContent.slice(lm.markerLen)) : false;
          if (!lm || lmBlankStart) {
            // Lazily continue — append this line to the paragraph
            newContainers.push(container);
            containerIdx++;
            lazyParagraphContinuation = true;
            break; // stop deeper continuation checks
          }
          // Non-empty list marker can interrupt → blockquote ends here
          break;
        } else {
          break;
        }
      } else if (container.kind === "list") {
        // Lists themselves pass through — tightness/continuation is determined
        // by whether the contained list items continue.
        newContainers.push(container);
        containerIdx++;
      } else if (container.kind === "list_item") {
        const item = container as MutableListItem;
        // A line is blank for continuation purposes if the raw line was blank
        // OR if it became blank after stripping outer container markers (e.g. a
        // line `>>` inside a doubly-nested blockquote). This fixes blank-line
        // propagation into list items that live inside blockquotes.
        const effectiveBlank = origBlank || isBlank(lineContent);
        const indent = indentOf(lineContent, lineBaseCol);
        if (!effectiveBlank && indent >= item.contentIndent) {
          [lineContent, lineBaseCol] = stripIndent(lineContent, item.contentIndent, lineBaseCol);
          newContainers.push(container);
          containerIdx++;
        } else if (effectiveBlank) {
          // Blank lines continue list items only when the item already has content.
          // A blank-start item (no content yet) is ended by a blank line.
          if (item.children.length > 0 || (currentLeaf !== null && item === openContainers[containerIdx])) {
            newContainers.push(container);
            containerIdx++;
          } else {
            break; // blank-start item: blank line closes it
          }
        } else if (currentLeaf?.kind === "paragraph" && !origBlank
            && !isThematicBreak(lineContent)
            && !parseListMarker(lineContent)
            && !(indentOf(lineContent, lineBaseCol) < 4 && lineContent.trimStart().match(/^(`{3,}|~{3,})/))
            && !parseAtxHeading(lineContent)) {
          // Lazy paragraph continuation for list items (CommonMark §5.4):
          // a line that doesn't meet the content indent can still lazily continue
          // a paragraph, UNLESS it would start a new block (thematic break, ATX
          // heading, fenced code, list marker — any list marker ends the item).
          newContainers.push(container);
          containerIdx++;
          lazyParagraphContinuation = true;
          break;
        } else {
          break;
        }
      } else {
        break;
      }
    }

    // Save the previous innermost container before updating
    const prevInnerContainer = openContainers[openContainers.length - 1]!;
    openContainers = newContainers;

    // After stripping container markers, the remaining lineContent may now be
    // blank (e.g. `> ` with no content after). Re-check blank status.
    let blank = origBlank;
    if (!blank && isBlank(lineContent)) {
      blank = true;
    }

    // ── Container exit cleanup ──────────────────────────────────────────
    //
    // If the innermost container changed (we left a blockquote or list item),
    // we must finalize any open leaf block in the old container. This prevents
    // a paragraph or code block from leaking across container boundaries.
    // It also prevents setext underlines from misinterpreting a paragraph that
    // was inside a blockquote we just exited.
    const currentInnerAfterContinuation = openContainers[openContainers.length - 1]!;

    // ── Multi-line block continuation ────────────────────────────────────
    //
    // Fenced code and HTML blocks are handled AFTER container continuation so
    // that lineContent already has container markers stripped.

    // If we're inside a fenced code block, accumulate lines
    if (modal.currentMode === "fenced" && currentLeaf?.kind === "fenced_code") {
      const fence = currentLeaf as MutableFencedCode;
      // Check if the container that held the fence is still active
      if (currentInnerAfterContinuation !== prevInnerContainer) {
        // The fenced code's container was dropped — force-close the fence
        fence.closed = true;
        modal.switchMode("exit_fenced");
        currentLeaf = null;
        // Fall through to normal block processing below
      } else {
        const stripped = lineContent.trimStart();
        // Does this line close the fence?
        // A closing fence may have at most 3 leading spaces (spec §4.5).
        const closingFenceRe = new RegExp(
          `^${fence.fence[0] === "`" ? "`" : "~"}{${fence.fenceLen},}\\s*$`
        );
        if (indentOf(lineContent, lineBaseCol) < 4 && stripped.match(closingFenceRe) && !stripped.startsWith(fence.fence[0] === "`" ? "~" : "`")) {
          fence.closed = true;
          modal.switchMode("exit_fenced");
          currentLeaf = null;
        } else {
          // Strip the fence's base indentation (0-3 spaces) from each content line
          const [fenceLine] = stripIndent(lineContent, fence.baseIndent, lineBaseCol);
          fence.lines.push(fenceLine);
        }
        lastLineWasBlank = origBlank;
        continue;
      }
    }

    // If we're inside an HTML block, accumulate lines
    if (modal.currentMode === "html_block" && currentLeaf?.kind === "html_block") {
      const htmlBlock = currentLeaf as MutableHtmlBlock;
      if (currentInnerAfterContinuation !== prevInnerContainer) {
        // Container was dropped — force-close the HTML block
        htmlBlock.closed = true;
        modal.switchMode("exit_html");
        currentLeaf = null;
        // Fall through to normal block processing below
      } else {
        htmlBlock.lines.push(lineContent);
        if (htmlBlockEnds(lineContent, htmlBlock.htmlType)) {
          htmlBlock.closed = true;
          modal.switchMode("exit_html");
          currentLeaf = null;
        }
        lastLineWasBlank = origBlank;
        continue;
      }
    }

    // Finalize the current leaf if we left its container.
    // Skip when lazyParagraphContinuation is true — the paragraph is being
    // lazily continued from outside the container, not abandoned.
    if (currentInnerAfterContinuation !== prevInnerContainer && currentLeaf !== null
        && !lazyParagraphContinuation) {
      finalizeBlock(currentLeaf, prevInnerContainer, linkRefs);
      currentLeaf = null;
    }

    // ── Lazy paragraph continuation ─────────────────────────────────────
    //
    // A line that lazily continues a blockquote paragraph is appended as-is
    // without any block detection (no setext headings, no ATX, etc.).
    if (lazyParagraphContinuation && currentLeaf?.kind === "paragraph") {
      (currentLeaf as MutableParagraph).lines.push(lineContent);
      lastLineWasBlank = false;
      continue;
    }

    // If the innermost container is a list (without a currently open item) and
    // this is not a blank line, check whether the line will start a new item
    // in that list. If not, close the list and pop back to the parent container.
    // Exception: thematic breaks (e.g. `* * *`) should close the list even if
    // they match the list's marker character.
    while (!blank && openContainers.length > 1 &&
           openContainers[openContainers.length - 1]?.kind === "list") {
      const list = openContainers[openContainers.length - 1] as MutableList;
      const marker = parseListMarker(lineContent);
      if (marker && list.ordered === marker.ordered && list.marker === marker.marker
          && !isThematicBreak(lineContent)) {
        break; // will add a new item to this list
      }
      openContainers.pop();
    }

    // Get the innermost container
    let innerContainer = openContainers[openContainers.length - 1]!;

    // ── Blank line handling ───────────────────────────────────────────────

    if (blank) {
      // Blank line closes the current leaf block
      if (currentLeaf?.kind === "paragraph") {
        finalizeBlock(currentLeaf, innerContainer, linkRefs);
        currentLeaf = null;
      } else if (currentLeaf?.kind === "indented_code") {
        // Blank lines inside indented code are preserved with their stripped
        // indentation content (e.g. 6 spaces → 2 spaces after stripping 4).
        const [blankCodeLine] = stripIndent(rawLine, 4);
        (currentLeaf as MutableIndentedCode).lines.push(blankCodeLine);
      }

      // Mark blank line for list tightness tracking
      if (innerContainer.kind === "list_item") {
        (innerContainer as MutableListItem).hadBlankLine = true;
      }
      if (innerContainer.kind === "list") {
        (innerContainer as MutableList).hadBlankLine = true;
      }

      lastLineWasBlank = true;
      lastBlankInnerContainer = innerContainer;
      continue;
    }

    // ── New block detection ───────────────────────────────────────────────
    //
    // Wrapped in a labeled loop so that blockquote detection can update
    // innerContainer and re-dispatch without using recursion or goto.

    blockDetect: while (true) {

    // After a blank line in a list, any new content makes the list loose.
    // Only trigger when the blank was at list/list_item level — blanks inside
    // nested sub-containers (e.g. blockquote inside list item) must NOT make
    // the outer list loose.
    if (lastLineWasBlank && innerContainer.kind === "list"
        && (lastBlankInnerContainer.kind === "list"
            || lastBlankInnerContainer.kind === "list_item")) {
      (innerContainer as MutableList).tight = false;
    }

    // When content resumes after a blank line inside a list item (e.g. a sub-list
    // followed by more content), mark the list item as having had a blank line.
    // This propagates tightness correctly for items with multiple block children.
    if (lastLineWasBlank && innerContainer.kind === "list_item") {
      (innerContainer as MutableListItem).hadBlankLine = true;
    }

    const indent = indentOf(lineContent, lineBaseCol);

    // 1. Fenced code block opener
    const fenceMatch = lineContent.trimStart().match(/^(`{3,}|~{3,})/);
    if (fenceMatch && indent < 4) {
      const fenceChar = fenceMatch[1]![0]!;
      const fenceLen = fenceMatch[1]!.length;
      const infoLine = lineContent.trimStart().slice(fenceLen);
      const infoString = extractInfoString(lineContent);

      // Backtick fences cannot have backticks in info string
      if (fenceChar === "`" && infoLine.includes("`")) {
        // fall through to paragraph handling
      } else {
        closeParagraph(currentLeaf, innerContainer, linkRefs);
        currentLeaf = null;

        const fencedBlock: MutableFencedCode = {
          kind: "fenced_code",
          fence: fenceChar.repeat(fenceLen),
          fenceLen,
          baseIndent: indent,  // strip this many leading spaces from content lines
          infoString,
          lines: [],
          closed: false,
        };
        addChild(innerContainer, fencedBlock);
        currentLeaf = fencedBlock;
        modal.process("content");
        modal.switchMode("enter_fenced");
        lastLineWasBlank = false;
        break blockDetect;
      }
    }

    // 2. ATX heading
    if (indent < 4) {
      const heading = parseAtxHeading(lineContent);
      if (heading) {
        closeParagraph(currentLeaf, innerContainer, linkRefs);
        currentLeaf = null;

        const headingBlock: MutableHeading = {
          kind: "heading",
          level: heading.level,
          content: heading.content,
        };
        addChild(innerContainer, headingBlock);
        currentLeaf = null; // headings are single-line
        lastLineWasBlank = false;
        break blockDetect;
      }
    }

    // 3. Thematic break (must check before list marker to avoid --- confusion)
    if (indent < 4 && isThematicBreak(lineContent)) {
      // BUT: if we're in a paragraph, --- might be a setext heading underline
      if (currentLeaf?.kind === "paragraph") {
        const level = isSetextUnderline(lineContent);
        if (level !== null) {
          const para = currentLeaf as MutableParagraph;
          // Extract any link reference definitions first (they come before heading content)
          finalizeBlock(para, innerContainer, linkRefs);
          if (para.lines.length > 0) {
            const headingBlock: MutableHeading = {
              kind: "heading",
              level: level as 1 | 2,
              content: para.lines.join("\n").trim(),
            };
            removeLastChild(innerContainer);
            addChild(innerContainer, headingBlock);
            currentLeaf = null;
            lastLineWasBlank = false;
            break blockDetect;
          }
          // All content was link defs — para is now empty. Fall through to thematic break.
          removeLastChild(innerContainer);
          currentLeaf = null;
        }
      }

      closeParagraph(currentLeaf, innerContainer, linkRefs);
      currentLeaf = null;
      addChild(innerContainer, { kind: "thematic_break" } as MutableThematicBreak);
      lastLineWasBlank = false;
      break blockDetect;
    }

    // 4. Setext heading underline (when no thematic break matched)
    if (indent < 4 && currentLeaf?.kind === "paragraph") {
      const level = isSetextUnderline(lineContent);
      if (level !== null) {
        const para = currentLeaf as MutableParagraph;
        // Extract any link reference definitions first
        finalizeBlock(para, innerContainer, linkRefs);
        if (para.lines.length > 0) {
          const headingBlock: MutableHeading = {
            kind: "heading",
            level: level as 1 | 2,
            content: para.lines.join("\n").trim(),
          };
          removeLastChild(innerContainer);
          addChild(innerContainer, headingBlock);
          currentLeaf = null;
          lastLineWasBlank = false;
          break blockDetect;
        }
        // All content was link defs — para is empty. Fall through to new para.
        removeLastChild(innerContainer);
        currentLeaf = null;
      }
    }

    // 5. HTML block
    if (indent < 4) {
      const htmlType = detectHtmlBlockType(lineContent);
      if (htmlType !== null) {
        // Type 7 cannot interrupt a paragraph
        if (htmlType !== 7 || currentLeaf?.kind !== "paragraph") {
          closeParagraph(currentLeaf, innerContainer, linkRefs);
          currentLeaf = null;

          const htmlBlock: MutableHtmlBlock = {
            kind: "html_block",
            htmlType,
            lines: [lineContent],
            closed: htmlBlockEnds(lineContent, htmlType),
          };
          addChild(innerContainer, htmlBlock);

          if (!htmlBlock.closed) {
            currentLeaf = htmlBlock;
            modal.process("content");
            modal.switchMode("enter_html");
          }
          lastLineWasBlank = false;
          break blockDetect;
        }
      }
    }

    // 6. Blockquote
    if (indent < 4 && lineContent.trimStart().startsWith(">")) {
      closeParagraph(currentLeaf, innerContainer, linkRefs);
      currentLeaf = null;

      let bq: MutableBlockquote;
      // Continue an existing blockquote only if no blank line intervened.
      // A blank line between "> ..." lines produces separate blockquotes.
      const bqLast = lastChild(innerContainer);
      if (bqLast?.kind === "blockquote" && !lastLineWasBlank) {
        bq = bqLast as MutableBlockquote;
      } else {
        bq = { kind: "blockquote", children: [] };
        addChild(innerContainer, bq);
      }

      openContainers.push(bq);
      // Strip the > marker (and optional following space/tab) with tab-aware
      // virtual-column arithmetic (CommonMark §2.1):
      //   - Up to 3 leading spaces, then `>`
      //   - Optional separator: a space (consumed) or a tab (1 virtual space
      //     consumed as the separator; any leftover expansion spaces prepended
      //     to the content).
      {
        let bqI = 0;
        let bqCol = lineBaseCol;
        while (bqI < lineContent.length && lineContent[bqI] === " " && bqI - 0 < 3) {
          bqI++; bqCol++;
        }
        if (bqI < lineContent.length && lineContent[bqI] === ">") {
          bqI++; bqCol++;
          if (bqI < lineContent.length) {
            if (lineContent[bqI] === " ") {
              bqI++; bqCol++;
            } else if (lineContent[bqI] === "\t") {
              const w = 4 - (bqCol % 4);
              bqI++;
              if (w > 1) {
                // Tab provides w virtual spaces; 1 used as separator, w-1 leftover.
                lineContent = " ".repeat(w - 1) + lineContent.slice(bqI);
                lineBaseCol = bqCol + 1;
                innerContainer = bq;
                if (isBlank(lineContent)) { break blockDetect; }
                continue blockDetect;
              }
              bqCol += w; // w === 1: tab exactly consumed as separator
            }
          }
        }
        lineContent = lineContent.slice(bqI);
        lineBaseCol = bqCol;
      }
      innerContainer = bq;

      // If content after stripping is blank, handle as blank inside the blockquote
      if (isBlank(lineContent)) {
        // No content in this blockquote line — just a blank
        lastLineWasBlank = false; // the > line itself isn't a blank-between-blocks
        break blockDetect;
      }

      // Re-dispatch block detection within the blockquote context.
      // This allows lists, fenced code, and other blocks inside blockquotes.
      continue blockDetect;
    }

    // 7. List item
    if (indent < 4) {
      const marker = parseListMarker(lineContent);
      if (marker !== null) {
        // Check if this continues an existing list or starts a new one.
        // The inner container may already BE the list (after a blank line
        // consumed by the list pass-through in the continuation loop).
        let list: MutableList | null = null;

        if (innerContainer.kind === "list") {
          // We're already inside the list — continue it directly.
          const existingList = innerContainer as MutableList;
          if (existingList.ordered === marker.ordered &&
              existingList.marker === marker.marker) {
            list = existingList;
          }
        }
        if (list === null) {
          const listLast = lastChild(innerContainer);
          if (listLast?.kind === "list") {
            const existingList = listLast as MutableList;
            // Same marker type continues the list
            if (existingList.ordered === marker.ordered &&
                existingList.marker === marker.marker) {
              list = existingList;
            }
          }
        }

        // Compute the virtual column of itemContent[0] (after the full marker).
        // This is needed for correct tab arithmetic in continued block detection.
        let newLineBaseCol = virtualColAfter(lineContent, marker.markerLen, lineBaseCol);
        let itemContent = lineContent.slice(marker.markerLen);

        // Handle tab separator (CommonMark §2.1): when the separator after the
        // list marker is a tab, that tab expands to `w` virtual spaces relative
        // to the column where it appears. Only 1 of those virtual spaces is the
        // required separator; the remaining `w-1` spaces are prepended to the
        // item content so subsequent indentation checks are correct.
        //
        // Example: `-\t\tfoo` — the first `\t` is the separator at col 1,
        //   expands to 3 virtual spaces; 1 used as separator, 2 prepended to
        //   `\tfoo`, giving `"  \tfoo"` at lineBaseCol 2.
        if (marker.spaceAfter === 1) {
          const sepChar = lineContent[marker.markerLen - 1];
          if (sepChar === "\t") {
            const sepCol = virtualColAfter(lineContent, marker.markerLen - 1, lineBaseCol);
            const w = 4 - (sepCol % 4);
            if (w > 1) {
              itemContent = " ".repeat(w - 1) + itemContent;
              newLineBaseCol = sepCol + 1;
            }
          }
        }

        const blankStart = isBlank(itemContent);

        // Empty list items (blank start) cannot interrupt a paragraph to start
        // a NEW list. Continuing an existing list is always allowed.
        // Also: ordered lists starting ≠ 1 cannot interrupt to start a new list.
        const paraInCurrentContainer =
          currentLeaf?.kind === "paragraph" &&
          lastChild(innerContainer) === currentLeaf;
        const canInterruptPara = (!marker.ordered || marker.start === 1 || list !== null)
          && (!blankStart || !paraInCurrentContainer);

        if (currentLeaf?.kind !== "paragraph" || canInterruptPara) {
          if (list === null) {
            closeParagraph(currentLeaf, innerContainer, linkRefs);
            currentLeaf = null;
            list = {
              kind: "list",
              ordered: marker.ordered,
              marker: marker.marker,
              start: marker.start,
              tight: true,
              items: [],
              hadBlankLine: false,
            };
            addChild(innerContainer, list);
          } else {
            closeParagraph(currentLeaf, innerContainer, linkRefs);
            currentLeaf = null;
            // If there was a blank line between items, the list is loose.
            // Only count blank lines at list/list_item level (not nested deeper).
            if (list.hadBlankLine
                || (lastLineWasBlank
                    && (lastBlankInnerContainer.kind === "list"
                        || lastBlankInnerContainer.kind === "list_item"))) {
              list.tight = false;
            }
            list.hadBlankLine = false;
          }

          // Compute content indent (W+1 rule):
          // - For blank-start items: use W+1 = markerLen - spaceAfter + 1
          // - For spaceAfter >= 5: also use W+1 (the extra spaces become code indent)
          // - Normal case: use full markerLen
          const normalIndent = marker.markerLen;
          const reducedIndent = marker.markerLen - marker.spaceAfter + 1;
          const contentIndent = (blankStart || marker.spaceAfter >= 5) ? reducedIndent : normalIndent;

          const item: MutableListItem = {
            kind: "list_item",
            marker: marker.marker,
            markerIndent: marker.indent,
            contentIndent,
            children: [],
            hadBlankLine: false,
          };
          list.items.push(item);
          // Push the list only if it's not already the inner container
          if (innerContainer !== list) {
            openContainers.push(list);
          }
          openContainers.push(item);

          if (!blankStart) {
            // Process the item's first-line content within the item's context.
            // When spaceAfter >= 5, restore (spaceAfter - 1) leading spaces so
            // the indented-code detection threshold (4 spaces) is computed
            // relative to the item's content indent (W+1), not the full marker width.
            innerContainer = item;
            if (marker.spaceAfter >= 5) {
              // W+1 rule: virtual col starts at marker end minus the extra spaces
              lineBaseCol = virtualColAfter(lineContent, marker.markerLen - marker.spaceAfter + 1, lineBaseCol);
              lineContent = " ".repeat(marker.spaceAfter - 1) + itemContent;
            } else {
              lineBaseCol = newLineBaseCol;
              lineContent = itemContent;
            }
            continue blockDetect;
          }
          currentLeaf = null;
          lastLineWasBlank = false;
          break blockDetect;
        }
      }
    }

    // 8. Indented code block (4+ spaces, but NOT inside a paragraph)
    if (indent >= 4 && currentLeaf?.kind !== "paragraph") {
      const [stripped] = stripIndent(lineContent, 4, lineBaseCol);
      if (currentLeaf?.kind === "indented_code") {
        const icb = currentLeaf as MutableIndentedCode;
        // Do NOT remove trailing blank lines here — they may be followed by
        // more code. Trailing blank removal happens only at finalization.
        icb.lines.push(stripped);
      } else {
        closeParagraph(currentLeaf, innerContainer, linkRefs);
        const icb: MutableIndentedCode = {
          kind: "indented_code",
          lines: [stripped],
        };
        addChild(innerContainer, icb);
        currentLeaf = icb;
      }
      lastLineWasBlank = false;
      break blockDetect;
    }

    // 9. Paragraph continuation or new paragraph
    // Note: we do NOT trimEnd() here — trailing spaces are significant for
    // hard line break detection in the inline parser (two or more trailing
    // spaces before a newline produce a <br>).
    if (currentLeaf?.kind === "paragraph") {
      (currentLeaf as MutableParagraph).lines.push(lineContent);
    } else {
      closeParagraph(currentLeaf, innerContainer, linkRefs);
      const para: MutableParagraph = {
        kind: "paragraph",
        lines: [lineContent],
      };
      addChild(innerContainer, para);
      currentLeaf = para;
    }

    lastLineWasBlank = false;
    break blockDetect;
    } // end blockDetect: while (true)
  }

  // Finalize any remaining open leaf block
  if (currentLeaf !== null) {
    const innerContainer = openContainers[openContainers.length - 1]!;
    finalizeBlock(currentLeaf, innerContainer, linkRefs);
  }

  // Force-close any open fenced code block
  if (modal.currentMode === "fenced") {
    modal.switchMode("exit_fenced");
  }
  if (modal.currentMode === "html_block") {
    modal.switchMode("exit_html");
  }

  return { document: root, linkRefs };
}

// ─── Container Helpers ────────────────────────────────────────────────────────

function lastChild(container: MutableBlock): MutableBlock | null {
  if (container.kind === "document") return (container as MutableDocument).children.at(-1) ?? null;
  if (container.kind === "blockquote") return (container as MutableBlockquote).children.at(-1) ?? null;
  if (container.kind === "list_item") return (container as MutableListItem).children.at(-1) ?? null;
  return null;
}

function addChild(container: MutableBlock, block: MutableBlock): void {
  if (container.kind === "document") (container as MutableDocument).children.push(block);
  else if (container.kind === "blockquote") (container as MutableBlockquote).children.push(block);
  else if (container.kind === "list_item") (container as MutableListItem).children.push(block);
}

function removeLastChild(container: MutableBlock): void {
  if (container.kind === "document") (container as MutableDocument).children.pop();
  else if (container.kind === "blockquote") (container as MutableBlockquote).children.pop();
  else if (container.kind === "list_item") (container as MutableListItem).children.pop();
}

function closeParagraph(
  leaf: MutableBlock | null,
  container: MutableBlock,
  linkRefs: LinkRefMap,
): void {
  if (leaf?.kind === "paragraph") {
    finalizeBlock(leaf, container, linkRefs);
  } else if (leaf?.kind === "indented_code") {
    // Trim trailing blank/whitespace-only lines from indented code blocks
    const icb = leaf as MutableIndentedCode;
    while (icb.lines.length > 0 && /^\s*$/.test(icb.lines[icb.lines.length - 1]!)) {
      icb.lines.pop();
    }
  }
}

function finalizeBlock(
  block: MutableBlock,
  _container: MutableBlock,
  linkRefs: LinkRefMap,
): void {
  if (block.kind === "paragraph") {
    // Attempt to extract link reference definitions from the paragraph
    const para = block as MutableParagraph;
    let text = para.lines.join("\n");
    while (true) {
      const def = parseLinkDefinition(text);
      if (!def) break;
      const key = def.label;
      if (!linkRefs.has(key)) {
        linkRefs.set(key, { destination: def.destination, title: def.title });
      }
      text = text.slice(def.charsConsumed);
    }
    // Update paragraph lines with remaining text
    if (text.trim() === "") {
      para.lines = [];
    } else {
      // Preserve trailing spaces — they are significant for inline hard breaks.
      // Only strip a trailing newline from the very last line.
      para.lines = text.split("\n");
      if (para.lines.length > 0) {
        para.lines[para.lines.length - 1] = para.lines[para.lines.length - 1]!.trimEnd();
      }
    }
  } else if (block.kind === "indented_code") {
    // Trim trailing blank lines — these are buffered during blank-line handling
    // but should not appear in the final code block value.
    const icb = block as MutableIndentedCode;
    while (icb.lines.length > 0 && icb.lines[icb.lines.length - 1] === "") {
      icb.lines.pop();
    }
  }
}

/** Process a single line of content inside a container (for blockquotes/lists). */
function processLineInContainer(
  lineContent: string,
  container: MutableBlock,
  currentLeaf: MutableBlock | null,
  linkRefs: LinkRefMap,
  lastLineWasBlank: boolean,
): { currentLeaf: MutableBlock | null } {
  if (isBlank(lineContent)) {
    if (currentLeaf?.kind === "paragraph") {
      finalizeBlock(currentLeaf, container, linkRefs);
      currentLeaf = null;
    }
    return { currentLeaf: null };
  }

  const indent = indentOf(lineContent);

  // ATX heading
  if (indent < 4) {
    const heading = parseAtxHeading(lineContent);
    if (heading) {
      closeParagraph(currentLeaf, container, linkRefs);
      const h: MutableHeading = { kind: "heading", level: heading.level, content: heading.content };
      addChild(container, h);
      return { currentLeaf: null };
    }
  }

  // Thematic break
  if (indent < 4 && isThematicBreak(lineContent)) {
    if (currentLeaf?.kind === "paragraph") {
      const level = isSetextUnderline(lineContent);
      if (level !== null) {
        const para = currentLeaf as MutableParagraph;
        const h: MutableHeading = { kind: "heading", level: level as 1 | 2, content: para.lines.join("\n").trim() };
        removeLastChild(container);
        addChild(container, h);
        return { currentLeaf: null };
      }
    }
    closeParagraph(currentLeaf, container, linkRefs);
    addChild(container, { kind: "thematic_break" } as MutableThematicBreak);
    return { currentLeaf: null };
  }

  // Paragraph continuation
  if (currentLeaf?.kind === "paragraph") {
    (currentLeaf as MutableParagraph).lines.push(lineContent);
    return { currentLeaf };
  }

  // New paragraph
  closeParagraph(currentLeaf, container, linkRefs);
  const para: MutableParagraph = { kind: "paragraph", lines: [lineContent] };
  addChild(container, para);
  return { currentLeaf: para };
}

// ─── AST Conversion ───────────────────────────────────────────────────────────
//
// Convert mutable intermediate blocks into the final readonly AST types.
// Inline content (raw strings in paragraphs and headings) is returned as-is
// for Phase 2 (inline-parser.ts) to process.

export interface BlockParseResult {
  document: DocumentNode;
  linkRefs: LinkRefMap;
  /** Raw inline content strings, keyed by a unique id, for Phase 2. */
  rawInlineContent: Map<symbol, string>;
}

/**
 * Convert the mutable intermediate document into the final AST.
 * Inline content is NOT yet parsed — raw strings are stored in
 * `rawInlineContent` for the inline parser to process.
 */
export function convertToAst(
  mutableDoc: MutableDocument,
  linkRefs: LinkRefMap,
): BlockParseResult {
  const rawInlineContent = new Map<symbol, string>();

  function convertBlock(block: MutableBlock): BlockNode {
    switch (block.kind) {
      case "document":
        return {
          type: "document",
          children: (block as MutableDocument).children.map(convertBlock).filter(Boolean) as BlockNode[],
        } as DocumentNode;

      case "heading": {
        const h = block as MutableHeading;
        const id = Symbol();
        rawInlineContent.set(id, h.content);
        return { type: "heading", level: h.level, children: [], _rawId: id } as HeadingNode & { _rawId: symbol };
      }

      case "paragraph": {
        const p = block as MutableParagraph;
        if (p.lines.length === 0) return null as unknown as BlockNode;
        // Strip leading whitespace from each line per the CommonMark spec.
        // (A paragraph's raw content strips initial spaces/tabs from each line.)
        // Trailing spaces are preserved — they signal hard line breaks (  \n).
        const content = p.lines.map(l => l.replace(/^[ \t]+/, "")).join("\n");
        const id = Symbol();
        rawInlineContent.set(id, content);
        return { type: "paragraph", children: [], _rawId: id } as ParagraphNode & { _rawId: symbol };
      }

      case "fenced_code": {
        const fc = block as MutableFencedCode;
        return {
          type: "code_block",
          language: fc.infoString || null,
          value: fc.lines.join("\n") + (fc.lines.length > 0 ? "\n" : ""),
        } as CodeBlockNode;
      }

      case "indented_code": {
        const ic = block as MutableIndentedCode;
        return {
          type: "code_block",
          language: null,
          value: ic.lines.join("\n") + "\n",
        } as CodeBlockNode;
      }

      case "blockquote": {
        const bq = block as MutableBlockquote;
        return {
          type: "blockquote",
          children: bq.children.map(convertBlock).filter(Boolean) as BlockNode[],
        } as BlockquoteNode;
      }

      case "list": {
        const list = block as MutableList;
        // A list is loose if:
        //   - blank lines appeared between items (list.tight was set false), OR
        //   - blank lines appeared between blocks within an item that has > 1 block.
        // An item with hadBlankLine but only ONE block child is still "tight"
        // (the blank line was after the item, not between its own blocks).
        const isTight = list.tight && !list.hadBlankLine &&
          !list.items.some(i => i.hadBlankLine && i.children.length > 1);

        return {
          type: "list",
          ordered: list.ordered,
          start: list.ordered ? list.start : null,
          tight: isTight,
          children: list.items.map(item => convertBlock(item) as ListItemNode),
        } as ListNode;
      }

      case "list_item": {
        const item = block as MutableListItem;
        return {
          type: "list_item",
          children: item.children.map(convertBlock).filter(Boolean) as BlockNode[],
        } as ListItemNode;
      }

      case "thematic_break":
        return { type: "thematic_break" } as ThematicBreakNode;

      case "html_block": {
        const hb = block as MutableHtmlBlock;
        // For type 6/7 blocks, a blank line terminates the block and gets
        // pushed into hb.lines before the mode switches. Trim it.
        const lines = [...hb.lines];
        while (lines.length > 0 && lines[lines.length - 1]!.trim() === "") {
          lines.pop();
        }
        // Document AST: raw HTML passes through as RawBlockNode { format: "html" }
        return {
          type: "raw_block",
          format: "html",
          value: lines.join("\n") + "\n",
        } as RawBlockNode;
      }

      case "link_def":
        // Link definitions are Markdown parse artifacts. They have already been
        // resolved into linkRefs during Phase 1 and are NOT emitted into the
        // Document AST IR (which only contains fully resolved LinkNode values).
        return null as unknown as BlockNode;

      default:
        return null as unknown as BlockNode;
    }
  }

  const document = convertBlock(mutableDoc) as DocumentNode;
  return { document, linkRefs, rawInlineContent };
}

type RawInlineCarrier = { _rawId?: symbol };

/**
 * Apply GFM-specific block transforms after CommonMark block parsing but
 * before inline parsing. This keeps the original CommonMark parser untouched
 * while letting the GFM front-end upgrade paragraphs/list items into richer
 * Document AST nodes.
 */
export function applyGfmBlockExtensions(
  document: DocumentNode,
  rawInlineContent: Map<symbol, string>,
): DocumentNode {
  function transformBlocks(blocks: readonly BlockNode[]): BlockNode[] {
    return blocks.map(transformBlock);
  }

  function transformBlock(block: BlockNode): BlockNode {
    switch (block.type) {
      case "document":
        return { type: "document", children: transformBlocks(block.children) };

      case "blockquote":
        return { type: "blockquote", children: transformBlocks(block.children) };

      case "list":
        return {
          type: "list",
          ordered: block.ordered,
          start: block.start,
          tight: block.tight,
          children: block.children.map(transformListChild),
        };

      case "list_item":
      case "task_item":
        return transformListChild(block);

      case "paragraph": {
        const table = maybeTransformParagraphToTable(block, rawInlineContent);
        return table ?? block;
      }

      default:
        return block;
    }
  }

  function transformListChild(item: ListItemNode | TaskItemNode): ListItemNode | TaskItemNode {
    const children = transformBlocks(item.children);
    const nextItem = item.type === "task_item"
      ? { type: "task_item", checked: item.checked, children } as TaskItemNode
      : { type: "list_item", children } as ListItemNode;

    const first = children[0];
    if (first?.type !== "paragraph") {
      return nextItem;
    }

    const rawId = (first as ParagraphNode & RawInlineCarrier)._rawId;
    if (rawId === undefined) {
      return nextItem;
    }

    const raw = rawInlineContent.get(rawId);
    if (raw === undefined) {
      return nextItem;
    }

    const match = raw.match(/^\[( |x|X)\](?:[ \t]+|$)/);
    if (match === null) {
      return nextItem;
    }

    const checked = match[1]!.toLowerCase() === "x";
    rawInlineContent.set(rawId, raw.slice(match[0].length));
    return { type: "task_item", checked, children };
  }

  (document as { children: BlockNode[] }).children = transformBlocks(document.children);
  return document;
}

function maybeTransformParagraphToTable(
  block: ParagraphNode,
  rawInlineContent: Map<symbol, string>,
): TableNode | null {
  const rawId = (block as ParagraphNode & RawInlineCarrier)._rawId;
  if (rawId === undefined) {
    return null;
  }

  const raw = rawInlineContent.get(rawId);
  if (raw === undefined) {
    return null;
  }

  const parsed = tryParseTable(raw);
  if (parsed === null) {
    return null;
  }

  const makeCell = (content: string): TableCellNode => {
    const cellRawId = Symbol();
    rawInlineContent.set(cellRawId, content);
    return { type: "table_cell", children: [], _rawId: cellRawId } as TableCellNode & RawInlineCarrier;
  };

  const makeRow = (cells: string[], isHeader: boolean): TableRowNode => ({
    type: "table_row",
    isHeader,
    children: cells.map(makeCell),
  });

  rawInlineContent.delete(rawId);

  return {
    type: "table",
    align: parsed.align,
    children: [
      makeRow(parsed.header, true),
      ...parsed.rows.map((row) => makeRow(row, false)),
    ],
  };
}

function tryParseTable(raw: string): { align: TableAlignment[]; header: string[]; rows: string[][] } | null {
  const lines = raw.split("\n");
  if (lines.length < 2) {
    return null;
  }

  const headerCells = splitTableRow(lines[0]!);
  const delimiterCells = splitTableRow(lines[1]!);
  if (headerCells === null || delimiterCells === null) {
    return null;
  }
  if (headerCells.length === 0 || headerCells.length !== delimiterCells.length) {
    return null;
  }

  const align: TableAlignment[] = [];
  for (const cell of delimiterCells) {
    const trimmed = cell.trim();
    if (!/^:?-{3,}:?$/.test(trimmed)) {
      return null;
    }
    const left = trimmed.startsWith(":");
    const right = trimmed.endsWith(":");
    align.push(left && right ? "center" : left ? "left" : right ? "right" : null);
  }

  const rows: string[][] = [];
  for (const line of lines.slice(2)) {
    if (line.trim() === "") {
      return null;
    }
    const cells = splitTableRow(line);
    if (cells === null) {
      return null;
    }
    rows.push(normalizeTableRow(cells, headerCells.length));
  }

  return {
    align,
    header: normalizeTableRow(headerCells, delimiterCells.length),
    rows,
  };
}

function splitTableRow(line: string): string[] | null {
  if (!line.includes("|")) {
    return null;
  }

  const cells: string[] = [];
  let current = "";
  let escaped = false;
  let pipeCount = 0;
  let start = 0;
  let end = line.length;
  const trimmedLine = line.trim();
  const hadOuterPipe = trimmedLine.startsWith("|") || trimmedLine.endsWith("|");

  while (start < end && isAsciiWhitespace(line[start]!)) start++;
  while (end > start && isAsciiWhitespace(line[end - 1]!)) end--;
  if (start < end && line[start] === "|") start++;
  if (end > start && line[end - 1] === "|") end--;

  for (let i = start; i < end; i++) {
    const ch = line[i]!;
    if (escaped) {
      current += ch;
      escaped = false;
      continue;
    }
    if (ch === "\\") {
      escaped = true;
      current += ch;
      continue;
    }
    if (ch === "|") {
      pipeCount++;
      cells.push(current.trim());
      current = "";
      continue;
    }
    current += ch;
  }

  cells.push(current.trim());
  return pipeCount > 0 || hadOuterPipe ? cells : null;
}

function normalizeTableRow(cells: string[], width: number): string[] {
  const normalized = cells.slice(0, width);
  while (normalized.length < width) {
    normalized.push("");
  }
  return normalized;
}
