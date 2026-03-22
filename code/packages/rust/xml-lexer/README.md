# XML Lexer

A grammar-driven lexer (tokenizer) for [XML 1.0](https://www.w3.org/XML/) using pattern groups and an on-token callback for context-sensitive lexing.

## What it does

This crate tokenizes XML source text into a stream of typed tokens. Unlike the JSON lexer (which uses a flat pattern list), the XML lexer uses **pattern groups** and a **callback** to handle XML's context-sensitive lexical structure.

The same character has different meaning depending on position:
- `=` inside a tag: attribute delimiter
- `=` outside a tag: plain text content

Pattern groups solve this by defining separate sets of patterns for each context, and the callback switches between them at runtime.

## How it fits in the stack

```text
xml.tokens           (grammar file — 5 pattern groups)
       |
       v
grammar-tools        (parses .tokens file -> TokenGrammar with groups)
       |
       v
lexer::GrammarLexer  (tokenizes source using active group's patterns)
       |
       v
xml-lexer            (this crate — callback + glue layer)
```

## Pattern groups

| Group     | Active when...                  | Recognizes                         |
|-----------|---------------------------------|------------------------------------|
| default   | Between tags (initial state)    | TEXT, entities, tag/comment openers |
| tag       | Inside `<tag ...>` or `</tag>`  | TAG_NAME, ATTR_EQUALS, ATTR_VALUE  |
| comment   | Inside `<!-- ... -->`           | COMMENT_TEXT, COMMENT_END          |
| cdata     | Inside `<![CDATA[ ... ]]>`      | CDATA_TEXT, CDATA_END              |
| pi        | Inside `<? ... ?>`              | PI_TARGET, PI_TEXT, PI_END         |

## Token types

**Default group:**

| Token          | Example      | Description                 |
|----------------|--------------|-----------------------------|
| TEXT           | `hello`      | Text content between tags   |
| ENTITY_REF    | `&amp;`      | Named entity reference      |
| CHAR_REF      | `&#65;`      | Numeric character reference |
| OPEN_TAG_START | `<`         | Opening tag delimiter       |
| CLOSE_TAG_START | `</`       | Closing tag delimiter       |
| COMMENT_START  | `<!--`      | Comment opener              |
| CDATA_START    | `<![CDATA[` | CDATA section opener        |
| PI_START       | `<?`        | Processing instruction opener |

**Tag group:**

| Token       | Example    | Description              |
|-------------|------------|--------------------------|
| TAG_NAME    | `div`      | Tag or attribute name    |
| ATTR_EQUALS | `=`        | Attribute value delimiter |
| ATTR_VALUE  | `"main"`   | Quoted attribute value   |
| TAG_CLOSE   | `>`        | Tag closer               |
| SELF_CLOSE  | `/>`       | Self-closing tag closer  |

**Comment/CDATA/PI groups:** Each has text content and an end delimiter.

## Usage

```rust
use coding_adventures_xml_lexer::tokenize_xml;

let tokens = tokenize_xml("<p>Hello &amp; world</p>");
for token in &tokens {
    println!("{:?} {:?}", token.type_, token.value);
}
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_xml_lexer::create_xml_lexer;

let mut lexer = create_xml_lexer("<div>hello</div>");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## The callback

The `xml_on_token` callback fires after each token and drives group transitions:

```text
default --OPEN_TAG_START--> tag --TAG_CLOSE--> default
        --CLOSE_TAG_START-> tag --SELF_CLOSE-> default
        --COMMENT_START---> comment --COMMENT_END--> default
        --CDATA_START-----> cdata --CDATA_END--> default
        --PI_START--------> pi --PI_END--> default
```

For comment, CDATA, and PI groups, the callback disables skip patterns so whitespace is preserved as content.

## Running tests

```bash
cargo test -p coding-adventures-xml-lexer -- --nocapture
```
