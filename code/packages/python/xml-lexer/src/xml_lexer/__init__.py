"""XML Lexer — tokenizes XML using pattern groups and callback hooks.

This package is the first **callback-driven** lexer wrapper. Unlike the
JSON lexer (which uses a flat pattern list), the XML lexer uses **pattern
groups** and an **on-token callback** to handle XML's context-sensitive
lexical structure.

The Problem
-----------

XML is context-sensitive at the lexical level. The same character has
different meaning depending on position:

- ``=`` is an attribute delimiter inside ``<tag attr="val">``
- ``=`` is plain text content outside tags: ``1 + 1 = 2``

A flat pattern list cannot distinguish these contexts. Pattern groups
solve this by defining separate sets of patterns for each context, and
a callback function switches between them at runtime.

How It Works
------------

The ``xml.tokens`` grammar defines 5 pattern groups:

- **default** (implicit): Text content, entity refs, tag openers
- **tag**: Tag names, attributes, equals, quoted values, closers
- **comment**: Comment text and ``-->`` delimiter
- **cdata**: Raw text and ``]]>`` delimiter
- **pi**: Processing instruction target, text, and ``?>`` delimiter

The callback (``xml_on_token``) fires after each token match and:

- Pushes ``"tag"`` when ``<`` or ``</`` is matched
- Pops when ``>`` or ``/>`` is matched
- Pushes ``"comment"``/``"cdata"``/``"pi"`` for their start delimiters
- Pops and re-enables skip for their end delimiters
- Disables skip patterns inside comments/CDATA/PIs (whitespace matters)

Usage::

    from xml_lexer import tokenize_xml

    tokens = tokenize_xml('<div class="main">Hello &amp; world</div>')
    for token in tokens:
        print(token)
"""

from xml_lexer.tokenizer import (
    create_xml_lexer,
    tokenize_xml,
    xml_on_token,
)

__all__ = [
    "create_xml_lexer",
    "tokenize_xml",
    "xml_on_token",
]
