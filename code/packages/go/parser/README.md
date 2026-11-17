# Parser (Go Port)

**Layer 3 of the computing stack** — Maps token bounds explicitly translating them onto tree AST structures.

## Overview
Parsers convert sequence identifiers resolving syntax patterns incrementally avoiding context misinterpretations. Our parsing implementation explicitly evaluates loops bounding `recursive-descent` formats inside structured `Expression`, `Term`, and `Factor` LL(2) bounds natively evaluating operator precedence correctly natively inside `packages/go/parser` bypassing logic holes.

## Grammar-driven parser (`GrammarParser`, `Compiled`)

`grammar_parser.go` and `compiled.go` interpret a `.grammar` file's rules at runtime with packrat memoization and left-recursion support. Every `*-parser` package in the repo is a thin wrapper over it.

```go
// One-shot (unchanged API):
ast, err := parser.NewGrammarParser(tokens, grammar).Parse()

// Many inputs against one grammar: compile once, reuse (goroutine-safe).
c, err := parser.CompileGrammar(grammar)
ast, err := c.Parse(tokens, parser.Options{
    StartRule: "line",   // any named rule; default is the first rule
    MaxDepth:  2000,     // default 10,000 rule levels, hard ceiling 50,000
})
if errors.Is(err, parser.ErrMaxDepth) { /* hostile or absurdly nested input */ }
```

### Safety properties

- **Bounded recursion.** Nesting deeper than `MaxDepth` returns `ErrMaxDepth` (wrapped in `*GrammarParseError`). It can never overflow the goroutine stack, which in Go is a fatal, unrecoverable process crash. The limit applies to every caller, including ones that never heard of `Options`. A refusal is never swallowed by backtracking: depth exceeded always means error.
- **What the default limit means in practice.** Depth counts grammar rules, not tokens, and each successful step of a left-recursive rule (`e = e PLUS NUMBER`) counts too, so a chain of more than about 10,000 left-associative terms is refused. With the default, the `*-parser` wrappers refuse at roughly: JSON arrays about 5,000 nested levels; Python parentheses about 3,300; TypeScript call arguments about 500. Raise `Options.MaxDepth` (hard ceiling 50,000) via `NewGrammarParserWithOptions` if you really need more. Element nesting inside rules also draws on a global frame budget (`HardMaxFrames`), so the stack is bounded even at the ceiling.
- **Memory caps.** `Options.MaxTokens` and `Options.MaxMemoSlots` bound what an untrusted input can cost (`ErrMaxTokens`, `ErrMaxMemoSlots`); both are off by default.
- **Tree helpers are iterative** (`WalkAST`, `FindNodes`, `CollectTokens`), so they are safe on trees of any depth.
- **Compiled grammars are cached per `*ParserGrammar` pointer** by the `NewGrammarParser*` constructors. A grammar must not be mutated after first use; call `parser.ForgetGrammar(g)` if you do.
- **EOF semantics.** Reading past the end of the token list yields a synthetic EOF, so `p = A B EOF` accepts `[A, B]`. An EOF token in the middle of the list ends the parse and the rest is ignored: consumers that cannot allow that must reject EOF before the last position. A zero-width repetition iteration stops the repetition but its children are kept.
- **No error recovery, by design.** A parse returns a complete AST for the whole input or an error and a nil tree. It never skips tokens, never inserts synthetic ones and never returns a partial tree, so callers can fail closed.
- **Cheap memoization.** The packrat cache is two flat slices (no hashing, no per-entry allocation) pooled across parses of one `Compiled`.

Run `go test -bench . -benchmem` to see the `BenchmarkParse*` numbers.
