# CodingAdventures::JavascriptLexer (Perl)

A grammar-driven JavaScript tokenizer. Reads the shared `javascript.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes JavaScript source into a flat list of typed tokens.

## What it does

Given `const x = 42;`, produces:

| type      | value   | line | col |
|-----------|---------|------|-----|
| CONST     | `const` | 1    | 1   |
| NAME      | `x`     | 1    | 7   |
| EQUALS    | `=`     | 1    | 9   |
| NUMBER    | `42`    | 1    | 11  |
| SEMICOLON | `;`     | 1    | 13  |
| EOF       |         | 1    | 14  |

Whitespace is consumed silently. The last token is always `EOF`.

## Token types

### Literals
| Token  | Example |
|--------|---------|
| NAME   | `myVar`, `$el`, `_priv` |
| NUMBER | `42`, `0` |
| STRING | `"hello"`, `"a\nb"` |

### Keywords
| Token      | Keyword      |
|------------|--------------|
| LET        | `let`        |
| CONST      | `const`      |
| VAR        | `var`        |
| IF         | `if`         |
| ELSE       | `else`       |
| WHILE      | `while`      |
| FOR        | `for`        |
| DO         | `do`         |
| FUNCTION   | `function`   |
| RETURN     | `return`     |
| CLASS      | `class`      |
| IMPORT     | `import`     |
| EXPORT     | `export`     |
| FROM       | `from`       |
| AS         | `as`         |
| NEW        | `new`        |
| THIS       | `this`       |
| TYPEOF     | `typeof`     |
| INSTANCEOF | `instanceof` |
| TRUE       | `true`       |
| FALSE      | `false`      |
| NULL       | `null`       |
| UNDEFINED  | `undefined`  |

### Operators
| Token              | Symbol |
|--------------------|--------|
| STRICT_EQUALS      | `===`  |
| STRICT_NOT_EQUALS  | `!==`  |
| EQUALS_EQUALS      | `==`   |
| NOT_EQUALS         | `!=`   |
| LESS_EQUALS        | `<=`   |
| GREATER_EQUALS     | `>=`   |
| ARROW              | `=>`   |
| EQUALS             | `=`    |
| PLUS               | `+`    |
| MINUS              | `-`    |
| STAR               | `*`    |
| SLASH              | `/`    |
| LESS_THAN          | `<`    |
| GREATER_THAN       | `>`    |
| BANG               | `!`    |

### Delimiters
| Token     | Symbol |
|-----------|--------|
| LPAREN    | `(`    |
| RPAREN    | `)`    |
| LBRACE    | `{`    |
| RBRACE    | `}`    |
| LBRACKET  | `[`    |
| RBRACKET  | `]`    |
| COMMA     | `,`    |
| COLON     | `:`    |
| SEMICOLON | `;`    |
| DOT       | `.`    |

## Usage

```perl
use CodingAdventures::JavascriptLexer;

my $tokens = CodingAdventures::JavascriptLexer->tokenize('const x = 1;');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
javascript.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::JavascriptLexer  ← you are here
    ↓  feeds
javascript_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `javascript.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
