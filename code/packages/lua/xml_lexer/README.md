# coding-adventures-xml-lexer (Lua)

An XML lexer that tokenizes XML source text into a flat stream of typed tokens. It wraps the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `xml.tokens` grammar file, and registers group-switching callbacks to handle XML's context-sensitive lexical structure.

## What it does

Given `<root attr="v">text</root>`, the lexer produces:

| # | Type            | Value  |
|---|-----------------|--------|
| 1 | OPEN_TAG_START  | `<`    |
| 2 | TAG_NAME        | `root` |
| 3 | TAG_NAME        | `attr` |
| 4 | ATTR_EQUALS     | `=`    |
| 5 | ATTR_VALUE      | `"v"`  |
| 6 | TAG_CLOSE       | `>`    |
| 7 | TEXT            | `text` |
| 8 | CLOSE_TAG_START | `</`   |
| 9 | TAG_NAME        | `root` |
|10 | TAG_CLOSE       | `>`    |
|11 | EOF             |        |

## Context-sensitive groups

XML lexing is context-sensitive. The `xml.tokens` grammar defines pattern groups and this lexer registers callbacks to switch between them:

| Trigger token     | Action           |
|-------------------|------------------|
| OPEN_TAG_START    | push `tag` group |
| CLOSE_TAG_START   | push `tag` group |
| TAG_CLOSE         | pop group        |
| SELF_CLOSE        | pop group        |
| COMMENT_START     | push `comment` group |
| COMMENT_END       | pop group        |
| CDATA_START       | push `cdata` group |
| CDATA_END         | pop group        |
| PI_START          | push `pi` group  |
| PI_END            | pop group        |

## Token types

| Token type      | Example / context                          |
|-----------------|--------------------------------------------|
| OPEN_TAG_START  | `<` at start of opening tag                |
| CLOSE_TAG_START | `</`                                       |
| TAG_NAME        | `root`, `div`, `href`                      |
| ATTR_EQUALS     | `=`                                        |
| ATTR_VALUE      | `"value"`, `'value'` (aliased)             |
| TAG_CLOSE       | `>`                                        |
| SELF_CLOSE      | `/>`                                       |
| SLASH           | `/` (bare, inside tag)                     |
| TEXT            | plain text content between tags            |
| ENTITY_REF      | `&amp;`, `&lt;`                            |
| CHAR_REF        | `&#65;`, `&#x41;`                          |
| COMMENT_START   | `<!--`                                     |
| COMMENT_TEXT    | text inside `<!-- ... -->`                 |
| COMMENT_END     | `-->`                                      |
| CDATA_START     | `<![CDATA[`                                |
| CDATA_TEXT      | raw text inside CDATA                      |
| CDATA_END       | `]]>`                                      |
| PI_START        | `<?`                                       |
| PI_TARGET       | `xml` in `<?xml ...?>`                     |
| PI_TEXT         | content inside `<? ... ?>`                 |
| PI_END          | `?>`                                       |
| EOF             | (end of input)                             |

## Usage

```lua
local xml_lexer = require("coding_adventures.xml_lexer")

local tokens = xml_lexer.tokenize('<root attr="val">text</root>')
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
xml.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives (with group-switch callbacks)
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
xml_lexer  ← you are here
    ↓  feeds
xml_parser  (future)
```

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
