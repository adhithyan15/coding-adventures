# CodingAdventures::PythonLexer (Perl)

A grammar-driven Python tokenizer. Reads the shared `python.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes Python source into a flat list of typed tokens.

## What it does

Given `def foo(x):`, produces:

| type   | value  | line | col |
|--------|--------|------|-----|
| DEF    | `def`  | 1    | 1   |
| NAME   | `foo`  | 1    | 5   |
| LPAREN | `(`    | 1    | 8   |
| NAME   | `x`    | 1    | 9   |
| RPAREN | `)`    | 1    | 10  |
| COLON  | `:`    | 1    | 11  |
| EOF    |        | 1    | 12  |

Whitespace is consumed silently. The last token is always `EOF`.

## Token types

### Literals
| Token  | Example |
|--------|---------|
| NAME   | `my_var`, `_private`, `__init__` |
| NUMBER | `42`, `0` |
| STRING | `"hello"`, `"a\nb"` |

### Keywords
| Token  | Keyword   |
|--------|-----------|
| IF     | `if`      |
| ELIF   | `elif`    |
| ELSE   | `else`    |
| WHILE  | `while`   |
| FOR    | `for`     |
| DEF    | `def`     |
| RETURN | `return`  |
| CLASS  | `class`   |
| IMPORT | `import`  |
| FROM   | `from`    |
| AS     | `as`      |
| TRUE   | `True`    |
| FALSE  | `False`   |
| NONE   | `None`    |

### Operators
| Token         | Symbol |
|---------------|--------|
| EQUALS_EQUALS | `==`   |
| EQUALS        | `=`    |
| PLUS          | `+`    |
| MINUS         | `-`    |
| STAR          | `*`    |
| SLASH         | `/`    |

### Delimiters
| Token  | Symbol |
|--------|--------|
| LPAREN | `(`    |
| RPAREN | `)`    |
| COMMA  | `,`    |
| COLON  | `:`    |

## Usage

```perl
use CodingAdventures::PythonLexer;

my $tokens = CodingAdventures::PythonLexer->tokenize('def foo(x):');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
python.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::PythonLexer  ← you are here
    ↓  feeds
python_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `python.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
