"""Inline Parser.

Phase 2 of CommonMark parsing: scan raw inline content strings (produced
by the block parser) and emit inline AST nodes — emphasis, links, code
spans, etc.

=== Overview of Inline Constructs ===

CommonMark recognises ten inline constructs, processed left-to-right:

  1. Backslash escapes       \\*    → literal *
  2. HTML character refs     &amp;  → &
  3. Code spans              `code`
  4. HTML inline             <em>, <!-- -->, <?...?>
  5. Autolinks               <https://example.com>, <me@example.com>
  6. Hard line breaks        two trailing spaces + newline, or \\ + newline
  7. Soft line breaks        single newline within a paragraph
  8. Emphasis / strong       *em*, **strong**, _em_, __strong__
  9. Links                   [text](url), [text][label], [text][]
  10. Images                  ![alt](url), ![alt][label]

=== The Delimiter Stack Algorithm ===

Emphasis is the hardest part of CommonMark inline parsing. The rules are
context-sensitive: whether * or _ can open or close emphasis depends
on what precedes and follows the run. CommonMark Appendix A defines the
canonical "delimiter stack" algorithm.

The algorithm has two phases:

  A. SCAN — read the input left-to-right, building a flat list of "tokens":
     ordinary text, delimiter runs (* ** _ __), code spans, links, etc.
     Each delimiter run is tagged as "can_open", "can_close", or both.

  B. RESOLVE — walk the token list, matching openers with the nearest
     valid closers. For each matched pair, wrap the intervening tokens
     in an emphasis or strong node.

=== Flanking Rules (CommonMark spec §6.2) ===

A delimiter run of * is LEFT-FLANKING (can open) if:
  (a) not followed by Unicode whitespace, AND
  (b) either not followed by Unicode punctuation,
      OR preceded by Unicode whitespace or Unicode punctuation.

A delimiter run of * is RIGHT-FLANKING (can close) if:
  (a) not preceded by Unicode whitespace, AND
  (b) either not preceded by Unicode punctuation,
      OR followed by Unicode whitespace or Unicode punctuation.

For _, the open/close rules add extra conditions to avoid
intra-word emphasis:
  - _ can open only if left-flanking AND
    (preceded by whitespace/punctuation OR not right-flanking).
  - _ can close only if right-flanking AND
    (followed by whitespace/punctuation OR not left-flanking).
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from coding_adventures_document_ast import (
    AutolinkNode,
    BlockNode,
    CodeSpanNode,
    DocumentNode,
    ImageNode,
    InlineNode,
    LinkNode,
    RawInlineNode,
)

from coding_adventures_commonmark_parser.entities import decode_entities, decode_entity
from coding_adventures_commonmark_parser.scanner import (
    Scanner,
    is_ascii_punctuation,
    is_ascii_whitespace,
    is_unicode_punctuation,
    is_unicode_whitespace,
    normalize_link_label,
    normalize_url,
)

# ─── Delimiter Token Types ─────────────────────────────────────────────────────
#
# During the scan phase, the input is broken into a flat list of tokens.
# Delimiter runs (*/**/_/__) become DelimiterToken; everything else becomes
# a NodeToken wrapping a fully-resolved InlineNode. BracketToken marks
# open [ or ![ that may become link/image openers.


@dataclass
class DelimiterToken:
    """A delimiter run: maximal run of * or _."""
    kind: str = "delimiter"
    char: str = "*"
    count: int = 0      # length of the run
    can_open: bool = False    # left-flanking (may open emphasis)
    can_close: bool = False   # right-flanking (may close emphasis)
    active: bool = True  # False once consumed by the resolution pass


@dataclass
class NodeToken:
    """A fully-resolved inline node (produced during scanning)."""
    kind: str = "node"
    node: InlineNode = field(default_factory=lambda: {"type": "text", "value": ""})


@dataclass
class BracketToken:
    """A bracket opener [ or ![ — may become a link or image."""
    kind: str = "bracket"
    is_image: bool = False   # True if preceded by !
    active: bool = True       # False if deactivated
    source_pos: int = 0       # scanner position immediately after the [


InlineToken = DelimiterToken | NodeToken | BracketToken


# ─── Main Inline Parser ────────────────────────────────────────────────────────

def parse_inline(raw: str, link_refs: dict) -> list[InlineNode]:
    """Parse a raw inline content string into a list of InlineNode trees.

    This is the core Phase 2 function. It is called by `resolve_inline_content`
    for each paragraph and heading that contains inline markup.

    @param raw       The raw inline string from the block parser.
    @param link_refs  Link reference definitions collected in Phase 1.
    """
    scanner = Scanner(raw)
    tokens: list[InlineToken] = []

    # bracketStack holds the index into `tokens` of each open bracket.
    bracket_stack: list[int] = []

    # Text accumulation buffer — flush it into a NodeToken whenever
    # a non-text construct is encountered.
    text_buf = ""

    def flush_text() -> None:
        nonlocal text_buf
        if text_buf:
            tokens.append(NodeToken(node={"type": "text", "value": text_buf}))
            text_buf = ""

    # ─── Scan Phase ─────────────────────────────────────────────────────────────

    while not scanner.done:
        ch = scanner.peek()

        # ── 1. Backslash escape ────────────────────────────────────────────────
        #
        # \ followed by an ASCII punctuation character → the punctuation
        # is treated as a literal character (not a Markdown special).
        # \ followed by a newline → hard line break.
        # \ followed by anything else → literal backslash.
        if ch == "\\":
            next_ch = scanner.peek(1)
            if next_ch and is_ascii_punctuation(next_ch):
                scanner.skip(2)
                text_buf += next_ch
                continue
            if next_ch == "\n":
                scanner.skip(2)
                flush_text()
                tokens.append(NodeToken(node={"type": "hard_break"}))
                continue
            scanner.skip(1)
            text_buf += "\\"
            continue

        # ── 2. HTML character reference ────────────────────────────────────────
        #
        # &name;, &#NNN;, &#xHHH; → decoded Unicode character.
        # Unrecognised references are left as-is.
        if ch == "&":
            m = scanner.match_regex(_ENTITY_RE)
            if m is not None:
                text_buf += decode_entity(m)
                continue
            scanner.skip(1)
            text_buf += "&"
            continue

        # ── 3. Code span ───────────────────────────────────────────────────────
        if ch == "`":
            span = _try_code_span(scanner)
            if span is not None:
                flush_text()
                tokens.append(NodeToken(node=span))
                continue
            # Not a valid code span — literal backtick run
            ticks = scanner.consume_while(lambda c: c == "`")
            text_buf += ticks
            continue

        # ── 4 & 5. HTML inline and autolinks (both start with `<`) ────────────
        if ch == "<":
            autolink = _try_autolink(scanner)
            if autolink is not None:
                flush_text()
                tokens.append(NodeToken(node=autolink))
                continue
            html_inline = _try_html_inline(scanner)
            if html_inline is not None:
                flush_text()
                tokens.append(NodeToken(node=html_inline))
                continue
            scanner.skip(1)
            text_buf += "<"
            continue

        # ── Image opener ![ ────────────────────────────────────────────────────
        if ch == "!" and scanner.peek(1) == "[":
            flush_text()
            bracket_stack.append(len(tokens))
            scanner.skip(2)
            tokens.append(BracketToken(is_image=True, active=True, source_pos=scanner.pos))
            continue

        # ── Link opener [ ──────────────────────────────────────────────────────
        if ch == "[":
            flush_text()
            bracket_stack.append(len(tokens))
            scanner.skip(1)
            tokens.append(BracketToken(is_image=False, active=True, source_pos=scanner.pos))
            continue

        # ── Link/image closer ] ────────────────────────────────────────────────
        if ch == "]":
            scanner.skip(1)

            # CommonMark §6.3: when a link is formed inside brackets, the enclosing
            # non-image [ opener is deactivated. That ] is the "matching" bracket
            # for the deactivated opener. Per spec, such a ] must NOT skip over
            # the deactivated opener to find an outer ![ image opener.
            if bracket_stack:
                top_idx = bracket_stack[-1]
                top_tok = tokens[top_idx] if top_idx < len(tokens) else None
                if (top_tok and top_tok.kind == "bracket"
                        and not top_tok.active
                        and not top_tok.is_image):
                    bracket_stack.pop()
                    text_buf += "]"
                    continue

            opener_stack_idx = _find_active_bracket_opener(bracket_stack, tokens)

            if opener_stack_idx == -1:
                text_buf += "]"
                continue

            opener_token_idx = bracket_stack[opener_stack_idx]
            opener = tokens[opener_token_idx]

            # IMPORTANT: flush textBuf before collecting inner tokens
            flush_text()

            closer_pos = scanner.pos - 1  # position of the ] we just consumed
            inner_text_for_label = scanner.source[opener.source_pos:closer_pos]

            link_result = _try_link_after_close(scanner, link_refs, inner_text_for_label)

            if link_result is None:
                # No valid link — deactivate the opener and emit literal ]
                opener.active = False
                bracket_stack.pop(opener_stack_idx)
                text_buf += "]"
                continue

            # Valid link/image: resolve the inner tokens into inline nodes.
            flush_text()

            # Extract all tokens after the opener
            inner_tokens = tokens[opener_token_idx + 1:]
            del tokens[opener_token_idx + 1:]
            del tokens[opener_token_idx]
            bracket_stack.pop(opener_stack_idx)

            inner_nodes = _resolve_emphasis(inner_tokens)

            if opener.is_image:
                alt_text = _extract_plain_text(inner_nodes)
                img_node: ImageNode = {
                    "type": "image",
                    "destination": link_result["destination"],
                    "title": link_result["title"],
                    "alt": alt_text,
                }
                tokens.append(NodeToken(node=img_node))
            else:
                link_node: LinkNode = {
                    "type": "link",
                    "destination": link_result["destination"],
                    "title": link_result["title"],
                    "children": inner_nodes,
                }
                tokens.append(NodeToken(node=link_node))
                # CommonMark §6.3: links cannot contain other links.
                # After forming a link, deactivate ALL preceding non-image link openers.
                for k in range(len(bracket_stack) - 1, -1, -1):
                    idx = bracket_stack[k]
                    if idx < len(tokens):
                        t = tokens[idx]
                        if t.kind == "bracket" and not t.is_image:
                            t.active = False
            continue

        # ── 8. Emphasis / strong delimiter run ────────────────────────────────
        if ch in ("*", "_"):
            flush_text()
            delim = _scan_delimiter_run(scanner, ch)
            tokens.append(delim)
            continue

        # ── 6 & 7. Hard break (two+ trailing spaces before newline) ───────────
        #         or soft break (single newline).
        if ch == "\n":
            scanner.skip(1)
            if text_buf.endswith("  ") or re.search(r"[ \t]{2,}$", text_buf):
                text_buf = re.sub(r"[ \t]+$", "", text_buf)
                flush_text()
                tokens.append(NodeToken(node={"type": "hard_break"}))
            else:
                text_buf = text_buf.rstrip()
                flush_text()
                tokens.append(NodeToken(node={"type": "soft_break"}))
            continue

        # ── Regular character ──────────────────────────────────────────────────
        text_buf += scanner.advance()

    flush_text()

    # ─── Resolve Phase ────────────────────────────────────────────────────────
    return _resolve_emphasis(tokens)


# Pre-compiled entity regex for inline parsing
_ENTITY_RE = re.compile(
    r"&(?:#[xX][0-9a-fA-F]{1,6}|#[0-9]{1,7}|[a-zA-Z][a-zA-Z0-9]{0,31});"
)


# ─── Delimiter Run Scanning ────────────────────────────────────────────────────

def _scan_delimiter_run(scanner: Scanner, char: str) -> DelimiterToken:
    """Scan a delimiter run of * or _ starting at the current scanner position.

    Returns a DelimiterToken with the flanking classification.

    The four flanking variables are derived from the characters immediately
    before (pre_char) and after (post_char) the run:

      pre_char  — character just before the run's first delimiter (or "" at BOL)
      post_char — character just after the run's last delimiter (or "" at EOL)

    A blank pre_char/post_char counts as whitespace for the flanking rules.
    """
    source = scanner.source
    run_start = scanner.pos
    pre_char = source[run_start - 1] if run_start > 0 else ""

    run = scanner.consume_while(lambda c: c == char)
    count = len(run)
    post_char = source[scanner.pos] if scanner.pos < len(source) else ""

    after_whitespace = not post_char or is_unicode_whitespace(post_char)
    after_punctuation = bool(post_char) and is_unicode_punctuation(post_char)
    before_whitespace = not pre_char or is_unicode_whitespace(pre_char)
    before_punctuation = bool(pre_char) and is_unicode_punctuation(pre_char)

    # Left-flanking: not followed by whitespace AND
    #   (not followed by punctuation OR preceded by whitespace/punctuation)
    left_flanking = (
        not after_whitespace
        and (not after_punctuation or before_whitespace or before_punctuation)
    )

    # Right-flanking: not preceded by whitespace AND
    #   (not preceded by punctuation OR followed by whitespace/punctuation)
    right_flanking = (
        not before_whitespace
        and (not before_punctuation or after_whitespace or after_punctuation)
    )

    # Per CommonMark spec §6.4 emphasis rules:
    #
    # `*` rules (rules 1 & 2):
    #   - can open  iff left-flanking  AND (not right-flanking OR preceded by ASCII punctuation)
    #   - can close iff right-flanking AND (not left-flanking  OR followed by ASCII punctuation)
    #   The extra conditions use ASCII punctuation (not full Unicode punctuation).
    #
    # `_` rules (rules 3 & 4):
    #   - can open  iff left-flanking  AND (not right-flanking OR preceded by Unicode punctuation)
    #   - can close iff right-flanking AND (not left-flanking  OR followed by Unicode punctuation)
    #   These stricter rules prevent intra-word emphasis in identifiers like foo_bar_baz.
    if char == "*":
        can_open = left_flanking
        can_close = right_flanking
    else:  # _
        can_open = left_flanking and (not right_flanking or before_punctuation)
        can_close = right_flanking and (not left_flanking or after_punctuation)

    return DelimiterToken(char=char, count=count, can_open=can_open, can_close=can_close, active=True)


# ─── Emphasis Resolution ───────────────────────────────────────────────────────
#
# Implements the CommonMark Appendix A delimiter stack algorithm.
#
# We walk the token list left-to-right looking for closers. For each closer
# we search backwards for the nearest compatible opener (same character, can
# open). When a pair is found we wrap the tokens between them in an emphasis
# or strong node and continue scanning.
#
# Key rules from the spec:
#
#   1. Opener and closer must use the same character (* or _).
#   2. We prefer strong (length 2) over emphasis (length 1) when both sides
#      have enough characters.
#   3. Mod-3 rule: if the sum of opener+closer lengths is divisible by 3,
#      and either side can BOTH open and close, the pair is invalid — UNLESS
#      both lengths are individually divisible by 3.
#   4. After matching, remaining delimiter characters stay as new delimiters.


def _resolve_emphasis(tokens: list[InlineToken]) -> list[InlineNode]:
    """Resolve emphasis/strong delimiter pairs in the token list.

    Mutates the `tokens` list in-place as part of resolution.
    Returns the resulting list of InlineNode values.
    """
    i = 0
    while i < len(tokens):
        token = tokens[i]
        if token.kind != "delimiter" or not token.can_close or not token.active:
            i += 1
            continue

        closer = token

        # Search backwards for an opener
        opener_idx = -1
        for j in range(i - 1, -1, -1):
            t = tokens[j]
            if t.kind != "delimiter" or not t.can_open or not t.active or t.char != closer.char:
                continue
            # Mod-3 rule: if either side can both open and close, and sum % 3 === 0,
            # skip unless both individually divide by 3.
            if (t.can_open and t.can_close) or (closer.can_open and closer.can_close):
                if (t.count + closer.count) % 3 == 0 and t.count % 3 != 0:
                    continue
            opener_idx = j
            break

        if opener_idx == -1:
            i += 1
            continue

        opener = tokens[opener_idx]

        # How many delimiter characters do we consume?
        # If both sides have 2+, use strong (2). Otherwise use emphasis (1).
        use_len = 2 if opener.count >= 2 and closer.count >= 2 else 1
        is_strong = use_len == 2

        # Collect inner tokens (between opener and closer), recursively resolve them.
        inner_slice = list(tokens[opener_idx + 1:i])
        inner_nodes = _resolve_emphasis(inner_slice)

        emph_node: InlineNode
        if is_strong:
            emph_node = {"type": "strong", "children": inner_nodes}
        else:
            emph_node = {"type": "emphasis", "children": inner_nodes}

        # Replace ONLY the inner tokens with the emphasis node.
        # splice(opener_idx+1, i-opener_idx-1, emph_node):
        #   start = opener_idx+1  (first inner token)
        #   count = i - opener_idx - 1  (number of inner tokens, NOT including closer)
        del tokens[opener_idx + 1:i]
        tokens.insert(opener_idx + 1, NodeToken(node=emph_node))
        # After this splice, the closer is now at opener_idx + 2

        # Reduce delimiter counts
        opener.count -= use_len
        closer.count -= use_len

        # If opener count is now 0, remove it.
        if opener.count == 0:
            del tokens[opener_idx]
            # After removing opener, emphNode is at opener_idx, closer is at opener_idx+1.
            i = opener_idx + 1
        else:
            # Opener still has characters; emphNode is at opener_idx+1, closer at opener_idx+2.
            i = opener_idx + 2

        # If closer count is now 0, remove it.
        if closer.count == 0:
            del tokens[i]
            # Removing it means we stay at same i for the next iteration
        # Otherwise, re-examine the closer (still has characters, may match another opener).
        continue

    # Convert remaining tokens to InlineNodes
    result: list[InlineNode] = []
    for tok in tokens:
        if tok.kind == "node":
            result.append(tok.node)
        elif tok.kind == "bracket":
            result.append({"type": "text", "value": "![" if tok.is_image else "["})
        else:
            # Unused delimiter run — literal text
            result.append({"type": "text", "value": tok.char * tok.count})
    return result


# ─── Code Span Parsing ─────────────────────────────────────────────────────────

def _try_code_span(scanner: Scanner) -> CodeSpanNode | None:
    """Attempt to parse a code span starting at the scanner's current position.

    A code span opens with a run of N backticks and closes with the next
    run of exactly N backticks. Mismatched runs are not code spans.

    Content normalisation (per spec §6.1):
      1. CR/LF/newline → space
      2. If the content has a non-space character AND starts and ends with
         exactly one space, strip those surrounding spaces.

    Example:
        `foo`         → { type: "code_span", value: "foo" }
        `` foo ``     → { type: "code_span", value: "foo" }
        `  foo  `     → { type: "code_span", value: " foo " }   (two spaces each side)
    """
    saved_pos = scanner.pos

    open_ticks = scanner.consume_while(lambda c: c == "`")
    tick_len = len(open_ticks)

    content = ""
    while not scanner.done:
        if scanner.peek() == "`":
            close_ticks = scanner.consume_while(lambda c: c == "`")
            if len(close_ticks) == tick_len:
                # Matching close found — normalise line endings → spaces
                content = re.sub(r"\r\n|\r|\n", " ", content)
                # Strip one leading+trailing space if content is not all-space
                if (len(content) >= 2
                        and content[0] == " "
                        and content[-1] == " "
                        and content.strip()):
                    content = content[1:-1]
                return {"type": "code_span", "value": content}
            # Wrong number of backticks — treat as content
            content += close_ticks
        else:
            content += scanner.advance()

    # No matching close found
    scanner.pos = saved_pos
    return None


# ─── HTML Inline Parsing ───────────────────────────────────────────────────────

def _try_html_inline(scanner: Scanner) -> RawInlineNode | None:
    """Attempt to parse an inline HTML construct starting at <.

    CommonMark spec §6.6 defines six inline HTML forms:

      1. Open tag:           <tagname attr="val">
      2. Closing tag:        </tagname>
      3. HTML comment:       <!-- content -->
      4. Processing instr:   <?content?>
      5. Declaration:        <!UPPER content>
      6. CDATA section:      <![CDATA[content]]>

    Content is passed through verbatim — no entity decoding, no recursion.

    Restrictions (from spec): tag names must start with ASCII alpha; no
    newlines inside tags (except in comments/PI/CDATA); comment content
    cannot start with > or -> and cannot contain --.
    """
    if scanner.peek() != "<":
        return None
    saved_pos = scanner.pos
    scanner.skip(1)  # consume <

    ch = scanner.peek()

    # HTML comment: <!-- ... -->
    if scanner.match("!--"):
        content_start = scanner.pos
        if scanner.peek() == ">" or scanner.peek_slice(2) == "->":
            invalid = ">" if scanner.peek() == ">" else "->"
            scanner.skip(len(invalid))
            return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}
        while not scanner.done:
            if scanner.match("-->"):
                content = scanner.source[content_start:scanner.pos - 3]
                if content.endswith("-"):
                    scanner.pos = saved_pos
                    return None
                return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}
            scanner.skip(1)
        scanner.pos = saved_pos
        return None

    # Processing instruction: <? ... ?>
    if scanner.match("?"):
        while not scanner.done:
            if scanner.match("?>"):
                return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}
            scanner.skip(1)
        scanner.pos = saved_pos
        return None

    # CDATA section: <![CDATA[ ... ]]>
    if scanner.match("![CDATA["):
        while not scanner.done:
            if scanner.match("]]>"):
                return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}
            scanner.skip(1)
        scanner.pos = saved_pos
        return None

    # Declaration: <!UPPER...>
    if scanner.match("!"):
        if scanner.peek().isupper():
            scanner.consume_while(lambda c: c != ">")
            if scanner.match(">"):
                return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}
        scanner.pos = saved_pos
        return None

    # Closing tag: </tagname>
    if ch == "/":
        scanner.skip(1)
        tag = scanner.consume_while(lambda c: re.match(r"[a-zA-Z0-9\-]", c) is not None)
        if not tag:
            scanner.pos = saved_pos
            return None
        scanner.skip_spaces()
        if not scanner.match(">"):
            scanner.pos = saved_pos
            return None
        return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}

    # Open tag: <tagname attr...> or <tagname attr.../>
    if re.match(r"[a-zA-Z]", ch):
        tag_name = scanner.consume_while(lambda c: re.match(r"[a-zA-Z0-9\-]", c) is not None)
        if not tag_name:
            scanner.pos = saved_pos
            return None

        # Track newlines consumed in this tag (max 1 allowed total)
        newlines_in_tag = 0

        while True:
            space_len = scanner.skip_spaces()
            # Allow at most one newline anywhere in the attribute area
            if newlines_in_tag == 0 and scanner.peek() == "\n":
                newlines_in_tag += 1
                scanner.skip(1)
                space_len += 1 + scanner.skip_spaces()
            next_ch = scanner.peek()
            if next_ch in (">", "/", ""):
                break
            # Second newline → invalid tag
            if next_ch == "\n":
                scanner.pos = saved_pos
                return None
            # Each attribute must be preceded by whitespace
            if space_len == 0:
                scanner.pos = saved_pos
                return None

            # Attribute name: must start with ASCII alpha, _, or :
            if not re.match(r"[a-zA-Z_:]", next_ch):
                scanner.pos = saved_pos
                return None
            scanner.consume_while(lambda c: re.match(r"[a-zA-Z0-9_:\.\-]", c) is not None)

            # Optional `= value`.
            pos_before_eq_spaces = scanner.pos
            scanner.skip_spaces()
            if scanner.peek() == "=":
                scanner.skip(1)
                scanner.skip_spaces()
                q = scanner.peek()
                if q in ('"', "'"):
                    scanner.skip(1)
                    closed = False
                    while not scanner.done:
                        vc = scanner.source[scanner.pos]
                        if vc == q:
                            scanner.skip(1)
                            closed = True
                            break
                        if vc == "\n":
                            if newlines_in_tag >= 1:
                                scanner.pos = saved_pos
                                return None
                            newlines_in_tag += 1
                        scanner.skip(1)
                    if not closed:
                        scanner.pos = saved_pos
                        return None
                else:
                    # Unquoted value: no whitespace, ", ', =, <, >, `
                    unquoted = scanner.consume_while(lambda c: not re.match(r"""[\s"'=<>`]""", c))
                    if not unquoted:
                        scanner.pos = saved_pos
                        return None
            else:
                # No =: restore position to before the trailing spaces
                scanner.pos = pos_before_eq_spaces

        self_close = scanner.match("/>")
        if not self_close and not scanner.match(">"):
            scanner.pos = saved_pos
            return None
        return {"type": "raw_inline", "format": "html", "value": scanner.source[saved_pos:scanner.pos]}

    scanner.pos = saved_pos
    return None


# ─── Autolink Parsing ─────────────────────────────────────────────────────────

def _try_autolink(scanner: Scanner) -> AutolinkNode | None:
    """Attempt to parse an autolink: <URI> or <email>.

    URL autolink: <scheme:path> where scheme is 2–32 chars from
    [a-zA-Z0-9+.-], and path has no spaces or < or >.

    Email autolink: <user@domain> where the address matches a simple
    RFC-5322-ish pattern (CommonMark spec §6.7).
    """
    if scanner.peek() != "<":
        return None
    saved_pos = scanner.pos
    scanner.skip(1)

    start = scanner.pos

    # Try email autolink: local@domain
    local_part = scanner.consume_while(lambda c: re.match(r"[^\s<>@]", c) is not None)
    if local_part and scanner.peek() == "@":
        scanner.skip(1)
        domain_part = scanner.consume_while(lambda c: re.match(r"[^\s<>]", c) is not None)
        if domain_part and scanner.match(">"):
            # Validate email local part and domain
            if (re.match(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$", local_part)
                    and re.match(
                        r"^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$",
                        domain_part,
                    )):
                return {
                    "type": "autolink",
                    "destination": local_part + "@" + domain_part,
                    "is_email": True,
                }

    # Retry as URL autolink
    scanner.pos = start
    scheme = scanner.consume_while(lambda c: re.match(r"[a-zA-Z0-9+\-.]", c) is not None)
    if 2 <= len(scheme) <= 32 and scanner.match(":"):
        path = scanner.consume_while(lambda c: c not in (" ", "<", ">", "\n"))
        if scanner.match(">"):
            return {
                "type": "autolink",
                "destination": scheme + ":" + path,
                "is_email": False,
            }

    scanner.pos = saved_pos
    return None


# ─── Link / Image Destination Parsing ────────────────────────────────────────

def _try_link_after_close(
    scanner: Scanner,
    link_refs: dict,
    inner_text: str,
) -> dict | None:
    """After the ] of a potential link/image bracket, try to parse:

      Inline:     (destination "title") or (destination)
      Full ref:   [label]
      Collapsed:  []               — uses inner_text as the label
      Shortcut:   (nothing)          — uses inner_text as the label

    Returns None if no valid link form is recognised.

    @param scanner    Positioned just after the ].
    @param link_refs   The link reference map.
    @param inner_text  The plain-text content of the brackets (for reference forms).
    """
    saved_pos = scanner.pos

    # ── Inline link: ( destination "title" ) ────────────────────────────────
    if scanner.peek() == "(":
        inline_link_result = _parse_inline_link(scanner, saved_pos)
        if inline_link_result is not None:
            return inline_link_result
        # Inline link failed — reset and fall through to reference checks.
        scanner.pos = saved_pos

    # ── Full reference: [label] or Collapsed reference: [] ──────────────────
    if scanner.peek() == "[":
        scanner.skip(1)
        label_buf = ""
        valid_label = True
        while not scanner.done:
            c = scanner.peek()
            if c == "]":
                scanner.skip(1)
                break
            if c in ("\n", "["):
                valid_label = False
                break
            if c == "\\":
                scanner.skip(1)
                if not scanner.done:
                    label_buf += "\\" + scanner.advance()
            else:
                label_buf += scanner.advance()
        if valid_label:
            if label_buf.strip():
                label = normalize_link_label(label_buf)
                ref = link_refs.get(label)
                if ref:
                    return {"destination": ref.destination, "title": ref.title}
            else:
                # Collapsed reference: [] — use inner text as the label
                label = normalize_link_label(inner_text)
                ref = link_refs.get(label)
                if ref:
                    return {"destination": ref.destination, "title": ref.title}
        scanner.pos = saved_pos
        return None

    # ── Shortcut reference: no ( or [ follows — use inner text as label ──
    label = normalize_link_label(inner_text)
    ref = link_refs.get(label)
    if ref:
        return {"destination": ref.destination, "title": ref.title}

    return None


def _parse_inline_link(scanner: Scanner, saved_pos: int) -> dict | None:
    """Parse an inline link destination: (url "title") or (url).

    Called by _try_link_after_close when the next character is (.
    Returns {destination, title} or None on failure.
    """
    scanner.skip(1)  # consume (
    _skip_optional_spaces_and_newline(scanner)

    destination = ""

    if scanner.peek() == "<":
        # Angle-bracket destination: <url> — no line endings or bare <
        scanner.skip(1)
        dest_buf = ""
        while not scanner.done:
            c = scanner.peek()
            if c in ("\n", "\r"):
                return None
            if c == "\\":
                scanner.skip(1)
                next_ch = scanner.advance()
                dest_buf += next_ch if is_ascii_punctuation(next_ch) else "\\" + next_ch
            elif c == ">":
                scanner.skip(1)
                break
            elif c == "<":
                return None
            else:
                dest_buf += scanner.advance()
        destination = normalize_url(decode_entities(dest_buf))
    else:
        # Bare destination — no spaces, balanced parens, backslash-escapes
        depth = 0
        dest_start = scanner.pos
        while not scanner.done:
            c = scanner.peek()
            if c == "(":
                depth += 1
                scanner.skip(1)
            elif c == ")":
                if depth == 0:
                    break
                depth -= 1
                scanner.skip(1)
            elif c == "\\":
                scanner.skip(2)
            elif is_ascii_whitespace(c):
                break
            else:
                scanner.skip(1)
        dest_raw = scanner.source[dest_start:scanner.pos]
        destination = normalize_url(decode_entities(_apply_backslash_escapes_inline(dest_raw)))

    _skip_optional_spaces_and_newline(scanner)

    # Optional title
    title: str | None = None
    q = scanner.peek()
    if q in ('"', "'", "("):
        close_q = ")" if q == "(" else q
        scanner.skip(1)
        title_buf = ""
        while not scanner.done:
            c = scanner.peek()
            if c == "\\":
                scanner.skip(1)
                next_ch = scanner.advance()
                title_buf += next_ch if is_ascii_punctuation(next_ch) else "\\" + next_ch
            elif c == close_q:
                scanner.skip(1)
                title = decode_entities(title_buf)
                break
            elif c == "\n" and q == "(":
                break  # parens title cannot span lines
            else:
                title_buf += scanner.advance()

    scanner.skip_spaces()
    if not scanner.match(")"):
        return None
    return {"destination": destination, "title": title}


def _apply_backslash_escapes_inline(s: str) -> str:
    """Apply backslash escapes in inline context (same logic as block parser)."""
    result = []
    i = 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            next_ch = s[i + 1]
            if is_ascii_punctuation(next_ch):
                result.append(next_ch)
                i += 2
            else:
                result.append("\\")
                i += 1
        else:
            result.append(s[i])
            i += 1
    return "".join(result)


# ─── Utilities ────────────────────────────────────────────────────────────────

def _skip_optional_spaces_and_newline(scanner: Scanner) -> None:
    """Skip ASCII spaces/tabs and at most one line ending."""
    scanner.skip_spaces()
    if scanner.peek() == "\n":
        scanner.skip(1)
        scanner.skip_spaces()
    elif scanner.peek() == "\r" and scanner.peek(1) == "\n":
        scanner.skip(2)
        scanner.skip_spaces()


def _find_active_bracket_opener(
    bracket_stack: list[int],
    tokens: list[InlineToken],
) -> int:
    """Find the index (into bracket_stack) of the most recent active bracket opener.

    Returns -1 if none exists.
    """
    for i in range(len(bracket_stack) - 1, -1, -1):
        idx = bracket_stack[i]
        if idx < len(tokens):
            t = tokens[idx]
            if t.kind == "bracket" and t.active:
                return i
    return -1


def _extract_plain_text(nodes: list[InlineNode]) -> str:
    """Recursively extract plain text from inline nodes.

    Used for image `alt` attributes.

    CommonMark spec §6.4: "The content of the first sequence of brackets
    is used as the alt text, but all elements except the characters
    themselves are stripped."
    """
    result = ""
    for node in nodes:
        node_type = node["type"]
        if node_type == "text":
            result += node["value"]
        elif node_type == "code_span":
            result += node["value"]
        elif node_type == "hard_break":
            result += "\n"
        elif node_type == "soft_break":
            result += " "
        elif node_type in ("emphasis", "strong", "link"):
            result += _extract_plain_text(node["children"])
        elif node_type == "image":
            result += node["alt"]
        elif node_type == "autolink":
            result += node["destination"]
    return result


# ─── Document-Level Inline Resolution ─────────────────────────────────────────

def resolve_inline_content(
    document: DocumentNode,
    raw_inline_content: dict[int, str],
    link_refs: dict,
) -> None:
    """Walk the block AST and fill in inline content.

    The block parser attaches a raw inline content string (stored by node id)
    to heading and paragraph nodes instead of populating their children arrays.
    This function uses those ids to retrieve the raw strings, parses them,
    and writes the resulting InlineNode list back into the node's children.

    After this function returns, the AST is complete.

    @param document          Root of the block AST.
    @param raw_inline_content  id(node) → raw inline string map from convert_to_ast.
    @param link_refs          The link reference map from Phase 1.
    """

    def walk(block: BlockNode) -> None:
        """Recursively walk block nodes, parsing inline content where present."""
        block_type = block["type"]

        if block_type in ("heading", "paragraph"):
            node_id = id(block)
            raw = raw_inline_content.get(node_id)
            if raw is not None:
                block["children"] = parse_inline(raw, link_refs)

        # Recurse into container blocks
        if "children" in block and isinstance(block["children"], list):
            for child in block["children"]:
                walk(child)

    walk(document)
