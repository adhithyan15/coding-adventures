/**
 * CommonMark AST Node Types
 *
 * These are the 20 canonical node types defined in spec TE01. Every
 * document parsed by this library is represented as a tree of these nodes.
 *
 * The tree has two layers:
 *
 *   Block nodes  — structural containers (document, heading, list, ...)
 *   Inline nodes — content within blocks (text, emphasis, link, ...)
 *
 * All nodes are immutable (readonly fields). The parser builds them
 * bottom-up and never mutates them after construction.
 *
 * @module types
 */

// ─── Block Nodes ──────────────────────────────────────────────────────────────

/**
 * The root of every parsed document. Every parse call returns exactly
 * one DocumentNode as the top-level container.
 */
export interface DocumentNode {
  readonly type: "document";
  readonly children: readonly BlockNode[];
}

/**
 * ATX heading (`# Heading`) or setext heading (underlined with `=`/`-`).
 * Level 1 = `<h1>`, level 6 = `<h6>`.
 */
export interface HeadingNode {
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: readonly InlineNode[];
}

/**
 * A sequence of non-blank lines that does not begin a more specific
 * block. The most common block type — everything that isn't a heading,
 * list, code block, etc. becomes a paragraph.
 */
export interface ParagraphNode {
  readonly type: "paragraph";
  readonly children: readonly InlineNode[];
}

/**
 * A fenced code block (``` or ~~~) or an indented code block (4+ spaces).
 * The `language` field is extracted from the fenced block's info string
 * (the first word after the opening fence). It is null for indented
 * code blocks and fenced blocks with no info string.
 *
 * The `value` field contains the raw, unprocessed code text. No HTML
 * entity encoding, no Markdown processing — just the characters verbatim.
 */
export interface CodeBlockNode {
  readonly type: "code_block";
  readonly language: string | null;
  readonly value: string;
}

/**
 * A blockquote, introduced by `>` at the start of each line.
 * Can contain any block nodes, including nested blockquotes.
 */
export interface BlockquoteNode {
  readonly type: "blockquote";
  readonly children: readonly BlockNode[];
}

/**
 * An ordered or unordered list. Contains one or more ListItemNodes.
 *
 * `ordered`: true for `1. item`, false for `- item`
 * `start`: the start number for ordered lists (e.g., `42.` → start=42)
 * `tight`: a list is tight if no list items are separated by blank lines
 *   and no item contains a blank line internally. Tight lists render
 *   their paragraphs without `<p>` tags in HTML.
 */
export interface ListNode {
  readonly type: "list";
  readonly ordered: boolean;
  readonly start: number | null;
  readonly tight: boolean;
  readonly children: readonly ListItemNode[];
}

/**
 * One item in a ListNode. Contains block-level content.
 * For tight lists, the children are typically ParagraphNodes whose
 * content is rendered without wrapping `<p>` tags.
 */
export interface ListItemNode {
  readonly type: "list_item";
  readonly children: readonly BlockNode[];
}

/**
 * A thematic break (`---`, `***`, or `___`). Renders as `<hr>`.
 * No children — a leaf node.
 */
export interface ThematicBreakNode {
  readonly type: "thematic_break";
}

/**
 * A raw HTML block that passes through verbatim. CommonMark defines
 * 7 types of HTML blocks with different opening/closing conditions.
 * Their content is not processed for Markdown or HTML entities.
 */
export interface HtmlBlockNode {
  readonly type: "html_block";
  readonly value: string;
}

/**
 * A link reference definition: `[label]: destination "title"`.
 * These are not rendered directly — they define labels that inline
 * links can reference with `[text][label]`.
 *
 * The `label` is normalized: lowercased and internal whitespace
 * collapsed to a single space. So `[Example]`, `[EXAMPLE]`, and
 * `[  example  ]` all resolve to the label `"example"`.
 */
export interface LinkDefinitionNode {
  readonly type: "link_definition";
  readonly label: string;
  readonly destination: string;
  readonly title: string | null;
}

export type BlockNode =
  | DocumentNode
  | HeadingNode
  | ParagraphNode
  | CodeBlockNode
  | BlockquoteNode
  | ListNode
  | ListItemNode
  | ThematicBreakNode
  | HtmlBlockNode
  | LinkDefinitionNode;

// ─── Inline Nodes ─────────────────────────────────────────────────────────────

/**
 * Plain text. All characters that do not trigger other inline constructs
 * accumulate here. HTML character references (`&amp;`, `&#65;`, `&#x41;`)
 * are decoded into their Unicode equivalents. The `value` field holds the
 * final, display-ready string.
 */
export interface TextNode {
  readonly type: "text";
  readonly value: string;
}

/**
 * Single `*text*` or `_text_`. Renders as `<em>`.
 *
 * Whether `*` or `_` opens/closes emphasis depends on the surrounding
 * characters (whitespace, punctuation, Unicode categories). CommonMark
 * Appendix A defines the full 17-rule algorithm.
 */
export interface EmphasisNode {
  readonly type: "emphasis";
  readonly children: readonly InlineNode[];
}

/**
 * Double `**text**` or `__text__`. Renders as `<strong>`.
 *
 * Strong and emphasis can nest: `***text***` can produce either
 * `<em><strong>…</strong></em>` or `<strong><em>…</em></strong>`
 * depending on the surrounding delimiter context.
 */
export interface StrongNode {
  readonly type: "strong";
  readonly children: readonly InlineNode[];
}

/**
 * Inline code delimited by backtick strings: `` `code` ``.
 * The opening and closing backtick strings must have equal length.
 * Content is not decoded for HTML entities or processed for Markdown.
 * Leading and trailing spaces are stripped if both are present.
 */
export interface CodeSpanNode {
  readonly type: "code_span";
  readonly value: string;
}

/**
 * An inline link `[text](url "title")` or a reference link `[text][label]`.
 * Reference links are resolved against LinkDefinitionNodes in the document.
 * After resolution, all links have an explicit `destination`.
 *
 * Links cannot contain other links (no nesting).
 */
export interface LinkNode {
  readonly type: "link";
  readonly destination: string;
  readonly title: string | null;
  readonly children: readonly InlineNode[];
}

/**
 * An inline image `![alt](url "title")` or `![alt][label]`.
 * The `alt` field is the plain-text rendering of the alt content
 * (all markup stripped — just the text values concatenated).
 */
export interface ImageNode {
  readonly type: "image";
  readonly destination: string;
  readonly title: string | null;
  readonly alt: string;
}

/**
 * URL or email enclosed in angle brackets: `<https://example.com>`.
 * The `isEmail` field distinguishes email autolinks from URL autolinks.
 */
export interface AutolinkNode {
  readonly type: "autolink";
  readonly destination: string;
  readonly isEmail: boolean;
}

/**
 * Raw HTML tags and entities inline within paragraph text.
 * Passed through verbatim, not processed further.
 */
export interface HtmlInlineNode {
  readonly type: "html_inline";
  readonly value: string;
}

/**
 * A hard line break forces `<br>` in HTML output.
 * Produced by two or more trailing spaces before a newline,
 * or a backslash `\` immediately before a newline.
 */
export interface HardBreakNode {
  readonly type: "hard_break";
}

/**
 * A soft line break: a single newline within a paragraph.
 * Renderers typically emit a space or newline in HTML output.
 * Preserved in the AST so renderers can control the behaviour.
 */
export interface SoftBreakNode {
  readonly type: "soft_break";
}

export type InlineNode =
  | TextNode
  | EmphasisNode
  | StrongNode
  | CodeSpanNode
  | LinkNode
  | ImageNode
  | AutolinkNode
  | HtmlInlineNode
  | HardBreakNode
  | SoftBreakNode;

/** Union of all node types. */
export type Node = BlockNode | InlineNode;

// ─── Parse Options ─────────────────────────────────────────────────────────────

/**
 * Options passed to the `parse()` function.
 *
 * In V1 the only preset is `"commonmark"`. Future specs (TE04, TE05) will
 * add `"gfm"` and `"thunderegg"` presets with additional node types.
 */
export interface ParseOptions {
  /** Which feature preset to use. Defaults to "commonmark". */
  readonly preset?: "commonmark";
}

// ─── Link Reference Map ────────────────────────────────────────────────────────

/**
 * A resolved link reference definition, keyed by normalized label.
 * Populated during Phase 1 (block parsing) and consumed during Phase 2
 * (inline parsing) to resolve `[text][label]` and `[text][]` links.
 */
export interface LinkReference {
  readonly destination: string;
  readonly title: string | null;
}

export type LinkRefMap = Map<string, LinkReference>;
