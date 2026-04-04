/**
 * AsciiDoc Block Parser
 *
 * Transforms a raw AsciiDoc string into a DocumentNode AST by processing the
 * source line-by-line through a state machine. The algorithm mirrors the
 * CommonMark block-parsing phase: we build the structural skeleton (headings,
 * code blocks, lists, blockquotes, …) first, then a second pass fills in
 * inline content using the inline parser.
 *
 * === State machine ===
 *
 * The parser maintains a `ParseState` record that tracks the current parsing
 * mode. Possible modes:
 *
 *   normal          — between blocks; ready to start any new block type
 *   paragraph       — collecting lines of a paragraph
 *   code_block      — inside a `----` delimited listing block
 *   literal_block   — inside a `....` delimited literal block
 *   passthrough_block — inside a `++++` passthrough (raw HTML) block
 *   quote_block     — inside a `____` quote block (recursive parse)
 *   unordered_list  — collecting `*` / `**` list items
 *   ordered_list    — collecting `.` / `..` list items
 *
 * Line dispatch (normal mode):
 *
 *   blank line       → stay in normal (clears pending language hint)
 *   // comment       → skip
 *   [source,lang]    → store language hint for the next code block
 *   = text           → HeadingNode level 1  (= through ====== = levels 1–6)
 *   '''              → ThematicBreakNode
 *   ---- (≥4)        → enter code_block mode
 *   .... (≥4)        → enter literal_block mode
 *   ++++ (≥4)        → enter passthrough_block mode
 *   ____ (≥4)        → enter quote_block mode
 *   * text / ** text → enter unordered_list mode
 *   . text / .. text → enter ordered_list mode
 *   other            → enter paragraph mode
 *
 * When leaving a mode that accumulated content, we emit the corresponding
 * block node and transition back to normal.
 *
 * @module block-parser
 */

import type {
  DocumentNode,
  BlockNode,
  HeadingNode,
  ParagraphNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  ThematicBreakNode,
  RawBlockNode,
} from "@coding-adventures/document-ast";
import { parseInline } from "./inline-parser.js";

// ─── Parse state ──────────────────────────────────────────────────────────────

type ParseMode =
  | "normal"
  | "paragraph"
  | "code_block"
  | "literal_block"
  | "passthrough_block"
  | "quote_block"
  | "unordered_list"
  | "ordered_list";

/** Mutable internal state threaded through the line-dispatch loop. */
interface ParseState {
  mode: ParseMode;
  /** Language hint set by `[source,lang]` attribute line. */
  pendingLanguage: string | null;
  /** Lines accumulated in the current mode (paragraph lines, code lines, etc.) */
  currentLines: string[];
  /** Raw `{level, text}` pairs accumulated in list mode. */
  listItems: Array<{ level: number; text: string }>;
  /** Completed block nodes (output). */
  blocks: BlockNode[];
}

// ─── Heading detection ────────────────────────────────────────────────────────

/**
 * Test whether a line is an AsciiDoc section heading (Atx style).
 *
 * AsciiDoc headings use `=` characters at the start:
 *
 *   `= Title`    → level 1  (document title)
 *   `== Section` → level 2
 *   `=== Sub`    → level 3
 *   … up to 6
 *
 * Returns `{ level, text }` if the line is a heading, or `null` otherwise.
 */
function matchHeading(line: string): { level: 1 | 2 | 3 | 4 | 5 | 6; text: string } | null {
  const m = /^(={1,6})\s+(.+)$/.exec(line);
  if (!m) return null;
  const level = Math.min(m[1].length, 6) as 1 | 2 | 3 | 4 | 5 | 6;
  return { level, text: m[2] };
}

// ─── Delimiter detection ──────────────────────────────────────────────────────

/** Returns true if the line is an AsciiDoc code-block delimiter (`----` ≥ 4 dashes). */
function isCodeDelim(line: string): boolean {
  return /^-{4,}$/.test(line.trim());
}

/** Returns true if the line is a literal-block delimiter (`....` ≥ 4 dots). */
function isLiteralDelim(line: string): boolean {
  return /^\.{4,}$/.test(line.trim());
}

/** Returns true if the line is a passthrough-block delimiter (`++++`). */
function isPassthroughDelim(line: string): boolean {
  return /^\+{4,}$/.test(line.trim());
}

/** Returns true if the line is a quote-block delimiter (`____` ≥ 4 underscores). */
function isQuoteDelim(line: string): boolean {
  return /^_{4,}$/.test(line.trim());
}

/** Returns true if the line is a thematic break (`'''` 3+ apostrophes/single-quotes). */
function isThematicBreak(line: string): boolean {
  return /^'{3,}$/.test(line.trim());
}

// ─── Source attribute detection ───────────────────────────────────────────────

/**
 * Parse a `[source,lang]` attribute line.
 *
 * AsciiDoc attribute lines are enclosed in `[...]`. The `source` role
 * optionally followed by a comma and a language name sets up the language
 * hint for the next delimited code block.
 *
 * Returns the language string, or null if the line is not a source attribute.
 *
 * Examples:
 *   `[source,typescript]` → "typescript"
 *   `[source]`            → null (no language specified)
 *   `[NOTE]`              → null (not a source attribute)
 */
function matchSourceAttr(line: string): string | null {
  const m = /^\[source(?:,([^\]]+))?\]$/.exec(line.trim());
  if (!m) return null;
  return m[1] ?? null;
}

// ─── List item detection ──────────────────────────────────────────────────────

/**
 * Parse an unordered list item line: `* text` or `** text` etc.
 *
 * The number of `*` characters indicates the nesting level (1 = top level).
 * Returns `{ level, text }` or null.
 *
 * Example:
 *   `* foo`   → { level: 1, text: "foo" }
 *   `** bar`  → { level: 2, text: "bar" }
 */
function matchUnorderedItem(line: string): { level: number; text: string } | null {
  const m = /^(\*+)\s+(.+)$/.exec(line);
  if (!m) return null;
  return { level: m[1].length, text: m[2] };
}

/**
 * Parse an ordered list item line: `. text` or `.. text` etc.
 *
 * In AsciiDoc, ordered lists use `.` characters. One dot = level 1, two = 2.
 *
 * Example:
 *   `. foo`  → { level: 1, text: "foo" }
 *   `.. bar` → { level: 2, text: "bar" }
 */
function matchOrderedItem(line: string): { level: number; text: string } | null {
  const m = /^(\.+)\s+(.+)$/.exec(line);
  if (!m) return null;
  return { level: m[1].length, text: m[2] };
}

// ─── Flush helpers ────────────────────────────────────────────────────────────

/**
 * Flush accumulated paragraph lines into a ParagraphNode and push it to
 * the blocks array. Clears `state.currentLines`.
 */
function flushParagraph(state: ParseState): void {
  if (state.currentLines.length === 0) return;
  const text = state.currentLines.join("\n");
  const para: ParagraphNode = {
    type: "paragraph",
    children: parseInline(text),
  };
  state.blocks.push(para);
  state.currentLines = [];
}

/**
 * Flush accumulated list items into a ListNode.
 *
 * The list items are grouped into a nested structure based on their `level`
 * field. Level 1 items become top-level ListItemNodes. Level 2 items are
 * nested inside the previous level 1 item as a child ListNode.
 *
 * This only supports two levels of nesting for simplicity; deeper nesting
 * uses the same two-level grouping applied recursively.
 */
function flushList(state: ParseState, ordered: boolean): void {
  if (state.listItems.length === 0) return;

  const rootItems: ListItemNode[] = [];
  let currentSubItems: ListItemNode[] = [];
  let lastRootItem: ListItemNode | null = null;

  for (const item of state.listItems) {
    if (item.level === 1) {
      // If we've been collecting sub-items, attach them to the last root item
      if (currentSubItems.length > 0 && lastRootItem !== null) {
        const subList: ListNode = {
          type: "list",
          ordered,
          tight: true,
          start: ordered ? 1 : null,
          children: currentSubItems,
        };
        // We need a mutable reference here — use a cast to rebuild
        const rebuilt: ListItemNode = {
          type: "list_item",
          children: [...lastRootItem.children, subList],
        };
        // Replace the last root item
        rootItems[rootItems.length - 1] = rebuilt;
        currentSubItems = [];
      }
      const li: ListItemNode = {
        type: "list_item",
        children: [
          {
            type: "paragraph",
            children: parseInline(item.text),
          } as ParagraphNode,
        ],
      };
      rootItems.push(li);
      lastRootItem = li;
    } else {
      // Deeper level — collect as sub-item
      const li: ListItemNode = {
        type: "list_item",
        children: [
          {
            type: "paragraph",
            children: parseInline(item.text),
          } as ParagraphNode,
        ],
      };
      currentSubItems.push(li);
    }
  }

  // Attach any remaining sub-items
  if (currentSubItems.length > 0 && lastRootItem !== null) {
    const subList: ListNode = {
      type: "list",
      ordered,
      tight: true,
      start: ordered ? 1 : null,
      children: currentSubItems,
    };
    const rebuilt: ListItemNode = {
      type: "list_item",
      children: [...lastRootItem.children, subList],
    };
    rootItems[rootItems.length - 1] = rebuilt;
  }

  const listNode: ListNode = {
    type: "list",
    ordered,
    tight: true,
    start: ordered ? 1 : null,
    children: rootItems,
  };
  state.blocks.push(listNode);
  state.listItems = [];
}

// ─── Line dispatch ────────────────────────────────────────────────────────────

/**
 * Dispatch a single line in `normal` mode.
 *
 * Checks the line against each block-start pattern in priority order. This
 * is the core of the block parser's state machine. Returns the updated state.
 */
function dispatchNormal(line: string, state: ParseState): ParseState {
  // Blank line — stay in normal
  if (line.trim() === "") {
    state.pendingLanguage = null;
    return state;
  }

  // AsciiDoc comment — skip
  if (line.startsWith("//") && !line.startsWith("///")) {
    return state;
  }

  // Source attribute — store language hint
  const lang = matchSourceAttr(line);
  if (lang !== null) {
    state.pendingLanguage = lang;
    return state;
  }

  // AsciiDoc attribute lines like [NOTE], [WARNING], [source] without comma
  // We treat them as potential attribute hints. If `[source]` without language,
  // set pendingLanguage to null explicitly. Other attributes are skipped.
  if (/^\[.*\]$/.test(line.trim())) {
    // Any attribute block that isn't [source,...] we simply skip
    return state;
  }

  // Heading
  const heading = matchHeading(line);
  if (heading !== null) {
    const node: HeadingNode = {
      type: "heading",
      level: heading.level,
      children: parseInline(heading.text),
    };
    state.blocks.push(node);
    return state;
  }

  // Thematic break
  if (isThematicBreak(line)) {
    const node: ThematicBreakNode = { type: "thematic_break" };
    state.blocks.push(node);
    return state;
  }

  // Code block delimiter
  if (isCodeDelim(line)) {
    state.mode = "code_block";
    state.currentLines = [];
    return state;
  }

  // Literal block delimiter
  if (isLiteralDelim(line)) {
    state.mode = "literal_block";
    state.currentLines = [];
    return state;
  }

  // Passthrough block delimiter
  if (isPassthroughDelim(line)) {
    state.mode = "passthrough_block";
    state.currentLines = [];
    return state;
  }

  // Quote block delimiter
  if (isQuoteDelim(line)) {
    state.mode = "quote_block";
    state.currentLines = [];
    return state;
  }

  // Unordered list item
  const uItem = matchUnorderedItem(line);
  if (uItem !== null) {
    state.mode = "unordered_list";
    state.listItems = [uItem];
    return state;
  }

  // Ordered list item
  const oItem = matchOrderedItem(line);
  if (oItem !== null) {
    state.mode = "ordered_list";
    state.listItems = [oItem];
    return state;
  }

  // Everything else → start a paragraph
  state.mode = "paragraph";
  state.currentLines = [line];
  return state;
}

// ─── Main export: parse ───────────────────────────────────────────────────────

/**
 * Parse an AsciiDoc string into a DocumentNode.
 *
 * The function processes the source text line by line through a state machine.
 * Each line is classified and either initiates a new block, continues the
 * current block, or terminates the current block and starts another.
 *
 * === Two-phase processing ===
 *
 * Phase 1 (this function): Block structure — headings, lists, code blocks,
 *   blockquotes, paragraphs, thematic breaks.
 *
 * Phase 2 (inline-parser.ts): Inline content — emphasis, strong, code spans,
 *   links, images, hard/soft breaks.
 *
 * The inline parser is called during Phase 1 whenever we need to fill the
 * `children` array of a block node (paragraph, heading, list item).
 *
 * @param text  Raw AsciiDoc source string.
 * @returns     Root DocumentNode with all block and inline nodes.
 *
 * @example
 * ```typescript
 * const doc = parse("= Hello\n\nWorld *bold*.\n");
 * doc.children[0].type; // "heading"
 * doc.children[1].type; // "paragraph"
 * ```
 */
export function parse(text: string): DocumentNode {
  const lines = text.split("\n");

  const state: ParseState = {
    mode: "normal",
    pendingLanguage: null,
    currentLines: [],
    listItems: [],
    blocks: [],
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    switch (state.mode) {
      // ── normal mode ────────────────────────────────────────────────────
      case "normal": {
        dispatchNormal(line, state);
        break;
      }

      // ── paragraph mode ─────────────────────────────────────────────────
      case "paragraph": {
        if (line.trim() === "") {
          // Blank line terminates the paragraph
          flushParagraph(state);
          state.mode = "normal";
        } else if (
          isCodeDelim(line) ||
          isLiteralDelim(line) ||
          isPassthroughDelim(line) ||
          isQuoteDelim(line) ||
          matchHeading(line) !== null ||
          isThematicBreak(line)
        ) {
          // Block-opener — flush current paragraph and re-dispatch
          flushParagraph(state);
          state.mode = "normal";
          dispatchNormal(line, state);
        } else {
          // Continue accumulating paragraph text
          state.currentLines.push(line);
        }
        break;
      }

      // ── code_block mode ────────────────────────────────────────────────
      case "code_block": {
        if (isCodeDelim(line)) {
          // Closing delimiter — emit the code block
          const value = state.currentLines.join("\n") + (state.currentLines.length > 0 ? "\n" : "");
          const node: CodeBlockNode = {
            type: "code_block",
            language: state.pendingLanguage,
            value,
          };
          state.blocks.push(node);
          state.currentLines = [];
          state.pendingLanguage = null;
          state.mode = "normal";
        } else {
          state.currentLines.push(line);
        }
        break;
      }

      // ── literal_block mode ─────────────────────────────────────────────
      case "literal_block": {
        if (isLiteralDelim(line)) {
          const value = state.currentLines.join("\n") + (state.currentLines.length > 0 ? "\n" : "");
          const node: CodeBlockNode = {
            type: "code_block",
            language: null,
            value,
          };
          state.blocks.push(node);
          state.currentLines = [];
          state.mode = "normal";
        } else {
          state.currentLines.push(line);
        }
        break;
      }

      // ── passthrough_block mode ─────────────────────────────────────────
      case "passthrough_block": {
        if (isPassthroughDelim(line)) {
          const content = state.currentLines.join("\n");
          const node: RawBlockNode = {
            type: "raw_block",
            format: "html",
            value: content,
          };
          state.blocks.push(node);
          state.currentLines = [];
          state.mode = "normal";
        } else {
          state.currentLines.push(line);
        }
        break;
      }

      // ── quote_block mode ───────────────────────────────────────────────
      case "quote_block": {
        if (isQuoteDelim(line)) {
          // Recursively parse the accumulated content as AsciiDoc
          const innerText = state.currentLines.join("\n");
          const innerDoc = parse(innerText);
          const node: BlockquoteNode = {
            type: "blockquote",
            children: innerDoc.children as BlockNode[],
          };
          state.blocks.push(node);
          state.currentLines = [];
          state.mode = "normal";
        } else {
          state.currentLines.push(line);
        }
        break;
      }

      // ── unordered_list mode ────────────────────────────────────────────
      case "unordered_list": {
        if (line.trim() === "") {
          // Blank line terminates the list
          flushList(state, false);
          state.mode = "normal";
        } else {
          const item = matchUnorderedItem(line);
          if (item !== null) {
            state.listItems.push(item);
          } else {
            // Non-list line — flush list and re-dispatch
            flushList(state, false);
            state.mode = "normal";
            dispatchNormal(line, state);
          }
        }
        break;
      }

      // ── ordered_list mode ──────────────────────────────────────────────
      case "ordered_list": {
        if (line.trim() === "") {
          flushList(state, true);
          state.mode = "normal";
        } else {
          const item = matchOrderedItem(line);
          if (item !== null) {
            state.listItems.push(item);
          } else {
            flushList(state, true);
            state.mode = "normal";
            dispatchNormal(line, state);
          }
        }
        break;
      }
    }
  }

  // ── End-of-file: flush any pending content ────────────────────────────────

  switch (state.mode) {
    case "paragraph":
      flushParagraph(state);
      break;
    case "unordered_list":
      flushList(state, false);
      break;
    case "ordered_list":
      flushList(state, true);
      break;
    case "code_block":
    case "literal_block": {
      // Unterminated delimited block — emit what we have
      const value = state.currentLines.join("\n") + (state.currentLines.length > 0 ? "\n" : "");
      const node: CodeBlockNode = {
        type: "code_block",
        language: state.mode === "code_block" ? state.pendingLanguage : null,
        value,
      };
      state.blocks.push(node);
      break;
    }
    case "passthrough_block": {
      const node: RawBlockNode = {
        type: "raw_block",
        format: "html",
        value: state.currentLines.join("\n"),
      };
      state.blocks.push(node);
      break;
    }
    case "quote_block": {
      const innerText = state.currentLines.join("\n");
      const innerDoc = parse(innerText);
      const node: BlockquoteNode = {
        type: "blockquote",
        children: innerDoc.children as BlockNode[],
      };
      state.blocks.push(node);
      break;
    }
    default:
      break;
  }

  return { type: "document", children: state.blocks };
}
