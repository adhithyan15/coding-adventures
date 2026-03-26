/**
 * Inline Parser
 *
 * Phase 2 of CommonMark parsing: scan raw inline content strings (produced
 * by the block parser) and emit inline AST nodes — emphasis, links, code
 * spans, etc.
 *
 * === Overview of Inline Constructs ===
 *
 * CommonMark recognises ten inline constructs, processed left-to-right:
 *
 *   1. Backslash escapes       `\*`    → literal `*`
 *   2. HTML character refs     `&amp;` → `&`
 *   3. Code spans              `` `code` ``
 *   4. HTML inline             `<em>`, `<!-- -->`, `<?...?>`
 *   5. Autolinks               `<https://example.com>`, `<me@example.com>`
 *   6. Hard line breaks        two trailing spaces + newline, or `\` + newline
 *   7. Soft line breaks        single newline within a paragraph
 *   8. Emphasis / strong       `*em*`, `**strong**`, `_em_`, `__strong__`
 *   9. Links                   `[text](url)`, `[text][label]`, `[text][]`
 *  10. Images                  `![alt](url)`, `![alt][label]`
 *
 * === The Delimiter Stack Algorithm ===
 *
 * Emphasis is the hardest part of CommonMark inline parsing. The rules are
 * context-sensitive: whether `*` or `_` can open or close emphasis depends
 * on what precedes and follows the run. CommonMark Appendix A defines the
 * canonical "delimiter stack" algorithm.
 *
 * The algorithm has two phases:
 *
 *   A. SCAN — read the input left-to-right, building a flat list of "tokens":
 *      ordinary text, delimiter runs (* ** _ __), code spans, links, etc.
 *      Each delimiter run is tagged as "can_open", "can_close", or both.
 *
 *   B. RESOLVE — walk the token list, matching openers with the nearest
 *      valid closers. For each matched pair, wrap the intervening tokens
 *      in an emphasis or strong node.
 *
 * We use the PushdownAutomaton from @coding-adventures/state-machine
 * conceptually to illustrate that bracket matching (for link/image `[...]`)
 * is a context-free problem requiring pushdown memory. In practice, the
 * inline parser manages its own bracket stack as a plain array, because the
 * CommonMark algorithm requires popping from arbitrary positions (not just
 * the top) when openers are deactivated.
 *
 * === Flanking Rules (CommonMark spec §6.2) ===
 *
 * A delimiter run of `*` is LEFT-FLANKING (can open) if:
 *   (a) not followed by Unicode whitespace, AND
 *   (b) either not followed by Unicode punctuation,
 *       OR preceded by Unicode whitespace or Unicode punctuation.
 *
 * A delimiter run of `*` is RIGHT-FLANKING (can close) if:
 *   (a) not preceded by Unicode whitespace, AND
 *   (b) either not preceded by Unicode punctuation,
 *       OR followed by Unicode whitespace or Unicode punctuation.
 *
 * For `_`, the open/close rules add extra conditions to avoid
 * intra-word emphasis:
 *   - `_` can open only if left-flanking AND
 *     (preceded by whitespace/punctuation OR not right-flanking).
 *   - `_` can close only if right-flanking AND
 *     (followed by whitespace/punctuation OR not left-flanking).
 *
 * @module inline-parser
 */

// Note: PushdownAutomaton is imported for educational documentation purposes.
// It illustrates that link bracket matching is context-free. See the
// BRACKET_PDA constant below.
import { PushdownAutomaton } from "@coding-adventures/state-machine";
import type {
  InlineNode, TextNode, EmphasisNode, StrongNode, StrikethroughNode, CodeSpanNode,
  LinkNode, ImageNode, AutolinkNode, RawInlineNode, HardBreakNode,
  SoftBreakNode, BlockNode, DocumentNode,
} from "@coding-adventures/document-ast";
import type { LinkRefMap } from "./types.js";
import {
  Scanner,
  isAsciiPunctuation,
  isUnicodePunctuation,
  isAsciiWhitespace,
  isUnicodeWhitespace,
  normalizeLinkLabel,
  normalizeUrl,
} from "./scanner.js";
import { decodeEntity, decodeEntities } from "./entities.js";

// ─── PDA Illustration: Bracket Matching ───────────────────────────────────────
//
// Link brackets `[...]` require matching the correct opener for each `]`.
// This is a classic context-free problem — we need a stack to count nesting.
//
// The PDA below illustrates the formal structure. States:
//   "scan"   — no open brackets at this level
//   "nested" — one or more open brackets on the stack
//   "done"   — epsilon-accepting state (for the formal definition)
//
// We do not call pda.process() in the actual parser because CommonMark's
// bracket deactivation rules require non-standard stack operations.
// The PDA is here as a learning artifact.

const _BRACKET_PDA_ILLUSTRATION = new PushdownAutomaton(
  new Set(["scan", "nested", "done"]),
  new Set(["[", "]"]),
  new Set(["$", "["]),
  [
    { source: "scan",   event: "[",  stackRead: "$", target: "nested", stackPush: ["$", "["] },
    { source: "nested", event: "[",  stackRead: "[", target: "nested", stackPush: ["[", "["] },
    { source: "nested", event: "]",  stackRead: "[", target: "scan",   stackPush: ["$"]       },
    { source: "scan",   event: null, stackRead: "$", target: "done",   stackPush: []           },
  ],
  "scan",
  "$",
  new Set(["done"]),
);

// ─── Delimiter Token Types ─────────────────────────────────────────────────────
//
// During the scan phase, the input is broken into a flat list of tokens.
// Delimiter runs (*/**/_/__) become DelimiterToken; everything else becomes
// a NodeToken wrapping a fully-resolved InlineNode. BracketToken marks
// open `[` or `![` that may become link/image openers.

/** A delimiter run: maximal run of `*`, `_`, or `~~`. */
interface DelimiterToken {
  kind: "delimiter";
  char: "*" | "_" | "~";
  count: number;      // length of the run
  canOpen: boolean;   // left-flanking (may open emphasis)
  canClose: boolean;  // right-flanking (may close emphasis)
  active: boolean;    // false once consumed by the resolution pass
}

/** A fully-resolved inline node (produced during scanning). */
interface NodeToken {
  kind: "node";
  node: InlineNode;
}

/** A bracket opener `[` or `![` — may become a link or image. */
interface BracketToken {
  kind: "bracket";
  isImage: boolean;   // true if preceded by `!`
  active: boolean;    // false if deactivated (used or no valid link follows)
  sourcePos: number;  // scanner position immediately after the `[`
}

type InlineToken = DelimiterToken | NodeToken | BracketToken;

// ─── Main Inline Parser ────────────────────────────────────────────────────────

/**
 * Parse a raw inline content string into a list of InlineNode trees.
 *
 * This is the core Phase 2 function. It is called by `resolveInlineContent`
 * for each paragraph and heading that contains inline markup.
 *
 * @param raw       The raw inline string from the block parser.
 * @param linkRefs  Link reference definitions collected in Phase 1.
 */
export function parseInline(raw: string, linkRefs: LinkRefMap): InlineNode[] {
  const scanner = new Scanner(raw);
  const tokens: InlineToken[] = [];

  // bracketStack holds the index into `tokens` of each open bracket.
  // We use a stack because brackets can nest: [[text](url)](url2)
  const bracketStack: number[] = [];

  // Text accumulation buffer — we flush it into a NodeToken whenever
  // a non-text construct is encountered.
  let textBuf = "";

  function flushText(): void {
    if (textBuf.length > 0) {
      tokens.push({ kind: "node", node: { type: "text", value: textBuf } });
      textBuf = "";
    }
  }

  // ─── Scan Phase ─────────────────────────────────────────────────────────────

  while (!scanner.done) {
    const ch = scanner.peek();

    // ── 1. Backslash escape ────────────────────────────────────────────────
    //
    // `\` followed by an ASCII punctuation character → the punctuation
    // is treated as a literal character (not a Markdown special).
    // `\` followed by a newline → hard line break.
    // `\` followed by anything else → literal backslash.
    if (ch === "\\") {
      const next = scanner.peek(1);
      if (next !== "" && isAsciiPunctuation(next)) {
        scanner.skip(2);
        textBuf += next;
        continue;
      }
      if (next === "\n") {
        scanner.skip(2);
        flushText();
        tokens.push({ kind: "node", node: { type: "hard_break" } as HardBreakNode });
        continue;
      }
      scanner.skip(1);
      textBuf += "\\";
      continue;
    }

    // ── 2. HTML character reference ────────────────────────────────────────
    //
    // `&name;`, `&#NNN;`, `&#xHHH;` → decoded Unicode character.
    // Unrecognised references are left as-is.
    if (ch === "&") {
      const m = scanner.matchRegex(/&(?:#[xX][0-9a-fA-F]{1,6}|#[0-9]{1,7}|[a-zA-Z][a-zA-Z0-9]{0,31});/);
      if (m !== null) {
        textBuf += decodeEntity(m);
        continue;
      }
      scanner.skip(1);
      textBuf += "&";
      continue;
    }

    // ── 3. Code span ───────────────────────────────────────────────────────
    //
    // One or more backticks open a code span; the same number closes it.
    // The content is stripped of one leading/trailing space (if both present)
    // and newlines are normalised to spaces. Not processed for Markdown.
    if (ch === "`") {
      const span = tryCodeSpan(scanner);
      if (span !== null) {
        flushText();
        tokens.push({ kind: "node", node: span });
        continue;
      }
      // Not a valid code span — literal backtick run
      const ticks = scanner.consumeWhile(c => c === "`");
      textBuf += ticks;
      continue;
    }

    // ── 4 & 5. HTML inline and autolinks (both start with `<`) ────────────
    if (ch === "<") {
      const autolink = tryAutolink(scanner);
      if (autolink !== null) {
        flushText();
        tokens.push({ kind: "node", node: autolink });
        continue;
      }
      const html = tryHtmlInline(scanner);
      if (html !== null) {
        flushText();
        tokens.push({ kind: "node", node: html });
        continue;
      }
      scanner.skip(1);
      textBuf += "<";
      continue;
    }

    // ── Image opener `![` ──────────────────────────────────────────────────
    if (ch === "!" && scanner.peek(1) === "[") {
      flushText();
      bracketStack.push(tokens.length);
      scanner.skip(2);
      tokens.push({ kind: "bracket", isImage: true, active: true, sourcePos: scanner.pos });
      continue;
    }

    // ── Link opener `[` ────────────────────────────────────────────────────
    if (ch === "[") {
      flushText();
      bracketStack.push(tokens.length);
      scanner.skip(1);
      tokens.push({ kind: "bracket", isImage: false, active: true, sourcePos: scanner.pos });
      continue;
    }

    // ── Link/image closer `]` ──────────────────────────────────────────────
    //
    // When we see `]`, we look for the nearest active bracket opener.
    // If we find one and can parse a valid link destination/reference
    // after the `]`, we build a link or image node. Otherwise we emit
    // literal `]` text and deactivate the opener to avoid re-matching.
    if (ch === "]") {
      scanner.skip(1);

      // CommonMark §6.3: when a link is formed inside brackets, the enclosing
      // non-image `[` opener is deactivated (marked active=false). That `]`
      // is the "matching" bracket for the deactivated opener. Per spec, such
      // a `]` must NOT skip over the deactivated opener to find an outer `![`
      // image opener — doing so would cause the image to absorb the wrong
      // destination. Instead we consume the deactivated opener here (treating
      // this `]` as its match), emit literal `]`, and remove it from the stack.
      //
      // Example: `![[[foo](uri1)](uri2)](uri3)`
      //   After `[foo](uri1)` forms, the outer `[` is deactivated.
      //   The `]` before `(uri2)` is its matching bracket → literal `]`.
      //   The `]` before `(uri3)` then correctly closes the `![` image opener.
      if (bracketStack.length > 0) {
        const topIdx = bracketStack[bracketStack.length - 1]!;
        const topTok = tokens[topIdx];
        if (topTok?.kind === "bracket" && !(topTok as BracketToken).active && !(topTok as BracketToken).isImage) {
          bracketStack.splice(bracketStack.length - 1, 1);
          textBuf += "]";
          continue;
        }
      }

      const openerStackIdx = findActiveBracketOpener(bracketStack, tokens);

      if (openerStackIdx === -1) {
        textBuf += "]";
        continue;
      }

      const openerTokenIdx = bracketStack[openerStackIdx]!;
      const opener = tokens[openerTokenIdx] as BracketToken;

      // IMPORTANT: flush textBuf before collecting inner tokens, otherwise
      // characters accumulated in textBuf won't appear in innerTokensBefore.
      // For example, in `[foo][]`, "foo" would still be in textBuf when we
      // hit `]`, so we must flush it now.
      flushText();

      // Collect the inner text (between opener and here) from the token list.
      // We need this to:
      //   a) render the link text / image alt
      //   b) use as the label for collapsed/shortcut reference links
      const innerTokensBefore = tokens.slice(openerTokenIdx + 1);
      // Use raw source text for the label — spec §4.7 says no backslash
      // escaping is performed when matching labels, so we must compare
      // the un-processed source (e.g. `\]` stays `\]`, not `]`).
      const closerPos = scanner.pos - 1; // position of the `]` we just consumed
      const innerTextForLabel = scanner.source.slice(opener.sourcePos, closerPos);

      const linkResult = tryLinkAfterClose(scanner, linkRefs, innerTextForLabel);

      if (linkResult === null) {
        // No valid link — deactivate the opener and emit literal `]`
        opener.active = false;
        bracketStack.splice(openerStackIdx, 1);
        textBuf += "]";
        continue;
      }

      // Valid link/image: resolve the inner tokens into inline nodes.
      flushText();

      // Extract all tokens after the opener (the link text/alt content).
      const innerTokens = tokens.splice(openerTokenIdx + 1);
      // Remove the opener from the tokens array (it is now at openerTokenIdx).
      tokens.splice(openerTokenIdx, 1);
      // Remove the opener from the bracket stack.
      bracketStack.splice(openerStackIdx, 1);

      const innerNodes = resolveEmphasis(innerTokens);

      if (opener.isImage) {
        const altText = extractPlainText(innerNodes);
        tokens.push({
          kind: "node",
          node: {
            type: "image",
            destination: linkResult.destination,
            title: linkResult.title,
            alt: altText,
          } as ImageNode,
        });
      } else {
        tokens.push({
          kind: "node",
          node: {
            type: "link",
            destination: linkResult.destination,
            title: linkResult.title,
            children: innerNodes,
          } as LinkNode,
        });
        // CommonMark §6.3: links cannot contain other links.
        // After forming a link, deactivate ALL preceding non-image link openers
        // in the bracket stack. This prevents the outer `[` from forming a link
        // that would contain this inner link.
        // Example: [foo [bar](/uri)](/uri) — the outer [ is deactivated so the
        // outer ](/uri) is treated as literal text.
        for (let k = bracketStack.length - 1; k >= 0; k--) {
          const idx = bracketStack[k]!;
          const t = tokens[idx];
          if (t?.kind === "bracket" && !(t as BracketToken).isImage) {
            (t as BracketToken).active = false;
          }
        }
      }
      continue;
    }

    // ── 8. Emphasis / strong delimiter run ────────────────────────────────
    if (ch === "*" || ch === "_" || (ch === "~" && scanner.peek(1) === "~")) {
      flushText();
      const delim = scanDelimiterRun(scanner, ch as "*" | "_" | "~");
      tokens.push(delim);
      continue;
    }

    // ── 6 & 7. Hard break (two+ trailing spaces before newline) ───────────
    //         or soft break (single newline).
    if (ch === "\n") {
      scanner.skip(1);
      if (textBuf.endsWith("  ") || /[ \t]{2,}$/.test(textBuf)) {
        textBuf = textBuf.replace(/[ \t]+$/, "");
        flushText();
        tokens.push({ kind: "node", node: { type: "hard_break" } as HardBreakNode });
      } else {
        textBuf = textBuf.trimEnd();
        flushText();
        tokens.push({ kind: "node", node: { type: "soft_break" } as SoftBreakNode });
      }
      continue;
    }

    // ── Regular character ──────────────────────────────────────────────────
    textBuf += scanner.advance();
  }

  flushText();

  // ─── Resolve Phase ────────────────────────────────────────────────────────
  return resolveEmphasis(tokens);
}

// ─── Delimiter Run Scanning ────────────────────────────────────────────────────

/**
 * Scan a delimiter run of `*`, `_`, or `~` starting at the current scanner
 * position. Returns a DelimiterToken with the flanking classification.
 *
 * The four flanking variables are derived from the characters immediately
 * before (preChar) and after (postChar) the run:
 *
 *   preChar  — character just before the run's first delimiter (or "" at BOL)
 *   postChar — character just after the run's last delimiter (or "" at EOL)
 *
 * A blank preChar/postChar counts as whitespace for the flanking rules.
 */
function scanDelimiterRun(scanner: Scanner, char: "*" | "_" | "~"): DelimiterToken {
  const source = scanner.source;
  const runStart = scanner.pos;
  const preChar = runStart > 0 ? source[runStart - 1]! : "";

  const run = scanner.consumeWhile(c => c === char);
  const count = run.length;
  const postChar = scanner.pos < source.length ? source[scanner.pos]! : "";

  const afterWhitespace  = postChar === "" || isUnicodeWhitespace(postChar);
  const afterPunctuation = postChar !== "" && isUnicodePunctuation(postChar);
  const beforeWhitespace = preChar  === "" || isUnicodeWhitespace(preChar);
  const beforePunctuation = preChar !== "" && isUnicodePunctuation(preChar);

  // Left-flanking: not followed by whitespace AND
  //   (not followed by punctuation OR preceded by whitespace/punctuation)
  const leftFlanking =
    !afterWhitespace &&
    (!afterPunctuation || beforeWhitespace || beforePunctuation);

  // Right-flanking: not preceded by whitespace AND
  //   (not preceded by punctuation OR followed by whitespace/punctuation)
  const rightFlanking =
    !beforeWhitespace &&
    (!beforePunctuation || afterWhitespace || afterPunctuation);

  // Per CommonMark spec §6.4 emphasis rules:
  //
  // `*` rules (rules 1 & 2):
  //   - can open  iff left-flanking  AND (not right-flanking OR preceded by ASCII punctuation)
  //   - can close iff right-flanking AND (not left-flanking  OR followed by ASCII punctuation)
  //   The extra conditions use ASCII punctuation (not full Unicode punctuation).
  //
  // `_` rules (rules 3 & 4):
  //   - can open  iff left-flanking  AND (not right-flanking OR preceded by Unicode punctuation)
  //   - can close iff right-flanking AND (not left-flanking  OR followed by Unicode punctuation)
  //   These stricter rules prevent intra-word emphasis in identifiers like `foo_bar_baz`.
  let canOpen: boolean;
  let canClose: boolean;

  if (char === "*") {
    canOpen  = leftFlanking;
    canClose = rightFlanking;
  } else if (char === "~") {
    canOpen = count >= 2 && leftFlanking;
    canClose = count >= 2 && rightFlanking;
  } else {
    canOpen  = leftFlanking  && (!rightFlanking || beforePunctuation);
    canClose = rightFlanking && (!leftFlanking  || afterPunctuation);
  }

  return { kind: "delimiter", char, count, canOpen, canClose, active: true };
}

// ─── Emphasis Resolution ───────────────────────────────────────────────────────
//
// Implements the CommonMark Appendix A delimiter stack algorithm.
//
// We walk the token list left-to-right looking for closers. For each closer
// we search backwards for the nearest compatible opener (same character, can
// open). When a pair is found we wrap the tokens between them in an emphasis
// or strong node and continue scanning.
//
// Key rules from the spec:
//
//   1. Opener and closer must use the same character (* or _).
//   2. We prefer strong (length 2) over emphasis (length 1) when both sides
//      have enough characters.
//   3. Mod-3 rule: if the sum of opener+closer lengths is divisible by 3,
//      and either side can BOTH open and close, the pair is invalid — UNLESS
//      both lengths are individually divisible by 3.
//   4. After matching, remaining delimiter characters stay as new delimiters.

function resolveEmphasis(tokens: InlineToken[]): InlineNode[] {
  let i = 0;
  while (i < tokens.length) {
    const token = tokens[i]!;
    if (token.kind !== "delimiter" || !token.canClose || !token.active) {
      i++;
      continue;
    }

    const closer = token;

    // Search backwards for an opener
    let openerIdx = -1;
    for (let j = i - 1; j >= 0; j--) {
      const t = tokens[j]!;
      if (t.kind !== "delimiter" || !t.canOpen || !t.active || t.char !== closer.char) {
        continue;
      }
      // Mod-3 rule: if either side can both open and close, and sum % 3 === 0,
      // skip unless both individually divide by 3.
      if ((t.canOpen && t.canClose) || (closer.canOpen && closer.canClose)) {
        if ((t.count + closer.count) % 3 === 0 && t.count % 3 !== 0) {
          continue;
        }
      }
      openerIdx = j;
      break;
    }

    if (openerIdx === -1) {
      i++;
      continue;
    }

    const opener = tokens[openerIdx] as DelimiterToken;

    // How many delimiter characters do we consume?
    // If both sides have 2+, use strong (2). Otherwise use emphasis (1).
    const useLen = closer.char === "~"
      ? 2
      : opener.count >= 2 && closer.count >= 2 ? 2 : 1;
    const isStrong = useLen === 2;

    // Collect inner tokens (between opener and closer), recursively resolve them.
    // Note: we take tokens BETWEEN opener and closer, not including either.
    const innerSlice = tokens.slice(openerIdx + 1, i);
    const innerNodes = resolveEmphasis(innerSlice);

    const emphNode: InlineNode = closer.char === "~"
      ? { type: "strikethrough", children: innerNodes } as StrikethroughNode
      : isStrong
      ? { type: "strong",   children: innerNodes } as StrongNode
      : { type: "emphasis", children: innerNodes } as EmphasisNode;

    // Replace ONLY the inner tokens with the emphasis node. Do NOT remove
    // the closer yet — it may have remaining characters (e.g. `**foo***` →
    // the closer `***` uses 2 chars for strong, leaving 1 `*` as closer for
    // a potential outer emphasis).
    //
    // splice(openerIdx+1, i-openerIdx-1, emphNode):
    //   start = openerIdx+1  (first inner token)
    //   count = i - openerIdx - 1  (number of inner tokens, NOT including closer)
    //   insert the emphNode in their place
    tokens.splice(openerIdx + 1, i - openerIdx - 1, { kind: "node", node: emphNode });
    // After this splice, the closer is now at openerIdx + 2
    // (because emphNode is at openerIdx+1 and closer shifted left by (inner count - 1)).

    // Reduce delimiter counts
    opener.count -= useLen;
    closer.count -= useLen;

    // If opener count is now 0, remove it.
    if (opener.count === 0) {
      tokens.splice(openerIdx, 1);
      // After removing opener, emphNode is at openerIdx, closer is at openerIdx+1.
      // Set i to re-examine the closer (in case it can close another outer opener).
      i = openerIdx + 1;
    } else {
      // Opener still has characters; emphNode is at openerIdx+1, closer at openerIdx+2.
      i = openerIdx + 2;
    }

    // If closer count is now 0, remove it.
    if (closer.count === 0) {
      tokens.splice(i, 1);
      // closer was at i; removing it means we stay at same i for the next iteration.
    }
    // Otherwise, re-examine the closer (still has characters, may match another opener).
    continue;
  }

  // Convert remaining tokens to InlineNodes
  return tokens.flatMap(tok => {
    if (tok.kind === "node")    return [tok.node];
    if (tok.kind === "bracket") return [{ type: "text", value: tok.isImage ? "![" : "[" } as TextNode];
    // Unused delimiter run — literal text
    return [{ type: "text", value: tok.char.repeat(tok.count) } as TextNode];
  });
}

// ─── Code Span Parsing ─────────────────────────────────────────────────────────

/**
 * Attempt to parse a code span starting at the scanner's current position.
 *
 * A code span opens with a run of N backticks and closes with the next
 * run of exactly N backticks. Mismatched runs are not code spans.
 *
 * Content normalisation (per spec §6.1):
 *   1. CR/LF/newline → space
 *   2. If the content has a non-space character AND starts and ends with
 *      exactly one space, strip those surrounding spaces.
 *
 * Example:
 *   `foo`         → { type: "code_span", value: "foo" }
 *   `` foo `` bar → { type: "code_span", value: "foo `` bar" }
 *   `  foo  `     → { type: "code_span", value: " foo " }   (two spaces each side, only one stripped)
 */
function tryCodeSpan(scanner: Scanner): CodeSpanNode | null {
  const savedPos = scanner.pos;

  const openTicks = scanner.consumeWhile(c => c === "`");
  const tickLen = openTicks.length;

  let content = "";
  while (!scanner.done) {
    if (scanner.peek() === "`") {
      const closePos = scanner.pos;
      const closeTicks = scanner.consumeWhile(c => c === "`");
      if (closeTicks.length === tickLen) {
        // Matching close found
        // Normalise line endings → spaces
        content = content.replace(/\r\n|\r|\n/g, " ");
        // Strip one leading+trailing space if content is not all-space
        if (
          content.length >= 2 &&
          content[0] === " " &&
          content[content.length - 1] === " " &&
          content.trim() !== ""
        ) {
          content = content.slice(1, -1);
        }
        return { type: "code_span", value: content };
      }
      // Wrong number of backticks — treat as content
      content += closeTicks;
    } else {
      content += scanner.advance();
    }
  }

  // No matching close found
  scanner.pos = savedPos;
  return null;
}

// ─── HTML Inline Parsing ───────────────────────────────────────────────────────

/**
 * Attempt to parse an inline HTML construct starting at `<`.
 *
 * CommonMark spec §6.6 defines six inline HTML forms:
 *
 *   1. Open tag:           `<tagname attr="val">`
 *   2. Closing tag:        `</tagname>`
 *   3. HTML comment:       `<!-- content -->`
 *   4. Processing instr:   `<?content?>`
 *   5. Declaration:        `<!UPPER content>`
 *   6. CDATA section:      `<![CDATA[content]]>`
 *
 * Content is passed through verbatim — no entity decoding, no recursion.
 *
 * Restrictions (from spec): tag names must start with ASCII alpha; no
 * newlines inside tags (except in comments/PI/CDATA); comment content
 * cannot start with `>` or `->` and cannot contain `--`.
 */
function tryHtmlInline(scanner: Scanner): RawInlineNode | null {
  if (scanner.peek() !== "<") return null;
  const savedPos = scanner.pos;
  scanner.skip(1); // consume `<`

  const ch = scanner.peek();

  // HTML comment: <!-- ... -->
  // Rules (CommonMark §6.11):
  //   - Content must not start with `>` or `->`
  //   - Content must not end with `-` (prevents `---->`)
  //   - Content cannot contain `-->` except as the closing sequence
  if (scanner.match("!--")) {
    const contentStart = scanner.pos;
    if (scanner.peek() === ">" || scanner.peekSlice(2) === "->") {
      // CommonMark §6.6: comment content must not start with `>` or `->`.
      // cmark's behaviour is to emit the opening `<!--` together with the
      // invalid starter (`>` or `->`) as a raw HTML fragment rather than
      // escaping `<` as `&lt;`. For example, `<!--> foo -->` produces
      // `<!--> foo --&gt;` in the rendered HTML.
      // Consume `>` (1 char) or `->` (2 chars) as part of the construct.
      const invalid = scanner.peek() === ">" ? ">" : "->";
      scanner.skip(invalid.length);
      return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
    }
    while (!scanner.done) {
      if (scanner.match("-->")) {
        // Check rule: content must not end with `-`
        const content = scanner.source.slice(contentStart, scanner.pos - 3);
        if (content.endsWith("-")) { scanner.pos = savedPos; return null; }
        return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
      }
      scanner.skip(1);
    }
    scanner.pos = savedPos;
    return null;
  }

  // Processing instruction: <? ... ?>
  if (scanner.match("?")) {
    while (!scanner.done) {
      if (scanner.match("?>")) {
        return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
      }
      scanner.skip(1);
    }
    scanner.pos = savedPos;
    return null;
  }

  // CDATA section: <![CDATA[ ... ]]>
  if (scanner.match("![CDATA[")) {
    while (!scanner.done) {
      if (scanner.match("]]>")) {
        return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
      }
      scanner.skip(1);
    }
    scanner.pos = savedPos;
    return null;
  }

  // Declaration: <!UPPER...>
  if (scanner.match("!")) {
    if (/[A-Z]/.test(scanner.peek())) {
      scanner.consumeWhile(c => c !== ">");
      if (scanner.match(">")) {
        return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
      }
    }
    scanner.pos = savedPos;
    return null;
  }

  // Closing tag: </tagname>
  if (ch === "/") {
    scanner.skip(1);
    const tag = scanner.consumeWhile(c => /[a-zA-Z0-9\-]/.test(c));
    if (tag.length === 0) { scanner.pos = savedPos; return null; }
    scanner.skipSpaces();
    if (!scanner.match(">")) { scanner.pos = savedPos; return null; }
    return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
  }

  // Open tag: <tagname attr...> or <tagname attr.../>
  //
  // Per CommonMark §6.11, an open tag may contain at most one line ending
  // anywhere in its attribute area (between attributes OR inside a quoted
  // attribute value).  Each attribute must be preceded by whitespace.
  if (/[a-zA-Z]/.test(ch)) {
    const tagName = scanner.consumeWhile(c => /[a-zA-Z0-9\-]/.test(c));
    if (tagName.length === 0) { scanner.pos = savedPos; return null; }

    // Track newlines consumed so far in this tag (max 1 allowed total).
    let newlinesInTag = 0;

    while (true) {
      let spaceLen = scanner.skipSpaces();
      // Allow at most one newline anywhere in the attribute area.
      if (newlinesInTag === 0 && scanner.peek() === "\n") {
        newlinesInTag++;
        scanner.skip(1);
        spaceLen += 1 + scanner.skipSpaces();
      }
      const next = scanner.peek();
      if (next === ">" || next === "/" || next === "") break;
      // Second newline → invalid tag.
      if (next === "\n") { scanner.pos = savedPos; return null; }
      // Each attribute must be preceded by whitespace.
      if (spaceLen === 0) { scanner.pos = savedPos; return null; }

      // Attribute name: must start with ASCII alpha, `_`, or `:`.
      if (!/[a-zA-Z_:]/.test(next)) { scanner.pos = savedPos; return null; }
      scanner.consumeWhile(c => /[a-zA-Z0-9_:\.\-]/.test(c));

      // Optional `= value`.  Only consume the spaces around `=` if `=` is
      // actually present; otherwise leave them for the next loop iteration.
      const posBeforeEqSpaces = scanner.pos;
      scanner.skipSpaces();
      if (scanner.peek() === "=") {
        scanner.skip(1); // consume `=`
        scanner.skipSpaces();
        const q = scanner.peek();
        if (q === '"' || q === "'") {
          scanner.skip(1);
          // Scan until closing quote; allow one newline inside the value.
          let closed = false;
          while (!scanner.done) {
            const vc = scanner.source[scanner.pos]!;
            if (vc === q) { scanner.skip(1); closed = true; break; }
            if (vc === "\n") {
              if (newlinesInTag >= 1) { scanner.pos = savedPos; return null; }
              newlinesInTag++;
            }
            scanner.skip(1);
          }
          if (!closed) { scanner.pos = savedPos; return null; }
        } else {
          // Unquoted value: no whitespace, `"`, `'`, `=`, `<`, `>`, `` ` ``
          const unquoted = scanner.consumeWhile(c => !/[\s"'=<>`]/.test(c));
          if (unquoted.length === 0) { scanner.pos = savedPos; return null; }
        }
      } else {
        // No `=`: restore position to before the trailing spaces so the
        // next loop iteration can find them as inter-attribute whitespace.
        scanner.pos = posBeforeEqSpaces;
      }
    }

    const selfClose = scanner.match("/>");
    if (!selfClose && !scanner.match(">")) { scanner.pos = savedPos; return null; }
    return { type: "raw_inline", format: "html", value: scanner.source.slice(savedPos, scanner.pos) };
  }

  scanner.pos = savedPos;
  return null;
}

// ─── Autolink Parsing ─────────────────────────────────────────────────────────

/**
 * Attempt to parse an autolink: `<URI>` or `<email>`.
 *
 * URL autolink: `<scheme:path>` where scheme is 2–32 chars from
 * `[a-zA-Z0-9+.-]`, and path has no spaces or `<` or `>`.
 *
 * Email autolink: `<user@domain>` where the address matches a simple
 * RFC-5322-ish pattern (CommonMark spec §6.7).
 */
function tryAutolink(scanner: Scanner): AutolinkNode | null {
  if (scanner.peek() !== "<") return null;
  const savedPos = scanner.pos;
  scanner.skip(1);

  const start = scanner.pos;

  // Try email autolink: local@domain
  const localPart = scanner.consumeWhile(c => /[^\s<>@]/.test(c));
  if (localPart.length > 0 && scanner.peek() === "@") {
    scanner.skip(1);
    const domainPart = scanner.consumeWhile(c => /[^\s<>]/.test(c));
    if (domainPart.length > 0 && scanner.match(">")) {
      // Validate email local part and domain
      if (
        /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$/.test(localPart) &&
        /^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/.test(domainPart)
      ) {
        return {
          type: "autolink",
          destination: localPart + "@" + domainPart,
          isEmail: true,
        };
      }
    }
  }

  // Retry as URL autolink
  scanner.pos = start;
  const scheme = scanner.consumeWhile(c => /[a-zA-Z0-9+\-.]/.test(c));
  if (scheme.length >= 2 && scheme.length <= 32 && scanner.match(":")) {
    // Path: anything except spaces, `<`, `>`
    const path = scanner.consumeWhile(c => c !== " " && c !== "<" && c !== ">" && c !== "\n");
    if (scanner.match(">")) {
      return {
        type: "autolink",
        destination: scheme + ":" + path,
        isEmail: false,
      };
    }
  }

  scanner.pos = savedPos;
  return null;
}

// ─── Link / Image Destination Parsing ────────────────────────────────────────

interface LinkResult {
  destination: string;
  title: string | null;
}

/**
 * After the `]` of a potential link/image bracket, try to parse:
 *
 *   Inline:     `(destination "title")` or `(destination)`
 *   Full ref:   `[label]`
 *   Collapsed:  `[]`               — uses `innerText` as the label
 *   Shortcut:   (nothing)          — uses `innerText` as the label
 *
 * Returns null if no valid link form is recognised.
 *
 * @param scanner    Positioned just after the `]`.
 * @param linkRefs   The link reference map.
 * @param innerText  The plain-text content of the brackets (for reference forms).
 */
function tryLinkAfterClose(
  scanner: Scanner,
  linkRefs: LinkRefMap,
  innerText: string,
): LinkResult | null {
  const savedPos = scanner.pos;

  // ── Inline link: ( destination "title" ) ────────────────────────────────
  //
  // If the `(...)` is present but does not form a valid inline link, we fall
  // through to the reference-link checks below (spec §6.3: if an inline link
  // fails, try collapsed then shortcut reference). For example:
  //   `[foo](not a link)` with `[foo]: /url1` → shortcut reference wins.
  if (scanner.peek() === "(") {
    // Attempt inline link. On any failure reset to savedPos and fall through.
    const inlineLinkResult = (() => {
      scanner.skip(1); // consume `(`
      skipOptionalSpacesAndNewline(scanner);

      let destination = "";

      if (scanner.peek() === "<") {
        // Angle-bracket destination: `<url>` — no line endings or bare `<`.
        scanner.skip(1);
        let destBuf = "";
        while (!scanner.done) {
          const c = scanner.peek();
          if (c === "\n" || c === "\r") return null;
          if (c === "\\") {
            scanner.skip(1);
            const next = scanner.advance();
            destBuf += isAsciiPunctuation(next) ? next : "\\" + next;
          } else if (c === ">") {
            scanner.skip(1);
            break;
          } else if (c === "<") {
            return null;
          } else {
            destBuf += scanner.advance();
          }
        }
        destination = normalizeUrl(decodeEntities(destBuf));
      } else {
        // Bare destination — no spaces, balanced parens, backslash-escapes.
        let depth = 0;
        const destStart = scanner.pos;
        while (!scanner.done) {
          const c = scanner.peek();
          if (c === "(")           { depth++; scanner.skip(1); }
          else if (c === ")")      { if (depth === 0) break; depth--; scanner.skip(1); }
          else if (c === "\\")     { scanner.skip(2); }
          else if (isAsciiWhitespace(c)) { break; }
          else                     { scanner.skip(1); }
        }
        const destRaw = scanner.source.slice(destStart, scanner.pos);
        destination = normalizeUrl(decodeEntities(applyBackslashEscapes(destRaw)));
      }

      skipOptionalSpacesAndNewline(scanner);

      // Optional title
      let title: string | null = null;
      const q = scanner.peek();
      if (q === '"' || q === "'" || q === "(") {
        const closeQ = q === "(" ? ")" : q;
        scanner.skip(1);
        let titleBuf = "";
        while (!scanner.done) {
          const c = scanner.peek();
          if (c === "\\") {
            scanner.skip(1);
            const next = scanner.advance();
            titleBuf += isAsciiPunctuation(next) ? next : "\\" + next;
          } else if (c === closeQ) {
            scanner.skip(1);
            title = decodeEntities(titleBuf);
            break;
          } else if (c === "\n" && q === "(") {
            break; // parens title cannot span lines
          } else {
            titleBuf += scanner.advance();
          }
        }
      }

      scanner.skipSpaces();
      if (!scanner.match(")")) return null;
      return { destination, title };
    })();

    if (inlineLinkResult !== null) return inlineLinkResult;
    // Inline link failed — reset and fall through to reference checks.
    scanner.pos = savedPos;
  }

  // ── Full reference: [label] or Collapsed reference: [] ──────────────────
  //
  // Link labels may contain backslash-escaped punctuation (e.g. `\!` → `!`)
  // but NOT unescaped `[` (spec §4.7: brackets must be balanced or escaped).
  if (scanner.peek() === "[") {
    scanner.skip(1);
    let labelBuf = "";
    let validLabel = true;
    while (!scanner.done) {
      const c = scanner.peek();
      if (c === "]") { scanner.skip(1); break; }
      if (c === "\n" || c === "[") { validLabel = false; break; }
      if (c === "\\") {
        scanner.skip(1);
        if (!scanner.done) {
          labelBuf += "\\" + scanner.advance();  // include backslash + next char verbatim
        }
      } else {
        labelBuf += scanner.advance();
      }
    }
    if (validLabel) {
      if (labelBuf.trim() !== "") {
        const label = normalizeLinkLabel(labelBuf);
        const ref = linkRefs.get(label);
        if (ref) return { destination: ref.destination, title: ref.title };
      } else {
        // Collapsed reference: [] — use inner text as the label
        const label = normalizeLinkLabel(innerText);
        const ref = linkRefs.get(label);
        if (ref) return { destination: ref.destination, title: ref.title };
      }
    }
    scanner.pos = savedPos;
    return null;
  }

  // ── Shortcut reference: no `(` or `[` follows — use inner text as label ──
  const label = normalizeLinkLabel(innerText);
  const ref = linkRefs.get(label);
  if (ref) return { destination: ref.destination, title: ref.title };

  return null;
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/**
 * Apply backslash escapes: replace `\X` with `X` only when X is ASCII
 * punctuation. Non-punctuation backslash sequences are left as-is.
 *
 * CommonMark §2.4: "Any ASCII punctuation character may be backslash-escaped.
 * All other characters preceded by a backslash are treated literally."
 *
 * Examples:
 *   `\*`  → `*`   (punctuation escape)
 *   `\\`  → `\`   (backslash is punctuation)
 *   `\b`  → `\b`  (not punctuation — kept as-is)
 */
function applyBackslashEscapes(s: string): string {
  return s.replace(/\\(.)/g, (match, ch: string) =>
    isAsciiPunctuation(ch) ? ch : match
  );
}

/**
 * Skip ASCII spaces/tabs and at most one line ending (LF or CRLF).
 * Used between link destination and title per CommonMark spec §6.3.
 */
function skipOptionalSpacesAndNewline(scanner: Scanner): void {
  scanner.skipSpaces();
  if (scanner.peek() === "\n") {
    scanner.skip(1);
    scanner.skipSpaces();
  } else if (scanner.peek() === "\r" && scanner.peek(1) === "\n") {
    scanner.skip(2);
    scanner.skipSpaces();
  }
}

/**
 * Find the index (into `bracketStack`) of the most recent active bracket
 * opener. Returns -1 if none exists.
 */
function findActiveBracketOpener(
  bracketStack: number[],
  tokens: InlineToken[],
): number {
  for (let i = bracketStack.length - 1; i >= 0; i--) {
    const idx = bracketStack[i]!;
    const t = tokens[idx];
    if (t?.kind === "bracket" && (t as BracketToken).active) {
      return i; // return the stack index, not the token index
    }
  }
  return -1;
}

/**
 * Extract a raw text string from a token list for use as a link label.
 * This is a simplified version that concatenates text values from node tokens.
 */
function extractRawTextForLabel(tokens: InlineToken[]): string {
  return tokens.flatMap(t => {
    if (t.kind === "node") return [extractPlainText([t.node])];
    if (t.kind === "delimiter") return [t.char.repeat(t.count)];
    if (t.kind === "bracket") return [t.isImage ? "![" : "["];
    return [""];
  }).join("");
}

/**
 * Recursively extract plain text from inline nodes.
 * Used for image `alt` attributes and link label fallback.
 *
 * CommonMark spec §6.4: "The content of the first sequence of brackets
 * is used as the alt text, but all elements except the characters
 * themselves are stripped."
 */
function extractPlainText(nodes: readonly InlineNode[]): string {
  let result = "";
  for (const node of nodes) {
    switch (node.type) {
      case "text":       result += node.value; break;
      case "code_span":  result += node.value; break;
      case "hard_break": result += "\n"; break;
      case "soft_break": result += " "; break;
      case "emphasis":
      case "strong":
      case "strikethrough":
      case "link":       result += extractPlainText(node.children); break;
      case "image":      result += node.alt; break;
      case "autolink":   result += node.destination; break;
      // html_inline: stripped (no visible text)
      default:           break;
    }
  }
  return result;
}

// ─── Document-Level Inline Resolution ─────────────────────────────────────────

/**
 * Walk the block AST produced by `convertToAst` and fill in inline content.
 *
 * The block parser attaches a `_rawId: symbol` to heading and paragraph
 * nodes instead of populating their `children` arrays — because the block
 * parser does not know about inline syntax. This function uses those symbols
 * as keys into `rawInlineContent` to retrieve the raw strings, parses them,
 * and writes the resulting `InlineNode[]` back into the node's `children`.
 *
 * After this function returns, the AST is complete and the `_rawId`
 * properties are removed.
 *
 * @param document          Root of the block AST.
 * @param rawInlineContent  Symbol → raw inline string map from convertToAst.
 * @param linkRefs          The link reference map from Phase 1.
 */
export function resolveInlineContent(
  document: DocumentNode,
  rawInlineContent: Map<symbol, string>,
  linkRefs: LinkRefMap,
): void {
  function walk(block: BlockNode): void {
    // Cast to a mutable version so we can fill in children and remove _rawId.
    const b = block as BlockNode & { _rawId?: symbol; children?: InlineNode[] | BlockNode[] };

    if ((b.type === "heading" || b.type === "paragraph" || b.type === "table_cell") && b._rawId !== undefined) {
      const raw = rawInlineContent.get(b._rawId);
      if (raw !== undefined) {
        (b as { children: InlineNode[] }).children = parseInline(raw, linkRefs);
      }
      delete b._rawId;
    }

    // Recurse into container blocks
    if ("children" in block && Array.isArray((block as { children: readonly unknown[] }).children)) {
      for (const child of (block as { children: readonly BlockNode[] }).children) {
        walk(child);
      }
    }
  }

  walk(document);
}
