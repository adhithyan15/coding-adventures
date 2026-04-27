# CodingAdventures::CSharpLexer (Perl)

A grammar-driven C# tokenizer. Reads the shared `csharp/csharp<version>.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes C# source into a flat list of typed tokens.

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

## C#-specific operators

C# includes several operators not found in Java:

| Operator | Name | Since | Meaning |
|----------|------|-------|---------|
| `??`  | null-coalescing        | C# 2.0 | Return left if non-null; else right |
| `?.`  | null-conditional access | C# 6.0 | Return null if left is null; else access member |
| `??=` | null-coalescing assign  | C# 8.0 | Assign right to left only if left is null |

## Version support

| Version  | C# Release         |
|----------|--------------------|
| `"1.0"`  | C# 1.0 (2002)      |
| `"2.0"`  | C# 2.0 (2005)      |
| `"3.0"`  | C# 3.0 (2007)      |
| `"4.0"`  | C# 4.0 (2010)      |
| `"5.0"`  | C# 5.0 (2012)      |
| `"6.0"`  | C# 6.0 (2015)      |
| `"7.0"`  | C# 7.0 (2017)      |
| `"8.0"`  | C# 8.0 (2019)      |
| `"9.0"`  | C# 9.0 (2020)      |
| `"10.0"` | C# 10.0 (2021)     |
| `"11.0"` | C# 11.0 (2022)     |
| `"12.0"` | C# 12.0 (2023)     |

Default version: `"12.0"` (when no version is specified).

## Usage

```perl
use CodingAdventures::CSharpLexer;

# Default (C# 12.0)
my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;');

# Version-specific
my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;', '8.0');

for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}

# Convenience functions
use CodingAdventures::CSharpLexer qw(tokenize_csharp);
my $tokens = tokenize_csharp('string s = "hello";', '6.0');
```

## How it fits in the stack

```
csharp/csharp<version>.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::CSharpLexer  ← you are here
    ↓  feeds
CodingAdventures::CSharpParser
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `csharp<version>.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```

## Version

0.01
