# XML Lexer (Elixir)

Context-sensitive XML tokenizer using pattern groups and an on-token callback.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.XmlLexer.tokenize(~s(<p class="main">Hello &amp; world</p>))
# => [%Token{type: "OPEN_TAG_START"}, %Token{type: "TAG_NAME", value: "p"}, ...]
```

## How It Works

XML is context-sensitive at the lexical level -- the same character means different things depending on position. This lexer uses **pattern groups** from the `xml.tokens` grammar to handle five contexts:

- **default**: Text content, entity refs, tag/comment/CDATA/PI openers
- **tag**: Tag names, attributes, equals, quoted values, closers
- **comment**: Comment text and `-->` delimiter
- **cdata**: Raw text and `]]>` delimiter
- **pi**: Processing instruction target, text, and `?>` delimiter

The `xml_on_token/2` callback returns action tuples that push/pop groups and toggle skip patterns as the lexer encounters context boundaries.

## Dependencies

- `grammar_tools` — parses `.tokens` files
- `lexer` — grammar-driven tokenization engine with pattern group and callback support
