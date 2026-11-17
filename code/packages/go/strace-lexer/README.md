# strace-lexer

A grammar-driven lexer for ONE LINE of `strace -f --quiet=attach -y -s 4096`
output. Kiln, the build system, reads its traces through this package and
`strace-parser` (D24, KILN04 section 4.9).

## How it fits in the stack

```
code/grammars/strace/strace.tokens          the tokens of a line
code/grammars/strace/strace-string.tokens   the escapes inside "..." and <...>
        |
   grammar-tools  (compile-tokens: grammar -> Go data, grammar_data.go)
        |
   lexer          (lexer.Compile: compile once, tokenize many times, no panic)
        |
   strace-lexer   (this package: compiled once at start-up, one line in, tokens out)
        |
   strace-parser  (structure of the line)
        |
   kiln internal/trace  (policy: pending calls, refusal reasons, limits)
```

The syntax is NOT written in Go. It is in the two `.tokens` files. This package
compiles their embedded data once and adds a per-line entry point and strict
escape decoding.

## Usage

```go
toks, err := stracelexer.TokenizeLine(`100  openat(AT_FDCWD</w>, "/etc/x", O_RDONLY) = 3</etc/x>`)
// PID "100  ", SYSCALL "openat(", ANNOTATED "AT_FDCWD</w>", COMMA, QUOTED "\"/etc/x\"",
// COMMA, WORD "O_RDONLY", RPAREN, PUNCT "=", ANNOTATED "3</etc/x>", EOF

if tok, _, bad := stracelexer.FirstError(toks); bad {
    // an ERR_* token: refuse the line; stracelexer.Ambiguous(tok) says whether
    // the old scanner called it an ambiguous path
}

body, _ := stracelexer.DecodeAnnotation(`a\<b\>`) // "a<b>"
```

`Token.Column` is the 1-based byte column, so the raw text of a run of tokens is
`line[first.Column-1 : last.Column-1+len(last.Value)]`.

## Properties (each pinned by a test)

- **Total.** Whatever no real token matches becomes an `ERR_*` token
  (`ERR_PID`, `ERR_QUOTE`, `ERR_ANNOT_BAD`, `ERR_ANNOT_OPEN`, `ERR_LT`,
  `ERR_RESUME`, `ERR_CTRL`, `ERR_ANY`), so `TokenizeLine` never fails on a line
  without a newline. It never panics. A newline in the line is `ErrNewline`, and a line over
  `DefaultMaxLineBytes` (1 MiB; `TokenizeLineMax` takes another) is `ErrLineTooLong`,
  checked before lexing.
- **Case-sensitive.** A case-insensitive grammar lexes a lower-cased copy of the
  source and re-reads some values from the original at the same offsets, which is
  unsafe on untrusted text.
- **Fast.** `SlowRules()` is empty: every rule is a literal or a deterministic
  matcher, never the regexp engine, so cost does not grow with the rest of the
  line. About 40 MB/s per line on the kiln fixtures.
- **No token name contains `STRING`** (the lexer would strip quote-looking bytes
  from its value). Quoted text is `QUOTED`.
- **No drift.** `TestEmbeddedGrammarsMatchFiles` re-parses the `.tokens` files and
  compares them with the embedded data.

## Regenerating the embedded data

After editing a `.tokens` file, from `code/packages/go`:

```
grammar-tools compile-tokens ../../grammars/strace/strace.tokens -o strace-lexer/grammar_data.go -p stracelexer
grammar-tools compile-tokens ../../grammars/strace/strace-string.tokens -o strace-lexer/string_grammar_data.go -p stracelexer
```

then rename the variable in `string_grammar_data.go` to `StringTokenGrammarData`
and run `gofmt -w` on both. (Build the tool with
`go build -o grammar-tools .` in `code/programs/go/grammar-tools`.)

## Token overview

| Token | Matches |
|---|---|
| `PID`, `PID_TAG` | `1234  ` and `[pid 1234] ` at the start of the line (1 to 9 digits, no leading zero) |
| `SYSCALL`, `FUNC` | a name glued to `(`: `openat(`, `WIFEXITED(` |
| `ANNOTATED` | a word glued to a `-y` annotation: `3</etc/passwd>` |
| `QUOTED` | `"..."` with escapes, optionally cut: `"abc"...` |
| `UNFINISHED`, `RESUMED` | `<unfinished ...>`, `<... wait4 resumed>` |
| `EXITED_LINE`, `KILLED_LINE`, `KILLED_CORE_LINE` | the whole `+++ ... +++` line |
| `SIG_OPEN`, `SIG_SP`, `SIG_TEXT` | a `--- ... ---` signal line (spaces are tokens) |
| `DEAD_OPEN`, `DEAD_WHOLE`, `DEAD_RESUMED` | the three exact spellings of a thread killed at syscall entry |
| `WORD`, `ASSIGN`, `PUNCT`, brackets, `COMMA` | everything else in an argument list |

See the comments in `code/grammars/strace/strace.tokens` for why each is ordered
as it is.
