/**
 * Document AST — Format-Agnostic Intermediate Representation
 *
 * The Document AST is the LLVM IR of documents. It sits between front-end
 * parsers (Markdown, RST, HTML, DOCX) and back-end renderers (HTML, PDF,
 * plain text, LaTeX). Every front-end produces this IR; every back-end
 * consumes it. N front-ends × M back-ends requires only N + M implementations
 * instead of N × M.
 *
 *     Markdown ────────────────────────────────► HTML
 *     reStructuredText ────► Document AST ────► PDF
 *     HTML ────────────────────────────────────► Plain text
 *     DOCX ────────────────────────────────────► DOCX
 *
 * Spec: TE00 — Document AST
 *
 * Design principles:
 *   1. Semantic, not notational — nodes carry meaning, not syntax
 *   2. Resolved, not deferred   — all link references resolved before IR
 *   3. Format-agnostic          — RawBlockNode/RawInlineNode carry a `format` tag
 *   4. Immutable and typed      — all fields are `readonly`
 *   5. Minimal and stable       — only universal document concepts
 *
 * @module types
 */

// ─── Block Nodes ──────────────────────────────────────────────────────────────
//
// Block nodes form the structural skeleton of a document. They live at the
// top level of the document and can be nested (e.g. blockquotes, list items).

/**
 * The root of every document produced by a front-end parser.
 *
 * Every IR value is exactly one DocumentNode. An empty document has an empty
 * `children` array. DocumentNode is the only node type that cannot appear as
 * a child of another node.
 *
 * ```
 * DocumentNode
 *   ├── HeadingNode (level 1)
 *   ├── ParagraphNode
 *   └── ListNode (ordered, tight)
 *         ├── ListItemNode
 *         └── ListItemNode
 * ```
 */
export interface DocumentNode {
  readonly type: "document";
  readonly children: readonly BlockNode[];
}

/**
 * A section heading with a nesting depth. Semantically corresponds to
 * `<h1>`–`<h6>` in HTML, `=====` / `-----` underlines in RST,
 * `\section{}` / `\subsection{}` in LaTeX, and Heading 1–6 styles in DOCX.
 *
 * Levels beyond 6 (if a source format supports them) are clamped to 6.
 *
 * ```
 * HeadingNode { level: 2, children: [TextNode { value: "Hello" }] }
 * → <h2>Hello</h2>
 * ```
 */
export interface HeadingNode {
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: readonly InlineNode[];
}

/**
 * A block of prose. Contains one or more inline nodes — text, emphasis, links,
 * and soft breaks between the original source lines.
 *
 * Paragraphs are the most common block type. Any content that is not more
 * specifically typed (heading, list, code block, etc.) becomes a paragraph.
 *
 * ```
 * ParagraphNode {
 *   children: [
 *     TextNode { value: "Hello " },
 *     EmphasisNode { children: [TextNode { value: "world" }] },
 *   ]
 * }
 * → <p>Hello <em>world</em></p>
 * ```
 */
export interface ParagraphNode {
  readonly type: "paragraph";
  readonly children: readonly InlineNode[];
}

/**
 * A block of literal code or pre-formatted text.
 *
 * The `value` is raw — it is NOT decoded for HTML entities and NOT processed
 * for inline markup. Back-ends that render to a visual format should use a
 * monospace font. Syntax highlighting tools can use the `language` hint.
 *
 * The `value` field always ends with `\n`. Back-ends should not add extra
 * newlines when rendering.
 *
 * ```typescript
 * // Fenced code block:
 * // ```typescript
 * // const x = 1;
 * // ```
 * CodeBlockNode { language: "typescript", value: "const x = 1;\n" }
 * → <pre><code class="language-typescript">const x = 1;\n</code></pre>
 * ```
 */
export interface CodeBlockNode {
  readonly type: "code_block";
  /** Syntax language hint, e.g. `"typescript"`, `"python"`. `null` when unknown. */
  readonly language: string | null;
  /** Raw source code, including the trailing newline. Never HTML-encoded. */
  readonly value: string;
}

/**
 * A block of content set apart as a quotation or aside.
 *
 * Can contain any block nodes, including nested blockquotes. In HTML renders
 * as `<blockquote>…</blockquote>`.
 *
 * ```
 * BlockquoteNode {
 *   children: [ParagraphNode { children: [TextNode { value: "quote" }] }]
 * }
 * → <blockquote>\n<p>quote</p>\n</blockquote>
 * ```
 */
export interface BlockquoteNode {
  readonly type: "blockquote";
  readonly children: readonly BlockNode[];
}

/**
 * An ordered (numbered) or unordered (bulleted) list.
 *
 * A ListNode contains one or more ListItemNode children. Each list item
 * contains block-level content (paragraphs, nested lists, code blocks, etc.).
 *
 * **Tight vs loose.** The `tight` flag is a rendering hint from the source.
 * A tight list is written without blank lines between items; a loose list has
 * blank lines. In HTML, tight lists suppress `<p>` wrappers around paragraph
 * content. Other back-ends may use this flag differently or ignore it.
 *
 * **Ordered list start.** `start` records the opening item number. `1` is the
 * default; `42` means the list begins at forty-two. `null` for unordered.
 *
 * ```
 * ListNode { ordered: false, start: null, tight: true, children: [...] }
 * → <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>
 *
 * ListNode { ordered: true, start: 3, tight: false, children: [...] }
 * → <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>
 * ```
 */
export interface ListNode {
  readonly type: "list";
  readonly ordered: boolean;
  /** Opening number for ordered lists. `null` for unordered lists. */
  readonly start: number | null;
  /**
   * Tight = no blank lines between items and no blank line inside any item.
   * In HTML tight mode, paragraph content is rendered without `<p>` tags.
   */
  readonly tight: boolean;
  readonly children: readonly ListItemNode[];
}

/**
 * One item in a ListNode. Contains block-level content.
 *
 * For tight lists the children are typically ParagraphNodes whose content
 * is rendered without wrapping `<p>` tags (the `tight` flag on the parent
 * ListNode controls this).
 */
export interface ListItemNode {
  readonly type: "list_item";
  readonly children: readonly BlockNode[];
}

/**
 * A visual separator between sections. Leaf node — no children.
 *
 * In HTML renders as `<hr />`. In RST `----`. In plain text `---`.
 * In DOCX a horizontal rule paragraph style.
 */
export interface ThematicBreakNode {
  readonly type: "thematic_break";
}

/**
 * A block of raw content to be passed through verbatim to a specific back-end.
 *
 * The `format` field identifies the target renderer (e.g. `"html"`,
 * `"latex"`, `"rtf"`). Back-ends that do not recognise `format` **must**
 * skip this node silently — they should not corrupt output with content
 * intended for a different renderer.
 *
 * **Generalisation of `HtmlBlockNode`.** The CommonMark AST (TE01) has
 * `HtmlBlockNode { type: "html_block" }`. The Document AST replaces it with
 * `RawBlockNode { type: "raw_block"; format: "html" }`. The semantics are
 * identical for HTML output; the `format` tag extends the concept to any
 * target format.
 *
 * ```
 * Back-end contract:
 *   format matches output → emit value verbatim (no escaping)
 *   format does not match → skip silently
 *
 * format     HTML back-end    LaTeX back-end    plain-text
 * ─────────  ─────────────    ──────────────    ──────────
 * "html"     emit             skip              skip
 * "latex"    skip             emit              skip
 * "rtf"      skip             skip              skip
 * ```
 */
export interface RawBlockNode {
  readonly type: "raw_block";
  /** Target back-end format tag, e.g. `"html"`, `"latex"`, `"rtf"`. */
  readonly format: string;
  /** Raw content — never HTML-encoded or otherwise processed. */
  readonly value: string;
}

/**
 * Union of all block node types.
 *
 * Use in `switch (node.type)` with an exhaustive `default: assertNever(node)`
 * to get TypeScript's full discriminated-union type narrowing.
 *
 * Note: `DocumentNode` is in this union even though it can only appear as the
 * root, never as a child. This simplifies `switch` exhaustiveness checks and
 * recursive traversal code.
 */
export type BlockNode =
  | DocumentNode
  | HeadingNode
  | ParagraphNode
  | CodeBlockNode
  | BlockquoteNode
  | ListNode
  | ListItemNode
  | ThematicBreakNode
  | RawBlockNode;

// ─── Inline Nodes ─────────────────────────────────────────────────────────────
//
// Inline nodes live inside block nodes that contain prose content: headings,
// paragraphs, and list items. They represent formatted text spans, links,
// images, and structural characters within a paragraph.

/**
 * Plain text with no markup.
 *
 * All HTML character references (`&amp;`, `&#65;`, `&#x41;`) are decoded into
 * their Unicode equivalents before being stored. The `value` field contains
 * the final, display-ready Unicode string.
 *
 * Adjacent text nodes are automatically merged during inline parsing — a
 * well-formed IR never has two consecutive TextNode siblings.
 *
 * ```
 * "Hello &amp; world" → TextNode { value: "Hello & world" }
 * ```
 */
export interface TextNode {
  readonly type: "text";
  /** Decoded Unicode string, ready for display. Never contains raw HTML entities. */
  readonly value: string;
}

/**
 * Stressed emphasis. In HTML renders as `<em>`. In Markdown, `*text*` or
 * `_text_`. In RST, `:emphasis:`. In DOCX, italic text.
 *
 * Whether `*` or `_` opens/closes emphasis in Markdown depends on surrounding
 * characters (whitespace, punctuation, Unicode categories). The front-end
 * parser resolves this before producing the IR.
 *
 * ```
 * EmphasisNode { children: [TextNode { value: "hello" }] }
 * → <em>hello</em>
 * ```
 */
export interface EmphasisNode {
  readonly type: "emphasis";
  readonly children: readonly InlineNode[];
}

/**
 * Strong importance. In HTML renders as `<strong>`. In Markdown, `**text**`
 * or `__text__`. In RST, `**bold**`. In DOCX, bold text.
 *
 * Strong and emphasis can nest: `***text***` can produce either
 * `<em><strong>…</strong></em>` or `<strong><em>…</em></strong>`.
 *
 * ```
 * StrongNode { children: [TextNode { value: "bold" }] }
 * → <strong>bold</strong>
 * ```
 */
export interface StrongNode {
  readonly type: "strong";
  readonly children: readonly InlineNode[];
}

/**
 * Inline code. The value is raw — not decoded for HTML entities and not
 * processed for Markdown. Leading and trailing spaces are stripped when
 * the content is surrounded by spaces on both sides.
 *
 * ```
 * `` `const x = 1` `` → CodeSpanNode { value: "const x = 1" }
 * → <code>const x = 1</code>
 * ```
 */
export interface CodeSpanNode {
  readonly type: "code_span";
  /** Raw code content, not decoded. */
  readonly value: string;
}

/**
 * A hyperlink with resolved destination.
 *
 * The `destination` is always a fully resolved URL — all reference
 * indirections have been resolved by the front-end. The IR never contains
 * unresolved reference links.
 *
 * **Resolution contract.** Front-ends that handle reference-style links
 * (Markdown's `[text][label]`) must resolve them against link definition maps
 * before emitting a `LinkNode`. If a reference is unresolvable, the front-end
 * emits the source text as `TextNode` children (CommonMark spec behaviour).
 *
 * Links cannot be nested — a `LinkNode` cannot contain another `LinkNode`.
 *
 * ```
 * LinkNode {
 *   destination: "https://example.com",
 *   title: "Example",
 *   children: [TextNode { value: "click here" }]
 * }
 * → <a href="https://example.com" title="Example">click here</a>
 * ```
 */
export interface LinkNode {
  readonly type: "link";
  /** Fully resolved URL. Never a `[label]` reference — always an explicit destination. */
  readonly destination: string;
  /** Optional tooltip / hover text. `null` if absent. */
  readonly title: string | null;
  readonly children: readonly InlineNode[];
}

/**
 * An embedded image.
 *
 * Like `LinkNode`, `destination` is always the fully resolved URL. The `alt`
 * field is the plain-text fallback description (all inline markup stripped).
 *
 * **Alt text.** The `alt` field is a plain string (not inline nodes) because
 * alt text is by definition a plain-text description for screen readers and
 * fallback contexts. For example, `![**hello**](img.png)` produces
 * `ImageNode { alt: "hello", … }` — markup is stripped before storing.
 *
 * **Back-end contract.** Back-ends that cannot embed images (plain text,
 * plain-text email) should render the `alt` text instead.
 *
 * ```
 * ImageNode { destination: "cat.png", alt: "a cat", title: null }
 * → <img src="cat.png" alt="a cat" />
 * ```
 */
export interface ImageNode {
  readonly type: "image";
  /** Fully resolved image URL. */
  readonly destination: string;
  /** Optional tooltip / hover text. `null` if absent. */
  readonly title: string | null;
  /** Plain-text alt description, markup stripped. */
  readonly alt: string;
}

/**
 * A URL or email address presented as a direct link, without custom link text.
 * The link text in all back-ends is the raw address itself.
 *
 * **Why preserve `isEmail`?** Two reasons:
 *
 *   1. HTML back-ends need to prepend `mailto:` for email autolinks:
 *      `<https://example.com>` → `<a href="https://example.com">…</a>` but
 *      `<user@example.com>` → `<a href="mailto:user@example.com">…</a>`.
 *
 *   2. Other back-ends (PDF, DOCX) may format email addresses differently from
 *      URLs — e.g. not underlining email addresses in print output.
 *
 * This distinction is semantically meaningful downstream and would be
 * unrecoverable if collapsed to a plain `LinkNode`.
 *
 * ```
 * AutolinkNode { destination: "user@example.com", isEmail: true }
 * → <a href="mailto:user@example.com">user@example.com</a>
 * ```
 */
export interface AutolinkNode {
  readonly type: "autolink";
  /** The URL or email address, without the surrounding `< >`. */
  readonly destination: string;
  /** `true` for email autolinks; `false` for URL autolinks. */
  readonly isEmail: boolean;
}

/**
 * An inline span of raw content to be passed through verbatim to a specific
 * back-end. The `format` field names the target renderer.
 *
 * **Generalisation of `HtmlInlineNode`.** The CommonMark AST (TE01) has
 * `HtmlInlineNode { type: "html_inline" }`. The Document AST replaces it with
 * `RawInlineNode { type: "raw_inline"; format: "html" }`. The semantics are
 * identical for HTML output; the `format` tag extends the concept to any target.
 *
 * The same back-end contract applies as for `RawBlockNode`: emit verbatim if
 * `format` matches, skip silently if it does not.
 *
 * ```
 * RawInlineNode { format: "html", value: "<em>raw</em>" }
 * → (HTML back-end) <em>raw</em>
 * → (LaTeX back-end) (nothing)
 * ```
 */
export interface RawInlineNode {
  readonly type: "raw_inline";
  /** Target back-end format tag, e.g. `"html"`, `"latex"`. */
  readonly format: string;
  /** Raw content — never escaped or processed. */
  readonly value: string;
}

/**
 * A forced line break within a paragraph.
 *
 * Forces `<br />` in HTML, `\newline` in LaTeX, a literal `\n` in plain-text
 * renderers. In Markdown, produced by two or more trailing spaces before a
 * newline, or a backslash `\` immediately before a newline.
 */
export interface HardBreakNode {
  readonly type: "hard_break";
}

/**
 * A soft line break — a newline within a paragraph that is not a hard break.
 *
 * In HTML, soft breaks render as `\n` (browsers collapse to a single space).
 * In plain text, they render as a literal newline. The back-end controls
 * the exact rendering.
 *
 * The IR preserves soft breaks so that back-ends controlling line-wrapping
 * behaviour can make the right choice. A back-end may also discard soft breaks
 * and re-wrap paragraphs independently.
 */
export interface SoftBreakNode {
  readonly type: "soft_break";
}

/**
 * Union of all inline node types.
 *
 * Use in `switch (node.type)` to get full discriminated-union narrowing.
 */
export type InlineNode =
  | TextNode
  | EmphasisNode
  | StrongNode
  | CodeSpanNode
  | LinkNode
  | ImageNode
  | AutolinkNode
  | RawInlineNode
  | HardBreakNode
  | SoftBreakNode;

/** Union of all node types (block + inline). */
export type Node = BlockNode | InlineNode;
