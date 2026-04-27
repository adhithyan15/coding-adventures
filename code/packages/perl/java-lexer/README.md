# CodingAdventures::JavaLexer (Perl)

A grammar-driven Java tokenizer. Reads the shared `java/java<version>.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes Java source into a flat list of typed tokens.

## What it does

Given `int x = 42;`, produces:

| type      | value | line | col |
|-----------|-------|------|-----|
| INT       | `int` | 1    | 1   |
| NAME      | `x`   | 1    | 5   |
| EQUALS    | `=`   | 1    | 7   |
| NUMBER    | `42`  | 1    | 9   |
| SEMICOLON | `;`   | 1    | 11  |
| EOF       |       | 1    | 12  |

Whitespace is consumed silently. The last token is always `EOF`.

## Version support

| Version | Java Release |
|---------|-------------|
| `"1.0"` | Java 1.0 (1996) |
| `"1.1"` | Java 1.1 (1997) |
| `"1.4"` | Java 1.4 (2002) |
| `"5"`   | Java 5 (2004) |
| `"7"`   | Java 7 (2011) |
| `"8"`   | Java 8 (2014) |
| `"10"`  | Java 10 (2018) |
| `"14"`  | Java 14 (2020) |
| `"17"`  | Java 17 (2021) |
| `"21"`  | Java 21 (2023) |

Default version: `"21"` (when no version is specified).

## Usage

```perl
use CodingAdventures::JavaLexer;

# Default (Java 21)
my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;');

# Version-specific
my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '1.0');

for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
java/java<version>.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::JavaLexer  ← you are here
    ↓  feeds
CodingAdventures::JavaParser
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `java<version>.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
