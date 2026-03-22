# XML Lexer

Tokenizes XML documents using the grammar-driven lexer with pattern groups and callback hooks.

## What It Does

This is the first lexer wrapper that uses **pattern groups** — named sets of token patterns that are activated/deactivated at runtime via a group stack. A callback function (`xml_on_token`) switches between groups based on which token was just matched.

## How It Fits

```
xml.tokens (grammar) → grammar-tools (parser) → lexer (engine) → xml-lexer (this package)
```

The `xml.tokens` grammar defines 5 pattern groups (default, tag, comment, cdata, pi). The callback pushes/pops groups when opening/closing delimiters are matched.

## Usage

```python
from xml_lexer import tokenize_xml

tokens = tokenize_xml('<div class="main">Hello &amp; world</div>')
for token in tokens:
    print(f"{token.type:20s} {token.value!r}")
```

Output:
```
OPEN_TAG_START       '<'
TAG_NAME             'div'
TAG_NAME             'class'
ATTR_EQUALS          '='
ATTR_VALUE           '"main"'
TAG_CLOSE            '>'
TEXT                 'Hello '
ENTITY_REF           '&amp;'
TEXT                 'world'
CLOSE_TAG_START      '</'
TAG_NAME             'div'
TAG_CLOSE            '>'
EOF                  ''
```

## Token Types

| Group | Tokens |
|-------|--------|
| default | TEXT, ENTITY_REF, CHAR_REF, OPEN_TAG_START, CLOSE_TAG_START, COMMENT_START, CDATA_START, PI_START |
| tag | TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE |
| comment | COMMENT_TEXT, COMMENT_END |
| cdata | CDATA_TEXT, CDATA_END |
| pi | PI_TARGET, PI_TEXT, PI_END |
