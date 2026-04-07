# Dartmouth BASIC Lexer

## Purpose

This spec defines `dartmouth_basic_lexer` — the tokeniser for the 1964
Dartmouth BASIC language. It is a thin wrapper around the generic `lexer`
package, providing a `basic.tokens` grammar file and a module that caches and
delegates to it.

The lexer sits at Layer 1 of the Dartmouth BASIC pipeline:

```
BASIC source text
      │
      ▼
┌─────────────────────────────────┐
│   dartmouth_basic_lexer         │  ← this package
│   basic.tokens grammar          │
└─────────────────────────────────┘
      │
      ▼  [{type, value, line, column}, ...]
┌─────────────────────────────────┐
│   dartmouth_basic_parser        │
└─────────────────────────────────┘
```

The lexer knows nothing about statements, expressions, or program structure.
It only answers: "given this stream of characters, what are the tokens?"

---

## Depends On

| Package | Role |
|---------|------|
| `grammar_tools` | Parses `basic.tokens` into a `TokenGrammar` struct |
| `lexer` | Runs the `TokenGrammar` against source text, produces `[Token.t()]` |

---

## Grammar File: `code/grammars/basic.tokens`

The grammar file is the heart of this package. The `lexer` package reads it and
turns it into a matching engine at startup. Rules are matched in the order they
appear — first match wins.

### Directives

```
# @version 1
# @case_insensitive true
```

`@case_insensitive true` means the lexer normalises all input to uppercase before
matching. This means `print`, `Print`, and `PRINT` all produce the same `KEYWORD`
token with value `"PRINT"`. This was authentic to the original Dartmouth system,
which ran on teletypes that only had uppercase characters.

### Why Case-Insensitive Normalisation Is Applied to the Whole Source

An alternative would be to match keywords case-insensitively while preserving
the case of strings and identifiers. But the 1964 Dartmouth BASIC:

1. Had no lowercase letters — the GE-225's teletypes were uppercase-only
2. Had no string variables — strings only appear in PRINT and DATA, where
   case preservation is irrelevant for computation
3. Had single-letter variable names — `X` and `x` are the same

So whole-source uppercasing is the correct historical behaviour and simplifies
the grammar considerably.

### Token Definitions (in priority order)

```
# ============================================================
# SECTION 1: Multi-character operators (must come before single-char)
#
# The three two-character comparison operators must be matched before
# their component characters, otherwise `<=` would lex as `LT` `EQ`.
# ============================================================

LE          = "<="
GE          = ">="
NE          = "<>"

# ============================================================
# SECTION 2: Line numbers
#
# A line number is a sequence of digits that appears at the start of
# a physical line. It is syntactically distinct from a NUMBER because:
#
#   - In the grammar, `line := LINE_NUM statement NEWLINE`
#     The parser uses LINE_NUM to know it is looking at a new program line.
#   - In GOTO/GOSUB/IF...THEN, the target is also a LINE_NUM token,
#     not a numeric expression. `GOTO 100` means jump to line 100,
#     not compute the expression 100 and jump there.
#
# We distinguish LINE_NUM from NUMBER by position: LINE_NUM only appears
# at the very start of a source line (preceded by a newline or at col 1).
# We achieve this with the `preceded_by_newline` flag in the on_token
# callback (see Implementation Notes below).
#
# For the .tokens grammar, we define BOTH as the same regex and let
# the wrapper module disambiguate via an on_token callback that renames
# NUMBER tokens at column 1 / after newline to LINE_NUM.
#
# Alternatively, we emit LINE_NUM first in the grammar and rely on
# the `preceded_by_newline` context. The simpler approach for a data-driven
# grammar: define LINE_NUM as a distinct pattern that the post-tokenize
# hook renames. We use the on_token approach described below.
# ============================================================

LINE_NUM    = /[0-9]+/

# ============================================================
# SECTION 3: Numeric literals
#
# BASIC 1964 supports:
#   42        integer-looking but stored as float
#   3.14      decimal
#   .5        leading dot
#   1.5E3     scientific notation (1500.0)
#   1.5E-3    negative exponent (0.0015)
#   1E10      no decimal part, scientific
#
# The regex matches all of these in one rule.
# ============================================================

NUMBER      = /[0-9]*\.?[0-9]+([Ee][+-]?[0-9]+)?/

# ============================================================
# SECTION 4: String literals
#
# In 1964 BASIC, strings appear only in:
#   PRINT "HELLO WORLD"
#   DATA "text" (rare in original)
#
# Strings are delimited by double quotes. The original 1964 spec does
# not support escape sequences inside strings — a double quote cannot
# appear inside a string literal. We follow this restriction.
#
# The alias `-> STRING` means the token type emitted is STRING, not
# STRING_BODY. The grammar tool uses the alias for what the parser sees.
# ============================================================

STRING_BODY = /"[^"]*"/ -> STRING

# ============================================================
# SECTION 5: Built-in mathematical functions
#
# These are the 11 built-in functions from the 1964 spec:
#   SIN, COS, TAN, ATN  — trigonometric
#   EXP                  — e^x
#   LOG                  — natural logarithm
#   ABS                  — absolute value
#   SQR                  — square root
#   INT                  — floor to integer
#   RND                  — random number in [0,1)
#   SGN                  — sign: -1, 0, or 1
#
# Additionally, user-defined functions use the form FNA, FNB, ..., FNZ.
# FN followed by exactly one uppercase letter.
#
# Because @case_insensitive is true, `sin` becomes `SIN` before matching.
# These must appear BEFORE the IDENT rule so they are not mistakenly
# tokenised as variable names.
# ============================================================

BUILTIN_FN  = /SIN|COS|TAN|ATN|EXP|LOG|ABS|SQR|INT|RND|SGN/
USER_FN     = /FN[A-Z]/

# ============================================================
# SECTION 6: Keywords
#
# All reserved words of Dartmouth BASIC 1964. They must appear
# BEFORE IDENT so that PRINT is not tokenised as an identifier.
#
# We use the keywords: section so they are emitted as KEYWORD tokens.
# ============================================================

NAME        = /[A-Z][0-9]?/

keywords:
  LET
  PRINT
  INPUT
  IF
  THEN
  GOTO
  GOSUB
  RETURN
  FOR
  TO
  STEP
  NEXT
  END
  STOP
  REM
  READ
  DATA
  RESTORE
  DIM
  DEF

# ============================================================
# SECTION 7: Single-character operators and punctuation
# ============================================================

PLUS        = "+"
MINUS       = "-"
STAR        = "*"
SLASH       = "/"
CARET       = "^"
EQ          = "="
LT          = "<"
GT          = ">"
LPAREN      = "("
RPAREN      = ")"
COMMA       = ","
SEMICOLON   = ";"

# ============================================================
# SECTION 8: Newlines (significant — kept in token stream)
#
# BASIC is line-oriented. Each statement ends at the newline.
# Newlines are NOT in the skip: section — the parser needs them
# to know where statements end.
#
# We match \r\n (Windows teletypes) and \n (Unix) both.
# ============================================================

NEWLINE     = /\r?\n/

# ============================================================
# SECTION 9: Skip patterns (silently consumed)
#
# Horizontal whitespace (spaces and tabs) between tokens is ignored.
# Vertical whitespace (newlines) is kept — see section 8.
# ============================================================

skip:
  WHITESPACE  = /[ \t]+/

# ============================================================
# SECTION 10: REM handling
#
# REM (remark) consumes everything up to the end of the line.
# This is handled via an on_token callback: when a KEYWORD token
# with value "REM" is seen, suppress all subsequent tokens until
# NEWLINE. See Implementation Notes.
#
# We do NOT define a REM_TEXT token in the grammar because the
# grammar_tools lexer does not have a "consume until newline" mode.
# The callback approach is cleaner.
# ============================================================

# ============================================================
# SECTION 11: Error recovery
#
# If none of the above match, capture the bad character as an
# UNKNOWN token so the lexer can report a meaningful error rather
# than looping forever.
# ============================================================

errors:
  UNKNOWN = /./
```

---

## The LINE_NUM Disambiguation

The trickiest part of BASIC lexing is that bare integers serve two purposes:

1. **Line labels**: `10 LET X = 5` — the `10` names the line
2. **GOTO targets**: `GOTO 30` — the `30` is the destination
3. **Numeric literals**: `LET X = 42` — the `42` is a value

In the grammar file, `LINE_NUM` and `NUMBER` use the same regex. The wrapper
module disambiguates with a **post-tokenize hook** that re-labels the first
`NUMBER` token on each line as `LINE_NUM`:

```
Algorithm:
  Walk the token list.
  A token at position 0 OR immediately after a NEWLINE token:
    if type == "NUMBER" → relabel to "LINE_NUM"
```

This is simpler than an `on_token` callback because it only needs to look at
the completed list, not make decisions mid-stream.

```elixir
# post_tokenize_hook implementation:
def relabel_line_numbers(tokens) do
  tokens
  |> Enum.reduce({[], :at_line_start}, fn token, {acc, state} ->
    case {state, token.type} do
      {:at_line_start, "NUMBER"} ->
        # This number is in line-number position — relabel it
        {[%{token | type: "LINE_NUM"} | acc], :in_line}

      {:at_line_start, _} ->
        # Non-number at line start (e.g., bare NEWLINE line)
        {[token | acc], :in_line}

      {:in_line, "NEWLINE"} ->
        # End of statement — next token begins a new line
        {[token | acc], :at_line_start}

      {:in_line, _} ->
        {[token | acc], :in_line}
    end
  end)
  |> then(fn {acc, _} -> Enum.reverse(acc) end)
end
```

---

## The REM Handling

`REM` introduces a comment that runs to the end of the line. Everything after
`REM` on the same line is a remark and should be discarded. The post-tokenize
hook handles this:

```
Algorithm:
  Walk the token list.
  When a KEYWORD "REM" token is encountered:
    suppress all subsequent tokens until (and not including) the next NEWLINE.
```

This means `10 REM THIS IS A COMMENT` tokenises as:
```
LINE_NUM("10"), KEYWORD("REM"), NEWLINE
```
The comment text never appears in the output.

---

## Token Reference

The final token stream uses these types:

| Type | Value | Notes |
|------|-------|-------|
| `LINE_NUM` | `"10"`, `"999"` | Digits only; always appears first on a line |
| `NUMBER` | `"3.14"`, `"42"`, `"1.5E3"` | Any numeric literal in expressions |
| `STRING` | `"\"HELLO\""` | Includes the double quotes |
| `KEYWORD` | `"PRINT"`, `"LET"`, `"IF"` | Always uppercase (case_insensitive normalised) |
| `BUILTIN_FN` | `"SIN"`, `"LOG"`, `"RND"` | One of the 11 built-ins |
| `USER_FN` | `"FNA"`, `"FNZ"` | FN followed by one letter |
| `NAME` | `"X"`, `"A1"`, `"B9"` | Variable names: one letter + optional digit |
| `PLUS` | `"+"` | |
| `MINUS` | `"-"` | |
| `STAR` | `"*"` | |
| `SLASH` | `"/"` | |
| `CARET` | `"^"` | Exponentiation |
| `EQ` | `"="` | Assignment and equality (context-determined by parser) |
| `LT` | `"<"` | |
| `GT` | `">"` | |
| `LE` | `"<="` | |
| `GE` | `">="` | |
| `NE` | `"<>"` | Not-equal |
| `LPAREN` | `"("` | |
| `RPAREN` | `")"` | |
| `COMMA` | `","` | |
| `SEMICOLON` | `";"` | Print separator: no space between items |
| `NEWLINE` | `"\n"` or `"\r\n"` | Statement terminator |
| `EOF` | `""` | Always appended by the lexer |

---

## Public API

```elixir
# Tokenise a Dartmouth BASIC source string.
# Returns the full token list including EOF.
# NEWLINE tokens are included (they are significant).
CodingAdventures.DartmouthBasicLexer.tokenize(source :: String.t()) ::
  {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
```

---

## Package Structure

```
code/packages/elixir/dartmouth_basic_lexer/
├── BUILD
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── mix.exs
├── lib/
│   └── coding_adventures/
│       └── dartmouth_basic_lexer.ex    ← main module
└── test/
    └── dartmouth_basic_lexer_test.exs
```

The grammar file lives at the repo level so all language implementations can
share it:

```
code/grammars/
└── basic.tokens    ← shared grammar file
```

---

## `mix.exs` Dependencies

```elixir
defp deps do
  [
    {:coding_adventures_grammar_tools, path: "../grammar_tools"},
    {:coding_adventures_lexer, path: "../lexer"},
  ]
end
```

---

## Implementation Notes

### Grammar Loading

The grammar is loaded from disk once at startup and cached in `:persistent_term`.
Subsequent calls to `tokenize/1` use the cached grammar with zero disk I/O:

```elixir
defp get_grammar do
  case :persistent_term.get({__MODULE__, :grammar}, nil) do
    nil ->
      grammar = load_grammar()
      :persistent_term.put({__MODULE__, :grammar}, grammar)
      grammar
    grammar ->
      grammar
  end
end

defp load_grammar do
  path = Path.join([@grammars_dir, "basic.tokens"])
  {:ok, grammar} = TokenGrammar.parse(File.read!(path))
  grammar
end
```

### Post-Tokenize Hook Pipeline

Both transformations (LINE_NUM disambiguation and REM suppression) are applied
as post-tokenize hooks in order:

```elixir
def tokenize(source) do
  grammar = get_grammar()
  GrammarLexer.tokenize(source, grammar,
    post_tokenize_hooks: [
      &relabel_line_numbers/1,
      &suppress_rem_content/1,
    ]
  )
end
```

---

## Grammar File Path Resolution

```elixir
@grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
              |> Path.expand()
```

From `lib/coding_adventures/dartmouth_basic_lexer.ex`, five `..` steps reach
`code/grammars/`.

---

## Test Strategy

Tests are written against the public `tokenize/1` function. They never import
or call the grammar loading internals directly.

### Happy Path Tests

```
"10 LET X = 5"
→ [LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE, EOF]

"20 PRINT X, Y"
→ [LINE_NUM("20"), KEYWORD("PRINT"), NAME("X"), COMMA, NAME("Y"), NEWLINE, EOF]

"30 GOTO 10"
→ [LINE_NUM("30"), KEYWORD("GOTO"), NUMBER("10"), NEWLINE, EOF]
   # Note: GOTO target is NUMBER here — parser will validate it is an integer

"40 IF X > 0 THEN 100"
→ [LINE_NUM("40"), KEYWORD("IF"), NAME("X"), GT, NUMBER("0"),
   KEYWORD("THEN"), NUMBER("100"), NEWLINE, EOF]

"50 FOR I = 1 TO 10 STEP 2"
→ [LINE_NUM("50"), KEYWORD("FOR"), NAME("I"), EQ, NUMBER("1"),
   KEYWORD("TO"), NUMBER("10"), KEYWORD("STEP"), NUMBER("2"), NEWLINE, EOF]

"60 DEF FNA(X) = X * X"
→ [LINE_NUM("60"), KEYWORD("DEF"), USER_FN("FNA"), LPAREN, NAME("X"), RPAREN,
   EQ, NAME("X"), STAR, NAME("X"), NEWLINE, EOF]

"70 LET Y = SIN(X) + COS(X)"
→ [LINE_NUM("70"), KEYWORD("LET"), NAME("Y"), EQ,
   BUILTIN_FN("SIN"), LPAREN, NAME("X"), RPAREN,
   PLUS, BUILTIN_FN("COS"), LPAREN, NAME("X"), RPAREN, NEWLINE, EOF]
```

### Case Insensitivity Tests

```
"10 print x"   → same tokens as "10 PRINT X"
"20 Let A = 1" → same tokens as "20 LET A = 1"
"30 goto 20"   → same tokens as "30 GOTO 20"
```

### Operator Tests

```
"10 IF X <= Y THEN 50"  → LE token, not LT then EQ
"10 IF X >= Y THEN 50"  → GE token, not GT then EQ
"10 IF X <> Y THEN 50"  → NE token, not LT then GT
```

### Number Format Tests

```
"10 LET X = 3.14"    → NUMBER("3.14")
"10 LET X = .5"      → NUMBER(".5")
"10 LET X = 1.5E3"   → NUMBER("1.5E3")
"10 LET X = 1.5E-3"  → NUMBER("1.5E-3")
"10 LET X = 1E10"    → NUMBER("1E10")
```

### String Tests

```
"10 PRINT \"HELLO WORLD\""
→ [LINE_NUM("10"), KEYWORD("PRINT"), STRING("\"HELLO WORLD\""), NEWLINE, EOF]
```

### REM Tests

```
"10 REM THIS IS A COMMENT"
→ [LINE_NUM("10"), KEYWORD("REM"), NEWLINE, EOF]
# Comment text is suppressed

"10 REM\n20 LET X = 1"
→ [LINE_NUM("10"), KEYWORD("REM"), NEWLINE,
   LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE, EOF]
```

### Multi-Line Program Test

```
"10 LET X = 1\n20 PRINT X\n30 END"
→ [LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE,
   LINE_NUM("20"), KEYWORD("PRINT"), NAME("X"), NEWLINE,
   LINE_NUM("30"), KEYWORD("END"), NEWLINE, EOF]
```

### Variable Name Tests

```
"10 LET X = 1"    → NAME("X")   single letter
"10 LET A1 = 2"   → NAME("A1")  letter + digit
"10 LET Z9 = 3"   → NAME("Z9")  letter + digit
```

### PRINT Separator Tests

```
"10 PRINT X, Y"   → COMMA  (space between items)
"10 PRINT X; Y"   → SEMICOLON  (no space between items)
```

### Error Recovery Tests

```
"10 LET @ = 1"
→ [LINE_NUM("10"), KEYWORD("LET"), UNKNOWN("@"), EQ, NUMBER("1"), NEWLINE, EOF]
# Lexer recovers and continues; parser will error on UNKNOWN
```

### Coverage Target: ≥ 95%

Every token type must appear in at least one test. Every post-tokenize hook
path must be exercised.

---

## What This Package Does NOT Do

- **Validate semantics** — `10 GOTO 999` is fine lexically even if line 999
  doesn't exist. That's the compiler's or VM's job.
- **Parse expressions** — the precedence of `+` vs `*` is the parser's concern.
- **Distinguish assignment `=` from comparison `=`** — both lex as `EQ`. The
  parser uses context (LET vs IF) to interpret them.
- **Handle line number ordering** — `20 ... \n 10 ...` is valid input. The
  compiler sorts lines by number; the lexer just tokenises in order.
