# XML Lexer

A Ruby gem that tokenizes XML text using the grammar-driven lexer engine with pattern groups and an on-token callback.

## Overview

This gem is the first **callback-driven** lexer wrapper. Unlike the JSON lexer (which uses a flat pattern list), the XML lexer uses **pattern groups** and an **on-token callback** to handle XML's context-sensitive lexical structure.

XML is context-sensitive at the lexical level. The same character has different meaning depending on position:

- `=` is an attribute delimiter inside `<tag attr="val">`
- `=` is plain text content outside tags: `1 + 1 = 2`

Pattern groups solve this by defining separate sets of patterns for each context, and the callback switches between them at runtime.

## How It Fits in the Stack

```
xml.tokens (grammar file with pattern groups)
       |
       v
grammar_tools (parses .tokens into TokenGrammar)
       |
       v
lexer (GrammarLexer with on_token callback support)
       |
       v
xml_lexer (this gem -- callback-driven wrapper providing XML API)
```

## Usage

```ruby
require "coding_adventures_xml_lexer"

tokens = CodingAdventures::XmlLexer.tokenize('<div class="main">Hello &amp; world</div>')
tokens.each { |t| puts t }
# Token(OPEN_TAG_START, "<", 1:1)
# Token(TAG_NAME, "div", 1:2)
# Token(TAG_NAME, "class", 1:6)
# Token(ATTR_EQUALS, "=", 1:11)
# Token(ATTR_VALUE, "\"main\"", 1:12)
# Token(TAG_CLOSE, ">", 1:18)
# Token(TEXT, "Hello ", 1:19)
# Token(ENTITY_REF, "&amp;", 1:25)
# Token(TEXT, " world", 1:30)
# Token(CLOSE_TAG_START, "</", 1:36)
# Token(TAG_NAME, "div", 1:38)
# Token(TAG_CLOSE, ">", 1:41)
# Token(EOF, "", 1:42)
```

## Pattern Groups

The `xml.tokens` grammar defines 5 pattern groups:

| Group | Active When | Token Types |
|-------|-------------|-------------|
| **default** | Between tags | TEXT, ENTITY_REF, CHAR_REF, tag/comment/CDATA/PI openers |
| **tag** | Inside `<...>` | TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE |
| **comment** | Inside `<!-- -->` | COMMENT_TEXT, COMMENT_END |
| **cdata** | Inside `<![CDATA[ ]]>` | CDATA_TEXT, CDATA_END |
| **pi** | Inside `<? ?>` | PI_TARGET, PI_TEXT, PI_END |

## The Callback

The `XML_ON_TOKEN` callback fires after each token match and controls group transitions:

```
default --OPEN_TAG_START--> tag --TAG_CLOSE--> default
        --CLOSE_TAG_START-> tag --SELF_CLOSE-> default
        --COMMENT_START---> comment --COMMENT_END--> default
        --CDATA_START-----> cdata --CDATA_END--> default
        --PI_START--------> pi --PI_END--> default
```

For comment, CDATA, and PI groups, the callback disables skip patterns so whitespace is preserved as content.

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine with pattern group and callback support

## Development

```bash
bundle install
bundle exec rake test
```
