# CodingAdventures::XmlLexer (Perl)

A context-sensitive XML tokenizer. Reads the shared `xml.tokens` grammar file, compiles token definitions per group into Perl regexes, and maintains a pattern-group stack to handle XML's context-sensitive lexical rules.

## What it does

Given `<root attr="v">text</root>`, produces tokens including:

| type            | value  |
|-----------------|--------|
| OPEN_TAG_START  | `<`    |
| TAG_NAME        | `root` |
| TAG_NAME        | `attr` |
| ATTR_EQUALS     | `=`    |
| ATTR_VALUE      | `"v"`  |
| TAG_CLOSE       | `>`    |
| TEXT            | `text` |
| CLOSE_TAG_START | `</`   |
| TAG_NAME        | `root` |
| TAG_CLOSE       | `>`    |
| EOF             |        |

## Context-sensitive groups

| Trigger token     | Action              |
|-------------------|---------------------|
| OPEN_TAG_START    | push `tag` group    |
| CLOSE_TAG_START   | push `tag` group    |
| TAG_CLOSE         | pop group           |
| SELF_CLOSE        | pop group           |
| COMMENT_START     | push `comment` group|
| COMMENT_END       | pop group           |
| CDATA_START       | push `cdata` group  |
| CDATA_END         | pop group           |
| PI_START          | push `pi` group     |
| PI_END            | pop group           |

## Token types

See `xml.tokens` grammar for the full list: TEXT, ENTITY_REF, CHAR_REF, COMMENT_START/TEXT/END, CDATA_START/TEXT/END, PI_START/TARGET/TEXT/END, OPEN_TAG_START, CLOSE_TAG_START, TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE, SLASH, EOF.

## Usage

```perl
use CodingAdventures::XmlLexer;

my $tokens = CodingAdventures::XmlLexer->tokenize('<root attr="val">text</root>');
for my $tok (@$tokens) {
    printf "%s  %s\n", $tok->{type}, $tok->{value};
}
```

## How it fits in the stack

```
xml.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar  (with pattern groups)
    ↓  compiled to Perl qr// rules per group
CodingAdventures::XmlLexer  ← you are here
    ↓  feeds
xml_parser  (future)
```

## Running tests

```bash
prove -l -v t/
```
