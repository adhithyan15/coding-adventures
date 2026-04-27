# TE04 — HTML 1.0 Lexer

## Overview

HTML was invented by Tim Berners-Lee in 1989-1991 at CERN, the European
particle physics laboratory in Geneva. Berners-Lee needed a way for physicists
to share documents with hyperlinks between them — click a reference in one paper
and jump to another paper on a different computer. He combined two existing ideas:
SGML (a complex document markup standard used by the publishing industry) and
hypertext (clickable links between documents, a concept from Ted Nelson in the
1960s). The result was HTML: a deliberately simplified subset of SGML with a few
dozen tags and a forgiving parser.

The first version of HTML had just 18 tags. By 1993, when Marc Andreessen's NCSA
Mosaic browser shipped and brought the Web to the general public, a few more had
been added — `<img>` for images, `<br>` for line breaks, `<hr>` for horizontal
rules. This early tag set was never formally standardised as an RFC. The first
formal specification was HTML 2.0 (RFC 1866, November 1995), which codified what
browsers already understood. But the tag set that Mosaic understood is
well-documented, and that is what this lexer targets.

### HTML Is Not XML

This is the single most important thing to understand before implementing an HTML
lexer, and it is why `html1.0-lexer` is a **separate package** from the existing
`xml-lexer` in this repo.

XML was created in 1998 as a strict, machine-friendly subset of SGML. HTML
predates XML by nearly a decade. The two languages look superficially similar —
angle brackets, tag names, attributes — but their rules are fundamentally
different:

| Property | XML | HTML |
|---|---|---|
| Tag closure | Required: `<br/>` | Optional: `<br>` is valid |
| Case sensitivity | Yes: `<P>` != `<p>` | No: `<P>` = `<p>` |
| Attribute quoting | Required: `src="x"` | Optional: `src=x` is valid |
| Entity declarations | Must be declared in DTD | Built-in: `&amp;`, `&lt;`, etc. |
| Error handling | Fatal: parser aborts | Tolerant: parser recovers and continues |
| Self-closing syntax | `<br/>` required for void elements | `<br>` sufficient; `<br/>` also accepted |

The XML lexer in this repo rejects malformed input. The HTML lexer **never**
rejects input. Real-world HTML from 1993 was full of unclosed tags, mismatched
nesting, bare ampersands, and unquoted attributes. Mosaic rendered all of it.
This lexer must do the same.

### The Mail-Sorting Analogy

Think of the HTML lexer as a mail room worker sorting incoming packages. The
worker's job is simple: look at each piece of mail and put it in the right bin.

- A piece of mail in an angle-bracket envelope (`<p>`) goes in the **tag** bin.
- A piece of mail with an ampersand stamp (`&amp;`) goes in the **entity** bin,
  gets decoded, and the decoded content goes in the **text** bin.
- Everything else goes straight in the **text** bin.

The mail room worker does not read the letters. They do not decide whether a
`<p>` should be followed by a `</p>`, or whether `<img>` can appear inside
`<title>`. That is the job of the parser (TE05), who reads the sorted bins and
builds the document tree. The lexer just sorts.

---

## Where It Fits

```
html1.0-lexer (TE04) <-- THIS PACKAGE
     |  Vec<HtmlToken>
     v
html1.0-parser (TE05) -- builds DOM tree from token stream
     |
     v
document-ast (TE00) -- universal document IR
```

**Depends on:** nothing (pure string processing, zero external dependencies)

**Depended on by:** `html1.0-parser` (TE05)

**Follows the pattern of:** `xml-lexer`, `css-lexer`, `javascript-lexer` in this
repo — each language has a separate lexer package that produces tokens, and a
separate parser package that builds a tree from those tokens.

---

## Concepts

### What Is a Lexer?

A lexer (also called a tokenizer or scanner) is the first stage of any language
processor. Its job is to take a flat string of characters and break it into
meaningful chunks called **tokens**.

Consider this analogy: imagine you are reading a sentence in a foreign language.
Before you can understand the grammar (parsing), you must first identify where
each word begins and ends (lexing). In English, words are separated by spaces.
In HTML, tokens are separated by structural characters like `<`, `>`, `&`, and
`=`.

```
Input:   <p class="intro">Hello &amp; world</p>

Lexer output (tokens):
  1. StartTag { name: "p", attributes: [("class", "intro")], self_closing: false }
  2. Text("Hello & world")
  3. EndTag { name: "p" }
```

Notice three things:

1. The angle brackets, quotes, and attribute syntax have been consumed — the
   tokens carry structured data, not raw syntax.
2. The entity `&amp;` has been decoded to `&` — the text token contains the
   final character, not the escape sequence.
3. The lexer does not know or care that `<p>` opens a paragraph. It just
   recognises that `<p class="intro">` is a start tag with one attribute.

### What Is a State Machine?

The HTML lexer is implemented as a **finite state machine**. This is a
programming pattern where the code is always in exactly one "state", and each
input character causes a transition to the same or a different state.

Think of it like a vending machine. The machine is in one state at a time:
"waiting for money", "waiting for selection", "dispensing product". Each input
(coin inserted, button pressed) transitions it to the next state. The machine
does not need to remember its entire history — only its current state and the
current input.

The HTML lexer has these states:

```
                    +----------+
    text chars      |          |   '<'
   +--------------->|   Data   |----------> TagOpen
   |                |          |
   |                +----------+
   |                     |
   |                     | '&'
   |                     v
   |                EntityRef ---- ';' --> back to Data (decoded char appended to text)
   |
   |
TagOpen
   |
   +-- '/' --> EndTagOpen --> TagName (collecting close tag name)
   +-- '!' --> MarkupDecl --> Comment or DOCTYPE
   +-- letter --> TagName (collecting open tag name)
                    |
                    +-- whitespace --> BeforeAttrName
                    +-- '>' --> emit StartTag, go to Data
                    +-- '/' --> SelfClosingTag --> '>' --> emit, go to Data

BeforeAttrName
   +-- letter --> AttrName (collecting attribute name)
   +-- '>' --> emit StartTag, go to Data
   +-- '/' --> SelfClosingTag

AttrName
   +-- '=' --> BeforeAttrValue
   +-- whitespace --> BeforeAttrName (attribute with no value)
   +-- '>' --> emit StartTag, go to Data

BeforeAttrValue
   +-- '"' --> AttrValueDoubleQuoted
   +-- "'" --> AttrValueSingleQuoted
   +-- other --> AttrValueUnquoted (read until whitespace or '>')
```

Each state is a function (or a match arm in Rust) that reads one character,
decides what to do with it, and transitions to the next state.

---

## Token Types

The lexer produces a stream of `HtmlToken` values. There are exactly six
variants, mirroring the six kinds of "thing" you can encounter in an HTML
document:

```rust
/// A single token produced by the HTML 1.0 lexer.
///
/// The lexer is error-tolerant: it always produces tokens, never errors.
/// Malformed input is handled gracefully — see the "Error Tolerance"
/// section of the spec for the exact recovery rules.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlToken {
    /// An opening tag, possibly with attributes.
    ///
    /// Examples:
    ///   <p>                → StartTag { name: "p", attributes: [], self_closing: false }
    ///   <img src="x.gif"> → StartTag { name: "img", attributes: [("src", "x.gif")], self_closing: false }
    ///   <br/>              → StartTag { name: "br", attributes: [], self_closing: true }
    StartTag {
        name: String,                          // always lowercased: "p", "h1", "img"
        attributes: Vec<(String, String)>,     // (name, value) pairs, names lowercased
        self_closing: bool,                    // true for <br/> or <img ... />
    },

    /// A closing tag.
    ///
    /// Examples:
    ///   </p>  → EndTag { name: "p" }
    ///   </P>  → EndTag { name: "p" }   (case-normalised)
    EndTag {
        name: String,  // always lowercased
    },

    /// Raw text content between tags, with entities already decoded.
    ///
    /// Examples:
    ///   "Hello world"       → Text("Hello world")
    ///   "Hello &amp; world" → Text("Hello & world")   (entity decoded)
    Text(String),

    /// An HTML comment.
    ///
    /// Example:
    ///   <!-- TODO: fix this --> → Comment(" TODO: fix this ")
    Comment(String),

    /// A DOCTYPE declaration.
    ///
    /// Example:
    ///   <!DOCTYPE html> → Doctype("html")
    Doctype(String),

    /// Signals the end of input. Always the last token in the stream.
    Eof,
}
```

### Why These Six?

Every character in an HTML document belongs to exactly one of these categories.
There is no seventh thing. You are either inside a tag, between tags, inside a
comment, inside a DOCTYPE, or at the end of the file. This exhaustive
partitioning is what makes the lexer a complete front-end: the parser never needs
to look at raw characters — it only sees tokens.

---

## Entity Decoding

HTML uses **entities** (also called character references) to represent characters
that have special meaning in the syntax or that cannot be typed directly on a
keyboard. The entity `&amp;` means "the literal ampersand character `&`", because
a bare `&` would otherwise start an entity reference.

### Named Entities

HTML 1.0 has exactly five named entities. These are the only ones the lexer must
recognise:

| Entity | Character | Code Point | Why It Exists |
|--------|-----------|-----------|---------------|
| `&amp;`  | & | U+0026 | Bare `&` starts entity references |
| `&lt;`   | < | U+003C | Bare `<` starts tags |
| `&gt;`   | > | U+003E | Symmetry with `&lt;`; also useful in attributes |
| `&quot;` | " | U+0022 | Bare `"` ends attribute values |
| `&nbsp;` | (non-breaking space) | U+00A0 | Prevents word-wrap between words |

### Numeric Character References

In addition to named entities, HTML supports numeric references that specify a
Unicode code point directly:

- **Decimal:** `&#60;` means "the character at code point 60 decimal", which is
  `<` (U+003C).
- **Hexadecimal:** `&#x3C;` (or `&#X3c;` — the `x` and hex digits are
  case-insensitive) means "the character at code point 3C hex", which is also
  `<` (U+003C).

### Unknown Entities

Unknown named entities pass through literally. If the lexer encounters `&foo;`,
it does not recognise `foo` as a named entity, so it emits the text `&foo;`
unchanged. This matches Mosaic's behaviour — early browsers did not crash or
strip unknown entities.

### Unterminated Entities

If the lexer encounters `&amp` without a trailing semicolon, it treats the entire
sequence as literal text: `&amp`. The semicolon is required for entity
recognition.

---

## Error Tolerance

The HTML lexer **never fails**. It always produces a token stream. This is a
fundamental design requirement — not a nice-to-have. Real-world HTML from the
1990s was full of errors, and Mosaic rendered all of it. The lexer follows
Postel's Law: "Be conservative in what you send, liberal in what you accept."

Here are the specific recovery rules:

| Malformed Input | Recovery | Output |
|---|---|---|
| Bare `<` not followed by letter, `/`, or `!` | Treat as literal text | `Text("<")` |
| Unterminated tag `<p` at EOF | Emit tag with what we have | `StartTag { name: "p", ... }` |
| Unterminated attribute value `<a href="foo` at EOF | Close value at EOF | `StartTag { name: "a", attributes: [("href", "foo")] }` |
| Unterminated comment `<!-- hello` at EOF | Close comment at EOF | `Comment(" hello")` |
| Unterminated entity `&amp` (no semicolon) | Treat as literal text | `Text("&amp")` |
| Null byte (`\0`) in input | Replace with U+FFFD | Replacement character in output |
| `</` not followed by letter | Treat `</` as literal text | `Text("</")` |

The guiding principle is: **do not lose information**. If the lexer cannot
interpret a character sequence as markup, it emits it as text. The user sees the
raw characters rather than having content silently dropped.

---

## Public API

The package exposes two ways to tokenize HTML: a convenience function that
returns all tokens at once, and a streaming iterator for large documents.

### Convenience Function

```rust
/// Tokenize an HTML string into a sequence of tokens.
///
/// This is the simplest way to use the lexer. It reads the entire input and
/// returns a `Vec` of all tokens. The last token is always `HtmlToken::Eof`.
///
/// The lexer is error-tolerant: it NEVER returns `Err`. Malformed input is
/// handled gracefully and produces the best-effort token stream.
///
/// # Example
///
/// ```rust
/// use html1_lexer::{tokenize, HtmlToken};
///
/// let tokens = tokenize("<p>Hello &amp; world</p>");
/// assert_eq!(tokens, vec![
///     HtmlToken::StartTag {
///         name: "p".into(),
///         attributes: vec![],
///         self_closing: false,
///     },
///     HtmlToken::Text("Hello & world".into()),
///     HtmlToken::EndTag { name: "p".into() },
///     HtmlToken::Eof,
/// ]);
/// ```
pub fn tokenize(input: &str) -> Vec<HtmlToken> { ... }
```

### Streaming Iterator

```rust
/// A streaming HTML tokenizer.
///
/// The iterator yields tokens one at a time without buffering the entire
/// token stream in memory. This is important for large documents — a multi-
/// megabyte HTML file should not require a multi-megabyte Vec allocation
/// just to get the first token.
///
/// # Example
///
/// ```rust
/// use html1_lexer::{HtmlLexer, HtmlToken};
///
/// let mut lexer = HtmlLexer::new("<p>Hello</p>");
/// assert_eq!(lexer.next(), Some(HtmlToken::StartTag {
///     name: "p".into(),
///     attributes: vec![],
///     self_closing: false,
/// }));
/// assert_eq!(lexer.next(), Some(HtmlToken::Text("Hello".into())));
/// assert_eq!(lexer.next(), Some(HtmlToken::EndTag { name: "p".into() }));
/// assert_eq!(lexer.next(), Some(HtmlToken::Eof));
/// assert_eq!(lexer.next(), None);  // exhausted after Eof
/// ```
pub struct HtmlLexer<'a> {
    // Internal fields: input slice, current position, current state, etc.
    // These are private implementation details.
}

impl<'a> HtmlLexer<'a> {
    /// Create a new lexer for the given HTML input string.
    pub fn new(input: &'a str) -> Self { ... }
}

impl<'a> Iterator for HtmlLexer<'a> {
    type Item = HtmlToken;

    /// Returns the next token, or `None` after `Eof` has been yielded.
    ///
    /// The iterator yields `Some(HtmlToken::Eof)` exactly once at the end
    /// of input, then returns `None` on all subsequent calls.
    fn next(&mut self) -> Option<HtmlToken> { ... }
}
```

### Design Rationale

**Why both `tokenize` and `HtmlLexer`?** Most callers just want all the tokens
and will use `tokenize`. But the streaming API exists for two reasons:

1. **Memory efficiency.** A parser processing a 10 MB HTML file does not need to
   allocate 10 MB of tokens upfront. It can consume tokens one at a time.

2. **Early termination.** A caller searching for the `<title>` tag can stop
   iteration as soon as it finds `</title>`, without tokenizing the rest of the
   document.

**Why does `tokenize` return a `Vec` and not an iterator?** Because `Vec` is
the simplest API for the common case. Callers who need an iterator can call
`.into_iter()` on the `Vec`, or use `HtmlLexer` directly.

---

## The Lexer vs. The Parser

The **lexer** (this package, TE04) answers: "What are the tokens?"

```
Input:  <p class="intro">Hello &amp; world</p>

Output: [
    StartTag("p", [("class", "intro")], false),
    Text("Hello & world"),
    EndTag("p"),
    Eof,
]
```

The **parser** (TE05, a future package) answers: "What is the tree structure?"

```
ParagraphElement
  class="intro"
  children:
    TextNode("Hello & world")
```

The parser knows things the lexer does not:

- `<p>` opens a paragraph and `</p>` closes it.
- `<p>one<p>two` means two paragraphs (the second `<p>` implicitly closes the
  first).
- `<img>` is void — it never has a closing tag, even without `/>`.
- `<b>` inside `<title>` is literal text, not a tag.

The lexer knows **nothing** about tag semantics. It does not know which tags are
void, which can nest, or what attributes mean. It just finds tokens in the
character stream. This separation of concerns makes both the lexer and the parser
simpler, more testable, and independently reusable.

---

## Case Normalisation

HTML is case-insensitive for tag names and attribute names. The lexer normalises
all tag names and attribute names to **lowercase** during tokenization:

```
<P>           → StartTag { name: "p", ... }
<IMG SRC="x"> → StartTag { name: "img", attributes: [("src", "x")], ... }
</BODY>       → EndTag { name: "body" }
```

Attribute **values** are NOT normalised — they are preserved exactly as written:

```
<a href="Page.HTML"> → StartTag { name: "a", attributes: [("href", "Page.HTML")], ... }
```

This matches how every browser has handled HTML since the beginning: tag names
and attribute names are case-insensitive, but attribute values are
case-sensitive (because URLs, filenames, and CSS class names are
case-sensitive).

---

## HTML 1.0 Tag Inventory

For reference, here are the tags that the Mosaic-era Web understood. The lexer
does not use this list — it tokenizes ANY tag name — but the parser (TE05) will
use it for semantic decisions.

### Block-Level Tags

| Tag | Purpose | Closing Tag |
|-----|---------|-------------|
| `<html>` | Document root | `</html>` |
| `<head>` | Document metadata container | `</head>` |
| `<body>` | Document content container | `</body>` |
| `<title>` | Document title (in head) | `</title>` |
| `<h1>`...`<h6>` | Section headings | `</h1>`...`</h6>` |
| `<p>` | Paragraph | `</p>` (often omitted) |
| `<ul>` | Unordered list | `</ul>` |
| `<ol>` | Ordered list | `</ol>` |
| `<li>` | List item | `</li>` (often omitted) |
| `<dl>` | Definition list | `</dl>` |
| `<dt>` | Definition term | `</dt>` (often omitted) |
| `<dd>` | Definition description | `</dd>` (often omitted) |
| `<pre>` | Preformatted text | `</pre>` |
| `<blockquote>` | Block quotation | `</blockquote>` |
| `<address>` | Contact information | `</address>` |
| `<hr>` | Horizontal rule | None (void element) |

### Inline Tags

| Tag | Purpose | Closing Tag |
|-----|---------|-------------|
| `<a>` | Anchor (hyperlink) | `</a>` |
| `<img>` | Image | None (void element) |
| `<br>` | Line break | None (void element) |
| `<em>` | Emphasis (italic) | `</em>` |
| `<strong>` | Strong emphasis (bold) | `</strong>` |
| `<code>` | Inline code | `</code>` |
| `<b>` | Bold | `</b>` |
| `<i>` | Italic | `</i>` |
| `<tt>` | Teletype (monospace) | `</tt>` |
| `<cite>` | Citation | `</cite>` |
| `<var>` | Variable | `</var>` |
| `<kbd>` | Keyboard input | `</kbd>` |
| `<samp>` | Sample output | `</samp>` |

---

## Testing Strategy

### 1. Basic Tags

Verify that simple opening and closing tags are tokenized correctly:

```rust
tokenize("<p>")       // → [StartTag("p", [], false), Eof]
tokenize("</p>")      // → [EndTag("p"), Eof]
tokenize("<br>")       // → [StartTag("br", [], false), Eof]
tokenize("<br/>")      // → [StartTag("br", [], true), Eof]
tokenize("<img src=\"x\">")  // → [StartTag("img", [("src","x")], false), Eof]
```

### 2. Attributes

Test all attribute value styles and edge cases:

- Double-quoted: `<a href="http://example.com">`
- Single-quoted: `<a href='http://example.com'>`
- Unquoted: `<a href=http://example.com>`
- Boolean (no value): `<option selected>`
- Multiple attributes: `<img src="x.gif" alt="photo" width=100>`
- Attribute with empty value: `<input value="">`

### 3. Entities

Test all five named entities plus numeric references:

- `&amp;` decodes to `&`
- `&lt;` decodes to `<`
- `&gt;` decodes to `>`
- `&quot;` decodes to `"`
- `&nbsp;` decodes to U+00A0
- `&#60;` decodes to `<` (decimal)
- `&#x3C;` decodes to `<` (hex)
- `&#X3c;` decodes to `<` (hex, case-insensitive)
- `&foo;` passes through as `&foo;` (unknown entity)

### 4. Text

- Plain text with no markup
- Text with entities mixed in
- Whitespace-only text (spaces, tabs, newlines)
- Text immediately adjacent to tags: `<b>bold</b>text`

### 5. Comments

- Standard comment: `<!-- hello -->`
- Empty comment: `<!---->`
- Comment with dashes: `<!-- -- -->`
- Comment with no spaces: `<!--hello-->`

### 6. Doctype

- Standard: `<!DOCTYPE html>`
- Lowercase: `<!doctype HTML>`
- With extra content: `<!DOCTYPE html SYSTEM "about:legacy-compat">`

### 7. Case Insensitivity

- `<P>` produces `StartTag { name: "p" }`
- `<IMG SRC="x">` produces `StartTag { name: "img", attributes: [("src", "x")] }`
- `</BODY>` produces `EndTag { name: "body" }`
- Attribute values preserve case: `<a HREF="Page.HTML">` → `("href", "Page.HTML")`

### 8. Error Tolerance

- Bare `<` not followed by valid tag start: `a < b` → `[Text("a < b"), Eof]`
- Unterminated tag at EOF: `<p` → `[StartTag("p", [], false), Eof]`
- Unterminated attribute value: `<a href="foo` → `[StartTag("a", [("href","foo")]), Eof]`
- Unterminated comment: `<!-- hello` → `[Comment(" hello"), Eof]`
- Unterminated entity: `&amp` → `[Text("&amp"), Eof]`
- Null bytes: `hello\0world` → `[Text("hello\u{FFFD}world"), Eof]`

### 9. Real HTML

Tokenize actual HTML from the earliest web pages:

- The CERN info page (`http://info.cern.ch/`, archived 1993)
- The original Mosaic "What's New" page
- Hand-crafted HTML that exercises all Mosaic-era features

These integration tests verify that the lexer handles real-world markup, not just
synthetic test cases.

### 10. Empty Input

- Empty string `""` produces `[Eof]`
- Whitespace-only `"   "` produces `[Text("   "), Eof]`

### 11. Streaming Iterator

- `HtmlLexer` produces the same tokens as `tokenize` for all test cases
- After yielding `Eof`, `next()` returns `None` on all subsequent calls
- Large input (multi-MB) does not cause excessive memory allocation — verify
  with a memory-bounded test

---

## Scope

### In Scope

- All HTML 1.0 era tokenization (tags, attributes, text, comments, doctype)
- Case normalisation (tag and attribute names lowercased)
- Entity decoding (5 named entities + decimal and hex numeric references)
- Error tolerance (never fails, always produces tokens)
- Streaming iterator API for memory-efficient processing
- Null byte replacement with U+FFFD

### Out of Scope

- **Tag semantics** — which tags self-close, which can nest, which are void.
  That is the parser's job (TE05).
- **CDATA sections** — HTML has no CDATA. That is an XML concept.
- **Processing instructions** (`<?xml ...?>`) — that is XML, not HTML.
- **Script/style content** (`<script>`, `<style>`) — these were not part of
  HTML 1.0. They arrive with HTML 2.0+ and require special "raw text" lexer
  states. A future `html2.0-lexer` will handle them.
- **Character encoding detection** — the lexer assumes its input is valid UTF-8.
  Encoding sniffing (reading `<meta charset>` or BOM) is a separate concern.
- **Named entities beyond the core 5** — HTML 2.0 and later added hundreds of
  named entities (`&eacute;`, `&mdash;`, etc.). A future `html2.0-lexer` will
  include the full entity table.

---

## Implementation Languages

This package will be implemented in:

- **Rust** (primary, for use in the Venture browser pipeline)
- Future: all 9 languages following the standard pattern in this repo

### Package Layout (Rust)

```
code/packages/rust/html1-lexer/
  src/
    lib.rs          -- public API: tokenize(), HtmlLexer, HtmlToken
    token.rs        -- HtmlToken enum definition
    lexer.rs        -- HtmlLexer state machine implementation
    entities.rs     -- entity decoding (5 named + numeric)
  Cargo.toml        -- { name = "html1-lexer" }
  BUILD
  README.md
  CHANGELOG.md
```

---

## Worked Examples

### Example 1: Simple Document

```
Input:
  <!DOCTYPE html>
  <html>
  <head><title>Hello</title></head>
  <body>
  <h1>Welcome</h1>
  <p>This is a <a href="page2.html">link</a>.</p>
  </body>
  </html>

Token stream:
  Doctype("html")
  Text("\n")
  StartTag { name: "html", attributes: [], self_closing: false }
  Text("\n")
  StartTag { name: "head", attributes: [], self_closing: false }
  StartTag { name: "title", attributes: [], self_closing: false }
  Text("Hello")
  EndTag { name: "title" }
  EndTag { name: "head" }
  Text("\n")
  StartTag { name: "body", attributes: [], self_closing: false }
  Text("\n")
  StartTag { name: "h1", attributes: [], self_closing: false }
  Text("Welcome")
  EndTag { name: "h1" }
  Text("\n")
  StartTag { name: "p", attributes: [], self_closing: false }
  Text("This is a ")
  StartTag { name: "a", attributes: [("href", "page2.html")], self_closing: false }
  Text("link")
  EndTag { name: "a" }
  Text(".")
  EndTag { name: "p" }
  Text("\n")
  EndTag { name: "body" }
  Text("\n")
  EndTag { name: "html" }
  Eof
```

### Example 2: Sloppy 1993-Era HTML

```
Input:
  <HTML>
  <BODY BGCOLOR=white>
  <H1>My Cool Page</H1>
  <P>Welcome to my page! Click <A HREF=links.html>here</A> for links.
  <P>Here is a picture:<BR>
  <IMG SRC=cool.gif ALT="My photo">
  <HR>
  <ADDRESS>webmaster@example.com</ADDRESS>
  </BODY></HTML>

Token stream:
  StartTag { name: "html", attributes: [], self_closing: false }
  Text("\n")
  StartTag { name: "body", attributes: [("bgcolor", "white")], self_closing: false }
  Text("\n")
  StartTag { name: "h1", attributes: [], self_closing: false }
  Text("My Cool Page")
  EndTag { name: "h1" }
  Text("\n")
  StartTag { name: "p", attributes: [], self_closing: false }
  Text("Welcome to my page! Click ")
  StartTag { name: "a", attributes: [("href", "links.html")], self_closing: false }
  Text("here")
  EndTag { name: "a" }
  Text(" for links.\n")
  StartTag { name: "p", attributes: [], self_closing: false }
  Text("Here is a picture:")
  StartTag { name: "br", attributes: [], self_closing: false }
  Text("\n")
  StartTag { name: "img", attributes: [("src", "cool.gif"), ("alt", "My photo")], self_closing: false }
  Text("\n")
  StartTag { name: "hr", attributes: [], self_closing: false }
  Text("\n")
  StartTag { name: "address", attributes: [], self_closing: false }
  Text("webmaster@example.com")
  EndTag { name: "address" }
  Text("\n")
  EndTag { name: "body" }
  EndTag { name: "html" }
  Eof
```

Notice: the second `<P>` has no closing `</P>`. The lexer does not care — it
just emits `StartTag("p")`. The parser (TE05) will handle implicit closing.
Also note that `BGCOLOR=white` uses an unquoted attribute value, and all tag
names are uppercased in the source but lowercased in the tokens.
