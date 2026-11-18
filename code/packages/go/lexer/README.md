# Lexer (Go Port)

**Layer 2 of the computing stack** — Deconstructs raw strings cleanly into mathematical identifiers without structural formatting constraints.

## Overview
A lexer consumes `runes` incrementally parsing definitions identifying whether specific groups translate onto integers, assignment markers natively mapping exactly character-limits accurately.
Extends Python limits defining mappings linearly removing complex REGEXP boundaries effectively replacing dynamically tracking native constants securely preserving context errors seamlessly over recursive loops cleanly inside `packages/go/lexer`.

## Grammar-driven lexing: compile once, tokenize many times

`NewGrammarLexer(source, grammar).Tokenize()` interprets a `TokenGrammar` (parsed
from a `.tokens` file). It rebuilds the grammar's matchers every time and panics
on bad input. For repeated use, or wherever a panic is unacceptable (a server, a
security-sensitive parser), use `Compile`:

```go
tg, err := grammartools.ParseTokenGrammar(tokensSource)
if err != nil { /* ... */ }

compiled, err := lexer.Compile(tg) // once; bad patterns are errors, not panics
if err != nil { /* ... */ }

tokens, err := compiled.Tokenize(line) // many times, from many goroutines
scratch := make([]lexer.Token, 0, 64)
tokens, err = compiled.TokenizeInto(line, scratch) // borrowed caller storage
var lexErr *lexer.LexError
if errors.As(err, &lexErr) {
    fmt.Println(lexErr.Line, lexErr.Column, lexErr.Message)
}
```

`Compiled.Tokenize` yields exactly the tokens `NewGrammarLexer(...).Tokenize()`
yields (types, values, lines, columns, flags) for every input the old path
accepts, and returns a `*LexError` where the old path panics. It never panics and
contains no `recover()`. A `*Compiled` is immutable and safe for concurrent use.

`Compiled.TokenizeInto` is an opt-in standard-mode caller-buffer API. Its
returned tokens borrow the returned slice storage until the caller reuses it.
The currently occupied input elements are cleared before reuse. If the caller
passes a shortened prefix, it remains responsible for any truncated tail. On a
lexical error the method returns nil and clears partial output, including
written entries in the original scratch if append grew. Indentation and layout
grammars use the existing owned-output path without changing `dst`.
`Tokenize` remains the allocation-owning API and earlier results are never
changed by later calls.

It is also fast, without changing what the grammar means (rules are still tried
in order, first match wins):

- **first-byte dispatch**: each rule's possible first bytes are computed once, so
  only rules that can start with the next byte are tried;
- **literal fast path**: literal rules (and regexes that are really plain
  strings) match with `strings.HasPrefix`;
- **deterministic matchers**: common shapes (identifiers, numbers, quoted strings
  with escapes, `name<annotation>`, whitespace, line comments) run as byte loops
  that are proven equivalent to the regexp engine; anything else uses `regexp`.

Measured on an Apple M2 Max (`go test -bench Tokenize`), strace-like lines
went from 0.23 MB/s (per line) and 1.26 MB/s (whole document) to about 46 and
39 MB/s, and a JSON document from 2.09 to about 32 MB/s.

`compiled.SlowRules()` lists the rules that still run on the regexp engine; a
caller that depends on the throughput can assert it is empty for its grammar.
`NewGrammarLexer` reuses a cached `*Compiled` per grammar pointer (fingerprint
checked, so an edited grammar is recompiled; `ForgetGrammar(tg)` releases it).

Things to know:

- A grammar with `CaseSensitive == false` (the zero value of a hand-built
  `TokenGrammar`) lowercases the source. `ParseTokenGrammar` defaults to true.
  Use a case-sensitive grammar for untrusted input.

- Tokens whose rule name (or alias) contains `STRING` and whose text starts with
  a quote have the quotes stripped (and escapes processed unless `escapes: none`).
  This is a legacy naming convention, kept for compatibility. Name a token
  `QUOTED` rather than `QUOTED_STRING` if you do not want it.
- With no `skip:` section, space, tab and carriage return are skipped.
- A rule that matches the empty string is reported as an error (the legacy
  lexer looped forever).
- `NewGrammarLexer`, `SetOnToken`, `AddPreTokenize` and `AddPostTokenize` keep
  working as before and share the compiled engine. The grammar's `errors:`
  section is not honoured in Go.

## Testing

`compiled_diff_test.go` runs the compiled engine and a frozen copy of the
previous engine (`legacy_oracle_test.go`) over real grammars in `code/grammars`,
every prefix of the samples, random mutations, and fuzz inputs
(`go test -fuzz FuzzCompiledMatchesLegacy`). `matcher_test.go` checks the
deterministic matchers and first-byte sets against `regexp`
(`go test -fuzz FuzzMatcherMatchesRegexp`).
