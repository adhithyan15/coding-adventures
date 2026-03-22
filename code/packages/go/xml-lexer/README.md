# xml-lexer

A grammar-driven lexer for XML text using pattern groups and an on-token callback for context-sensitive tokenization.

## How It Fits in the Stack

This package sits in the tokenization layer of the grammar-driven compiler infrastructure:

```
xml.tokens (grammar file with pattern groups)
        |
   grammar-tools (parses the grammar)
        |
      lexer (generic grammar-driven lexer engine)
        |
   xml-lexer (this package -- callback-driven wrapper)
        |
   xml-parser (consumes the token stream)
```

Unlike the JSON lexer (which uses a flat pattern list), the XML lexer uses **pattern groups** and an **on-token callback** to handle XML's context-sensitive lexical structure. The same character (e.g., `=`) has different meaning depending on whether it appears inside a tag or in text content.

## Pattern Groups

The `xml.tokens` grammar defines 5 pattern groups:

| Group     | Active When        | Recognizes                                    |
|-----------|--------------------|-----------------------------------------------|
| default   | Between tags       | TEXT, entity refs, tag/comment/CDATA/PI openers|
| tag       | Inside `<...>`     | TAG_NAME, ATTR_EQUALS, ATTR_VALUE, closers    |
| comment   | Inside `<!--...-->` | COMMENT_TEXT, COMMENT_END                     |
| cdata     | Inside `<![CDATA[...]]>` | CDATA_TEXT, CDATA_END                   |
| pi        | Inside `<?...?>`   | PI_TARGET, PI_TEXT, PI_END                    |

## Token Types

**Default group** (content between tags):

| Token          | Description                  | Example       |
|----------------|------------------------------|---------------|
| TEXT           | Text content                 | `Hello world` |
| ENTITY_REF    | Entity reference             | `&amp;`       |
| CHAR_REF      | Character reference          | `&#65;`       |
| OPEN_TAG_START | Opening tag delimiter       | `<`           |
| CLOSE_TAG_START| Closing tag delimiter       | `</`          |
| COMMENT_START  | Comment start               | `<!--`        |
| CDATA_START    | CDATA start                 | `<![CDATA[`   |
| PI_START       | Processing instruction start| `<?`          |

**Tag group** (inside tags):

| Token       | Description              | Example    |
|-------------|--------------------------|------------|
| TAG_NAME    | Tag or attribute name    | `div`      |
| ATTR_EQUALS | Attribute equals sign    | `=`        |
| ATTR_VALUE  | Quoted attribute value   | `"main"`   |
| TAG_CLOSE   | Tag close                | `>`        |
| SELF_CLOSE  | Self-closing delimiter   | `/>`       |

**Comment, CDATA, and PI groups** each have their text and end tokens.

## Usage

```go
package main

import (
    "fmt"
    xmllexer "github.com/adhithyan15/coding-adventures/code/packages/go/xml-lexer"
)

func main() {
    // One-shot tokenization
    tokens, err := xmllexer.TokenizeXml(`<div class="main">Hello &amp; world</div>`)
    if err != nil {
        panic(err)
    }
    for _, tok := range tokens {
        fmt.Printf("%s(%q)\n", tok.TypeName, tok.Value)
    }

    // Or create a reusable lexer
    lex, err := xmllexer.CreateXmlLexer(`<p>text</p>`)
    if err != nil {
        panic(err)
    }
    tokens = lex.Tokenize()
}
```

## Running Tests

```bash
go test -v ./...
```
