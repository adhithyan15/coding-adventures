"""CSS Parser — parses CSS text into ASTs using the grammar-driven approach.

This package is a **thin wrapper** around the generic ``GrammarParser``. It
tokenizes CSS using the ``css-lexer`` package, then parses the token stream
using the EBNF rules defined in ``css.grammar``.

The result is a generic ``ASTNode`` tree — the same type used for JSON,
Starlark, and every other language the grammar-driven infrastructure
supports. CSS is the most complex grammar in the collection, stress-testing
the parser with:

- Complex selector syntax (combinators, pseudo-classes, pseudo-elements)
- Nested structures (media queries containing rule sets)
- At-rule diversity (@media, @import, @keyframes, @font-face)
- CSS Nesting with ``&`` ampersand references
- ``!important`` priority annotation (literal matching)
- ``calc()`` expressions with nested function calls

Usage::

    from css_parser import parse_css

    ast = parse_css('h1 { color: red; }')
    print(ast.rule_name)  # "stylesheet"
"""

from css_parser.parser import create_css_parser, parse_css

__all__ = [
    "create_css_parser",
    "parse_css",
]
