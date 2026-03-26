# @coding-adventures/xml-lexer

Tokenizes XML text using pattern groups and callback hooks for context-sensitive lexing.

## What It Does

This package is the first **callback-driven** lexer wrapper in TypeScript. Unlike the JSON lexer (which uses a flat pattern list), the XML lexer uses **pattern groups** and an **on-token callback** to handle XML's context-sensitive lexical structure.

XML is context-sensitive at the lexical level. The same character has different meaning depending on position:

- `=` is an attribute delimiter inside `<tag attr="val">`
- `=` is plain text content outside tags: `1 + 1 = 2`

Pattern groups solve this by defining separate sets of patterns for each context, and a callback function switches between them at runtime.

## How It Fits in the Stack

```
xml.tokens (grammar file — 5 pattern groups)
    |
    v
grammar-tools (parses the grammar file)
    |
    v
lexer (GrammarLexer with on-token callback support)
    |
    v
xml-lexer (this package — callback-driven wrapper)
    |
    v
xml-parser (consumes the token stream)
```

## Pattern Groups

The `xml.tokens` grammar defines 5 pattern groups:

| Group     | Active When           | Recognizes                                    |
|-----------|-----------------------|-----------------------------------------------|
| default   | Between tags          | TEXT, ENTITY_REF, CHAR_REF, tag/comment openers |
| tag       | Inside `< >` or `</ >` | TAG_NAME, ATTR_EQUALS, ATTR_VALUE, closers    |
| comment   | Inside `<!-- -->`     | COMMENT_TEXT, COMMENT_END                     |
| cdata     | Inside `<![CDATA[ ]]>` | CDATA_TEXT, CDATA_END                        |
| pi        | Inside `<? ?>`        | PI_TARGET, PI_TEXT, PI_END                    |

## Usage

```typescript
import { tokenizeXML } from "@coding-adventures/xml-lexer";

const tokens = tokenizeXML('<div class="main">Hello &amp; world</div>');

for (const token of tokens) {
  console.log(`${token.type}: ${token.value} (line ${token.line}, col ${token.column})`);
}

// Output:
// OPEN_TAG_START: < (line 1, col 1)
// TAG_NAME: div (line 1, col 2)
// TAG_NAME: class (line 1, col 6)
// ATTR_EQUALS: = (line 1, col 11)
// ATTR_VALUE: "main" (line 1, col 12)
// TAG_CLOSE: > (line 1, col 18)
// TEXT: Hello  (line 1, col 19)
// ENTITY_REF: &amp; (line 1, col 25)
// TEXT:  world (line 1, col 30)
// CLOSE_TAG_START: </ (line 1, col 36)
// TAG_NAME: div (line 1, col 38)
// TAG_CLOSE: > (line 1, col 41)
// EOF:  (line 1, col 42)
```

## Token Types

**Default group** (content between tags):

| Token         | Example       | Description                        |
|---------------|---------------|------------------------------------|
| TEXT          | Hello world   | Text content                       |
| ENTITY_REF   | &amp;         | Named entity reference             |
| CHAR_REF     | &#65;, &#x41; | Numeric character reference        |
| OPEN_TAG_START | <            | Start of an open tag               |
| CLOSE_TAG_START | </          | Start of a close tag               |
| COMMENT_START | <!--          | Start of a comment                 |
| CDATA_START  | <![CDATA[     | Start of a CDATA section           |
| PI_START     | <?            | Start of a processing instruction  |

**Tag group** (inside tags):

| Token       | Example | Description             |
|-------------|---------|-------------------------|
| TAG_NAME    | div     | Tag or attribute name   |
| ATTR_EQUALS | =       | Attribute delimiter     |
| ATTR_VALUE  | "main"  | Quoted attribute value  |
| TAG_CLOSE   | >       | End of tag              |
| SELF_CLOSE  | />      | Self-closing tag end    |

**Comment, CDATA, PI groups**:

| Token        | Example  | Description                 |
|--------------|----------|-----------------------------|
| COMMENT_TEXT | text     | Comment content             |
| COMMENT_END  | -->      | End of comment              |
| CDATA_TEXT   | raw text | CDATA content               |
| CDATA_END    | ]]>      | End of CDATA section        |
| PI_TARGET    | xml      | Processing instruction name |
| PI_TEXT      | ver...   | PI content                  |
| PI_END       | ?>       | End of PI                   |

## Running Tests

```bash
npm install
npm test
```
