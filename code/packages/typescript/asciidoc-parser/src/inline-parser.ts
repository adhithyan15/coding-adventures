/**
 * AsciiDoc Inline Parser
 *
 * Converts a plain text string (the content of a paragraph, heading, list
 * item, etc.) into an array of InlineNode values using the Document AST types.
 *
 * The scanner is a left-to-right character-by-character loop. At each position
 * we check for special sequences in priority order — the first match wins and
 * advances the position past the matched span. Any character that does not
 * match a special form is accumulated into a text buffer that is flushed as a
 * TextNode whenever a special form is found.
 *
 * === AsciiDoc vs CommonMark differences ===
 *
 *   CommonMark:  `*text*`  → EmphasisNode    `**text**` → StrongNode
 *   AsciiDoc:    `*text*`  → StrongNode       `_text_`   → EmphasisNode
 *   AsciiDoc:    `**text**`→ StrongNode (unconstrained — works mid-word)
 *   AsciiDoc:    `__text__`→ EmphasisNode (unconstrained)
 *
 * This distinction is critical. In AsciiDoc, the asterisk signals *strong*
 * (bold) text, not emphasis (italic). Underscores signal emphasis (italic).
 *
 * === Priority order ===
 *
 *   1.  `  \n`  — HardBreakNode  (two trailing spaces before newline)
 *   2.  `\\\n`  — HardBreakNode  (backslash before newline)
 *   3.  `\n`    — SoftBreakNode
 *   4.  `` ` `` — CodeSpanNode   (find closing backtick)
 *   5.  `**`    — StrongNode     (unconstrained — find closing `**`)
 *   6.  `__`    — EmphasisNode   (unconstrained — find closing `__`)
 *   7.  `*`     — StrongNode     (constrained — word-boundary)
 *   8.  `_`     — EmphasisNode   (constrained)
 *   9.  `link:` — LinkNode
 *  10.  `image:`— ImageNode
 *  11.  `<<`    — CrossRefNode
 *  12.  `https://` or `http://` — LinkNode (with [text]) or AutolinkNode
 *  13.  other   — append to text buffer
 *
 * @module inline-parser
 */

import type {
  InlineNode,
  TextNode,
  EmphasisNode,
  StrongNode,
  CodeSpanNode,
  LinkNode,
  ImageNode,
  AutolinkNode,
  HardBreakNode,
  SoftBreakNode,
} from "@coding-adventures/document-ast";

// ─── Helper: flush text buffer ────────────────────────────────────────────────

/**
 * Flush the accumulated text buffer as a TextNode.
 *
 * We call this every time we encounter a special inline form so that the plain
 * text before it is emitted as its own TextNode. After flushing, the buffer is
 * reset to the empty string.
 */
function flushText(buf: string, nodes: InlineNode[]): void {
  if (buf.length > 0) {
    const node: TextNode = { type: "text", value: buf };
    nodes.push(node);
  }
}

// ─── Helper: find closing delimiter ──────────────────────────────────────────

/**
 * Locate the first occurrence of `delim` in `text` starting at `startPos`,
 * returning the content before it and the position after it.
 *
 * Returns null if the delimiter is not found, meaning the opening delimiter
 * was not a real marker and should be treated as literal text.
 *
 * Example:
 *   findClosing("hello *world*!", "*", 0) → { content: "hello ", rest: "world*!", idx: 6 }
 *   Wait — we scan from startPos so the caller already consumed the opening.
 *   findClosing("world*!", "*", 0) → { content: "world", after: 6 }
 */
function findClosing(
  text: string,
  delim: string,
  startPos: number
): { content: string; after: number } | null {
  const idx = text.indexOf(delim, startPos);
  if (idx === -1) return null;
  return {
    content: text.slice(startPos, idx),
    after: idx + delim.length,
  };
}

// ─── Helper: parse inline macro argument `url[text]` ──────────────────────────

/**
 * After consuming the macro prefix (e.g. `link:` or `image:`), parse the
 * remaining `url[text]` or `url[alt]` form.
 *
 * Returns the url, the display text/alt, and the position after the closing `]`.
 * Returns null if the form is not complete (no `[` or `]`).
 */
function parseMacroArgs(
  text: string,
  pos: number
): { url: string; label: string; after: number } | null {
  // Find the opening `[`
  const bracketOpen = text.indexOf("[", pos);
  if (bracketOpen === -1) return null;
  // Find the closing `]`
  const bracketClose = text.indexOf("]", bracketOpen + 1);
  if (bracketClose === -1) return null;

  const url = text.slice(pos, bracketOpen);
  const label = text.slice(bracketOpen + 1, bracketClose);
  return { url, label, after: bracketClose + 1 };
}

// ─── Main export: parseInline ─────────────────────────────────────────────────

/**
 * Parse an AsciiDoc inline string into an array of InlineNode values.
 *
 * This is used by the block parser to fill the `children` arrays of
 * ParagraphNode, HeadingNode, and ListItemNode. The algorithm is a simple
 * single-pass scanner with a text accumulator.
 *
 * @param text   Raw AsciiDoc inline content (may contain `\n` for line breaks).
 * @returns      Array of InlineNode values representing the parsed content.
 *
 * @example
 * ```typescript
 * parseInline("Hello *world*")
 * // → [TextNode("Hello "), StrongNode([TextNode("world")])]
 *
 * parseInline("_italic_ and `code`")
 * // → [EmphasisNode([TextNode("italic")]), TextNode(" and "), CodeSpanNode("code")]
 * ```
 */
export function parseInline(text: string): InlineNode[] {
  const nodes: InlineNode[] = [];
  let buf = ""; // accumulates plain text between special forms
  let i = 0;

  while (i < text.length) {
    const ch = text[i];

    // ── Priority 1 & 2: Hard break ─────────────────────────────────────────
    // Two trailing spaces before \n  → HardBreakNode
    if (ch === " " && text[i + 1] === " " && text[i + 2] === "\n") {
      flushText(buf, nodes);
      buf = "";
      const hb: HardBreakNode = { type: "hard_break" };
      nodes.push(hb);
      i += 3;
      continue;
    }
    // Backslash before \n → HardBreakNode
    if (ch === "\\" && text[i + 1] === "\n") {
      flushText(buf, nodes);
      buf = "";
      const hb: HardBreakNode = { type: "hard_break" };
      nodes.push(hb);
      i += 2;
      continue;
    }

    // ── Priority 3: Soft break ─────────────────────────────────────────────
    if (ch === "\n") {
      flushText(buf, nodes);
      buf = "";
      const sb: SoftBreakNode = { type: "soft_break" };
      nodes.push(sb);
      i += 1;
      continue;
    }

    // ── Priority 4: Code span (backtick) ──────────────────────────────────
    if (ch === "`") {
      const result = findClosing(text, "`", i + 1);
      if (result !== null) {
        flushText(buf, nodes);
        buf = "";
        const code: CodeSpanNode = { type: "code_span", value: result.content };
        nodes.push(code);
        i = result.after;
        continue;
      }
    }

    // ── Priority 5: Strong unconstrained `**...**` ────────────────────────
    if (ch === "*" && text[i + 1] === "*") {
      const result = findClosing(text, "**", i + 2);
      if (result !== null) {
        flushText(buf, nodes);
        buf = "";
        const strong: StrongNode = {
          type: "strong",
          children: parseInline(result.content),
        };
        nodes.push(strong);
        i = result.after;
        continue;
      }
    }

    // ── Priority 6: Emphasis unconstrained `__...__` ──────────────────────
    if (ch === "_" && text[i + 1] === "_") {
      const result = findClosing(text, "__", i + 2);
      if (result !== null) {
        flushText(buf, nodes);
        buf = "";
        const em: EmphasisNode = {
          type: "emphasis",
          children: parseInline(result.content),
        };
        nodes.push(em);
        i = result.after;
        continue;
      }
    }

    // ── Priority 7: Strong constrained `*...*` ────────────────────────────
    // In AsciiDoc, a single `*` signals STRONG (bold), unlike CommonMark.
    // Constrained means it must be at a word boundary (preceded by non-word
    // or start of string) and the closing `*` must be followed by non-word
    // or end of string.
    if (ch === "*") {
      const prevChar = i > 0 ? text[i - 1] : " ";
      const isWordBoundaryOpen = /\W/.test(prevChar);
      if (isWordBoundaryOpen) {
        const result = findClosing(text, "*", i + 1);
        if (result !== null) {
          // Verify the character after closing `*` is also a boundary
          const nextChar = text[result.after] ?? " ";
          const isWordBoundaryClose = /\W/.test(nextChar);
          if (isWordBoundaryClose) {
            flushText(buf, nodes);
            buf = "";
            const strong: StrongNode = {
              type: "strong",
              children: parseInline(result.content),
            };
            nodes.push(strong);
            i = result.after;
            continue;
          }
        }
      }
    }

    // ── Priority 8: Emphasis constrained `_..._` ──────────────────────────
    if (ch === "_") {
      const prevChar = i > 0 ? text[i - 1] : " ";
      const isWordBoundaryOpen = /\W/.test(prevChar);
      if (isWordBoundaryOpen) {
        const result = findClosing(text, "_", i + 1);
        if (result !== null) {
          const nextChar = text[result.after] ?? " ";
          const isWordBoundaryClose = /\W/.test(nextChar);
          if (isWordBoundaryClose) {
            flushText(buf, nodes);
            buf = "";
            const em: EmphasisNode = {
              type: "emphasis",
              children: parseInline(result.content),
            };
            nodes.push(em);
            i = result.after;
            continue;
          }
        }
      }
    }

    // ── Priority 9: Inline macro `link:url[text]` ─────────────────────────
    if (text.startsWith("link:", i)) {
      const args = parseMacroArgs(text, i + 5);
      if (args !== null) {
        flushText(buf, nodes);
        buf = "";
        const link: LinkNode = {
          type: "link",
          url: args.url,
          title: null,
          children: args.label.length > 0 ? parseInline(args.label) : [{ type: "text", value: args.url } as TextNode],
        };
        nodes.push(link);
        i = args.after;
        continue;
      }
    }

    // ── Priority 10: Inline macro `image:url[alt]` ────────────────────────
    if (text.startsWith("image:", i)) {
      const args = parseMacroArgs(text, i + 6);
      if (args !== null) {
        flushText(buf, nodes);
        buf = "";
        const img: ImageNode = {
          type: "image",
          url: args.url,
          title: null,
          alt: args.label,
        };
        nodes.push(img);
        i = args.after;
        continue;
      }
    }

    // ── Priority 11: Cross-reference `<<anchor,text>>` ───────────────────
    if (ch === "<" && text[i + 1] === "<") {
      const close = text.indexOf(">>", i + 2);
      if (close !== -1) {
        const inner = text.slice(i + 2, close);
        const commaIdx = inner.indexOf(",");
        const anchor = commaIdx === -1 ? inner : inner.slice(0, commaIdx);
        const label = commaIdx === -1 ? anchor : inner.slice(commaIdx + 1).trim();
        flushText(buf, nodes);
        buf = "";
        const link: LinkNode = {
          type: "link",
          url: `#${anchor}`,
          title: null,
          children: [{ type: "text", value: label } as TextNode],
        };
        nodes.push(link);
        i = close + 2;
        continue;
      }
    }

    // ── Priority 12: URL autolinks `https://...` or `http://...` ─────────
    if (text.startsWith("https://", i) || text.startsWith("http://", i)) {
      // Look ahead for `[text]` suffix → full link macro
      const protocol = text.startsWith("https://", i) ? "https://" : "http://";
      const urlStart = i;
      // Find where the URL ends: space, newline, or end of string
      // But if followed immediately by `[`, treat as a link macro
      let urlEnd = i + protocol.length;
      while (urlEnd < text.length && !/[\s[\]]/.test(text[urlEnd])) {
        urlEnd++;
      }
      const url = text.slice(urlStart, urlEnd);

      if (text[urlEnd] === "[") {
        // Link with explicit text: `https://example.com[click here]`
        const bracketClose = text.indexOf("]", urlEnd + 1);
        if (bracketClose !== -1) {
          const label = text.slice(urlEnd + 1, bracketClose);
          flushText(buf, nodes);
          buf = "";
          const link: LinkNode = {
            type: "link",
            url,
            title: null,
            children: label.length > 0
              ? parseInline(label)
              : [{ type: "text", value: url } as TextNode],
          };
          nodes.push(link);
          i = bracketClose + 1;
          continue;
        }
      }

      // Bare URL → AutolinkNode
      flushText(buf, nodes);
      buf = "";
      const autolink: AutolinkNode = { type: "autolink", url, isEmail: false };
      nodes.push(autolink);
      i = urlEnd;
      continue;
    }

    // ── Default: accumulate into text buffer ──────────────────────────────
    buf += ch;
    i += 1;
  }

  // Flush any remaining plain text
  flushText(buf, nodes);
  return nodes;
}
