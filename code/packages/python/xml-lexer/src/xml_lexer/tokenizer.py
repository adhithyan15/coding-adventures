"""XML Lexer — tokenizes XML using pattern groups and callback hooks.

This module is the first lexer wrapper that uses the **pattern group**
and **on-token callback** features of the grammar-driven lexer. It loads
the ``xml.tokens`` grammar and registers a callback that switches between
pattern groups based on which token was just matched.

Context-Sensitive Lexing
------------------------

XML requires context-sensitive lexing because different parts of an XML
document follow different lexical rules:

- **Between tags** (default group): Text content, entity references
  like ``&amp;``, and opening delimiters for tags/comments/CDATA/PIs.

- **Inside a tag** (tag group): Tag names, attribute names (same regex
  as tag names), equals signs, quoted attribute values, and closing
  delimiters like ``>`` and ``/>``.

- **Inside a comment** (comment group): Everything is comment text until
  ``-->`` is seen. Whitespace is significant (not skipped).

- **Inside CDATA** (cdata group): Everything is raw text until ``]]>``
  is seen. No entity processing, no tag recognition.

- **Inside a processing instruction** (pi group): Target name and text
  content until ``?>`` is seen.

The Callback
------------

The ``xml_on_token`` function is the callback that drives group switching.
It follows a simple state machine:

.. code-block:: text

    default ──OPEN_TAG_START──> tag ──TAG_CLOSE──> default
            ──CLOSE_TAG_START─> tag ──SELF_CLOSE─> default
            ──COMMENT_START───> comment ──COMMENT_END──> default
            ──CDATA_START─────> cdata ──CDATA_END──> default
            ──PI_START────────> pi ──PI_END──> default

For comment, CDATA, and PI groups, the callback also disables skip
patterns (so whitespace is preserved as content) and re-enables them
when leaving the group.

Locating the Grammar File
--------------------------

The ``xml.tokens`` file lives in ``code/grammars/`` at the repository
root. We navigate there from this file's location::

    tokenizer.py
    └── xml_lexer/        (parent)
        └── src/          (parent)
            └── xml-lexer/  (parent)
                └── python/   (parent)
                    └── packages/ (parent)
                        └── code/    (parent)
                            └── grammars/
                                └── xml.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, LexerContext, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = (
    Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
)
XML_TOKENS_PATH = GRAMMAR_DIR / "xml.tokens"


# ---------------------------------------------------------------------------
# XML On-Token Callback
# ---------------------------------------------------------------------------
#
# This callback drives the group transitions. It is a pure function of
# the token type — no external state is needed. The LexerContext provides
# all the control we need (push/pop groups, toggle skip).
#
# The pattern is simple:
# - Opening delimiters push a group
# - Closing delimiters pop the group
# - Comment/CDATA/PI groups disable skip (whitespace is content)
# ---------------------------------------------------------------------------


def xml_on_token(token: Token, ctx: LexerContext) -> None:
    """Callback that switches pattern groups for XML tokenization.

    This function fires after each token match. It examines the token
    type and pushes/pops pattern groups accordingly:

    - ``OPEN_TAG_START`` (``<``) or ``CLOSE_TAG_START`` (``</``):
      Push the "tag" group so the lexer recognizes tag names, attributes,
      and tag closers.

    - ``TAG_CLOSE`` (``>``) or ``SELF_CLOSE`` (``/>``):
      Pop the "tag" group to return to default (text content).

    - ``COMMENT_START`` (``<!--``):
      Push "comment" group and disable skip (whitespace is significant).

    - ``COMMENT_END`` (``-->``):
      Pop "comment" group and re-enable skip.

    - ``CDATA_START`` (``<![CDATA[``):
      Push "cdata" group and disable skip.

    - ``CDATA_END`` (``]]>``):
      Pop "cdata" group and re-enable skip.

    - ``PI_START`` (``<?``):
      Push "pi" group and disable skip.

    - ``PI_END`` (``?>``):
      Pop "pi" group and re-enable skip.

    Args:
        token: The token that was just matched.
        ctx: The ``LexerContext`` for controlling the lexer.
    """
    token_type = token.type if isinstance(token.type, str) else token.type.name

    match token_type:
        # --- Tag boundaries ---
        case "OPEN_TAG_START" | "CLOSE_TAG_START":
            ctx.push_group("tag")
        case "TAG_CLOSE" | "SELF_CLOSE":
            ctx.pop_group()

        # --- Comment boundaries ---
        case "COMMENT_START":
            ctx.push_group("comment")
            ctx.set_skip_enabled(False)
        case "COMMENT_END":
            ctx.pop_group()
            ctx.set_skip_enabled(True)

        # --- CDATA boundaries ---
        case "CDATA_START":
            ctx.push_group("cdata")
            ctx.set_skip_enabled(False)
        case "CDATA_END":
            ctx.pop_group()
            ctx.set_skip_enabled(True)

        # --- Processing instruction boundaries ---
        case "PI_START":
            ctx.push_group("pi")
            ctx.set_skip_enabled(False)
        case "PI_END":
            ctx.pop_group()
            ctx.set_skip_enabled(True)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_xml_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for XML text.

    This function reads the ``xml.tokens`` file, parses it into a
    ``TokenGrammar``, creates a ``GrammarLexer``, and registers the
    XML on-token callback for pattern group switching.

    Args:
        source: The XML text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with XML token definitions
        and the group-switching callback. Call ``.tokenize()`` to get
        the token list.

    Raises:
        FileNotFoundError: If the ``xml.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_xml_lexer('<div>hello</div>')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(XML_TOKENS_PATH.read_text())
    lexer = GrammarLexer(source, grammar)
    lexer.set_on_token(xml_on_token)
    return lexer


def tokenize_xml(source: str) -> list[Token]:
    """Tokenize XML text and return a list of tokens.

    This is the main entry point for the XML lexer. Pass in a string
    of XML text, and get back a flat list of ``Token`` objects. The
    list always ends with an ``EOF`` token.

    Token types you will see:

    **Default group** (content between tags):

    - **TEXT** — text content (e.g., ``Hello world``)
    - **ENTITY_REF** — entity reference (e.g., ``&amp;``)
    - **CHAR_REF** — character reference (e.g., ``&#65;``, ``&#x41;``)
    - **OPEN_TAG_START** — ``<``
    - **CLOSE_TAG_START** — ``</``
    - **COMMENT_START** — ``<!--``
    - **CDATA_START** — ``<![CDATA[``
    - **PI_START** — ``<?``

    **Tag group** (inside tags):

    - **TAG_NAME** — tag or attribute name (e.g., ``div``, ``class``)
    - **ATTR_EQUALS** — ``=``
    - **ATTR_VALUE** — quoted attribute value (e.g., ``"main"``)
    - **TAG_CLOSE** — ``>``
    - **SELF_CLOSE** — ``/>``

    **Comment group**:

    - **COMMENT_TEXT** — comment content
    - **COMMENT_END** — ``-->``

    **CDATA group**:

    - **CDATA_TEXT** — raw text content
    - **CDATA_END** — ``]]>``

    **Processing instruction group**:

    - **PI_TARGET** — PI target name (e.g., ``xml``)
    - **PI_TEXT** — PI content
    - **PI_END** — ``?>``

    **Always present**:

    - **EOF** — end of input

    Args:
        source: The XML text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``xml.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the active group.

    Example::

        tokens = tokenize_xml('<p>Hello &amp; world</p>')
        # [Token(OPEN_TAG_START, '<'), Token(TAG_NAME, 'p'),
        #  Token(TAG_CLOSE, '>'), Token(TEXT, 'Hello '),
        #  Token(ENTITY_REF, '&amp;'), Token(TEXT, ' world'),
        #  Token(CLOSE_TAG_START, '</'), Token(TAG_NAME, 'p'),
        #  Token(TAG_CLOSE, '>'), Token(EOF, '')]
    """
    return create_xml_lexer(source).tokenize()
