# CodingAdventures::Asciidoc

A self-contained Perl module that parses AsciiDoc markup and renders it to
HTML.  No external dependencies — everything lives in a single `.pm` file.

## What is AsciiDoc?

AsciiDoc is a structured markup language designed for technical writing.  It
is richer than Markdown and powers the Asciidoctor toolchain used to produce
books, API documentation, and man pages.

**Critical difference from Markdown:** in AsciiDoc, `*text*` produces
**strong (bold)**, not emphasis.  Emphasis uses `_text_`.

## Supported features

### Block elements

| Feature             | Syntax                            |
|---------------------|----------------------------------|
| Heading 1–6         | `= H1` … `====== H6`            |
| Thematic break      | `'''` (3+ single quotes)        |
| Paragraph           | consecutive non-blank lines      |
| Code block          | `----` … `----`                 |
| Literal block       | `....` … `....`                 |
| Passthrough block   | `++++` … `++++` (raw HTML)      |
| Quote block         | `____` … `____`                 |
| Unordered list      | `* item` / `** nested`          |
| Ordered list        | `. item` / `.. nested`          |
| Source annotation   | `[source,lang]`                 |
| Comment             | `// text` (silently skipped)    |

### Inline elements

| Feature             | Syntax                            |
|---------------------|----------------------------------|
| Strong (bold)       | `*text*` or `**text**`          |
| Emphasis (italic)   | `_text_` or `__text__`          |
| Inline code         | `` `code` ``                    |
| Link                | `link:url[text]`                |
| Image               | `image:url[alt]`                |
| Cross-reference     | `<<anchor,text>>`               |
| Bare URL            | `https://…` or `http://…`      |
| Hard break          | `+` at end of line              |

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::Asciidoc qw(to_html parse);

# One-step: AsciiDoc → HTML
my $html = to_html("= Hello\n\nWorld\n");
# → "<h1>Hello</h1>\n<p>World</p>\n"

# Two-step: parse then inspect AST
my $blocks = parse("= Title\n\n*bold* text\n");
for my $block (@$blocks) {
    print $block->{type}, "\n";   # "heading", "paragraph", …
}
```

## Running tests

```bash
prove -l -v t/
```

## License

MIT
