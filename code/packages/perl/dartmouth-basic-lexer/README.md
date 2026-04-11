# CodingAdventures::DartmouthBasicLexer (Perl)

A grammar-driven tokenizer for the 1964 Dartmouth BASIC language. Reads the
shared `dartmouth_basic.tokens` grammar file, compiles the token definitions
into Perl regexes, and tokenizes BASIC source into a flat list of typed tokens.

## What it does

Given `10 LET X = 5`, produces:

| type     | value | line | col |
|----------|-------|------|-----|
| LINE_NUM | `10`  | 1    | 1   |
| KEYWORD  | `LET` | 1    | 4   |
| NAME     | `X`   | 1    | 8   |
| EQ       | `=`   | 1    | 10  |
| NUMBER   | `5`   | 1    | 12  |
| NEWLINE  | `\n`  | 1    | 13  |
| EOF      |       | 1    | 14  |

Horizontal whitespace is consumed silently. Newlines are kept (BASIC is
line-oriented — the parser needs them). The last token is always `EOF`.

## Special features

### Case insensitivity

The entire source is normalised to uppercase before tokenizing
(`@case_insensitive true`). This mirrors the original GE-225 teletypes, which
only had uppercase characters.

```perl
# All three produce KEYWORD("LET"):
CodingAdventures::DartmouthBasicLexer->tokenize('10 let x = 5');   # let → LET
CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 5');   # LET → LET
CodingAdventures::DartmouthBasicLexer->tokenize('10 Let X = 5');   # Let → LET
```

### LINE_NUM disambiguation

A bare integer plays two roles in BASIC:

- **Line label**: `10 LET X = 5` — the leading `10` is a LINE_NUM
- **Expression value**: `LET X = 42` — the `42` is a NUMBER
- **GOTO target**: `GOTO 100` — the `100` is a NUMBER

After the base tokenization pass, the module relabels the first NUMBER on each
source line as LINE_NUM. Integers inside statements remain NUMBER.

### REM comments

`REM` introduces a comment that runs to the end of the line. Everything after
`REM` is dropped from the token stream:

```perl
my $toks = CodingAdventures::DartmouthBasicLexer->tokenize('10 REM HELLO WORLD');
# Produces: LINE_NUM(10) KEYWORD(REM) NEWLINE EOF
# "HELLO" and "WORLD" are suppressed.
```

## Token types

### Program structure

| Token    | Example          | Notes                                         |
|----------|------------------|-----------------------------------------------|
| LINE_NUM | `10`, `999`      | Integer at the start of each source line      |
| NEWLINE  | `\n`, `\r\n`     | Statement terminator; kept in token stream    |
| EOF      |                  | Always the last token                         |

### Values

| Token      | Example              | Notes                                     |
|------------|----------------------|-------------------------------------------|
| NUMBER     | `42`, `3.14`, `1.5E3`, `.5` | All numeric literals in expressions |
| STRING     | `"HELLO WORLD"`      | Double-quoted; includes surrounding quotes |
| BUILTIN_FN | `SIN`, `LOG`, `RND`  | One of the 11 built-in functions          |
| USER_FN    | `FNA`, `FNZ`         | FN followed by exactly one letter         |
| NAME       | `X`, `A1`, `Z9`      | Variable: one letter + optional digit     |

### Keywords (all 20 reserved words)

| Token   | Keyword                                                     |
|---------|-------------------------------------------------------------|
| KEYWORD | `LET` `PRINT` `INPUT` `IF` `THEN` `GOTO` `GOSUB` `RETURN`  |
| KEYWORD | `FOR` `TO` `STEP` `NEXT` `END` `STOP` `REM`                |
| KEYWORD | `READ` `DATA` `RESTORE` `DIM` `DEF`                         |

### Operators

| Token     | Symbol | Notes                               |
|-----------|--------|-------------------------------------|
| PLUS      | `+`    |                                     |
| MINUS     | `-`    |                                     |
| STAR      | `*`    |                                     |
| SLASH     | `/`    |                                     |
| CARET     | `^`    | Exponentiation                      |
| EQ        | `=`    | Assignment in LET; equality in IF   |
| LT        | `<`    |                                     |
| GT        | `>`    |                                     |
| LE        | `<=`   | Matched before LT + EQ              |
| GE        | `>=`   | Matched before GT + EQ              |
| NE        | `<>`   | Not-equal; matched before LT + GT   |

### Punctuation

| Token     | Symbol | Notes                                           |
|-----------|--------|-------------------------------------------------|
| LPAREN    | `(`    |                                                 |
| RPAREN    | `)`    |                                                 |
| COMMA     | `,`    | PRINT zone separator (advance to next col 14)   |
| SEMICOLON | `;`    | PRINT concatenation (no space between values)   |

### Errors

| Token   | Notes                                              |
|---------|----------------------------------------------------|
| UNKNOWN | Unrecognized character; lexing continues after it  |

## Usage

```perl
use CodingAdventures::DartmouthBasicLexer;

my $source = "10 LET X = 1\n20 PRINT X\n30 END";
my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize($source);

for my $tok (@$tokens) {
    printf "%-10s  %-15s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

Output:
```
LINE_NUM    10               (line 1, col 1)
KEYWORD     LET              (line 1, col 4)
NAME        X                (line 1, col 8)
EQ          =                (line 1, col 10)
NUMBER      1                (line 1, col 12)
NEWLINE                      (line 1, col 13)
LINE_NUM    20               (line 2, col 1)
KEYWORD     PRINT            (line 2, col 4)
NAME        X                (line 2, col 10)
NEWLINE                      (line 2, col 11)
LINE_NUM    30               (line 3, col 1)
KEYWORD     END              (line 3, col 4)
NEWLINE                      (line 3, col 7)
EOF                          (line 3, col 8)
```

## How it fits in the stack

```
code/grammars/dartmouth_basic.tokens
        ↓  parsed by CodingAdventures::GrammarTools
    TokenGrammar
        ↓  compiled to Perl qr// rules
CodingAdventures::DartmouthBasicLexer   ← you are here
        ↓  post-processed (LINE_NUM relabelling, REM suppression)
    [token list]
        ↓  feeds
CodingAdventures::DartmouthBasicParser  (future package)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `dartmouth_basic.tokens`

## Running tests

```bash
PERL5LIB=../grammar-tools/lib prove -l -v t/
```
