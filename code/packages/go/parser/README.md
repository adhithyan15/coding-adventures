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
- **What the default limit means in practice.** Depth counts grammar rules, not tokens, and each successful step of a left-recursive rule (`e = e PLUS NUMBER`) counts too, so a chain of more than about 10,000 left-associative terms is refused. With the default, the `*-parser` wrappers refuse at roughly: JSON arrays about 5,000 nested levels; Python parentheses about 3,300; TypeScript call arguments about 500. Raise `Options.MaxDepth` (hard ceiling 50,000) via `NewGrammarParserWithOptions` if you really need more. The finished tree is bounded too: a successful parse returns a tree of at most `MaxDepth` levels (memo reuse and left-recursive growth are checked against the subtree's height). Element nesting inside rules also draws on a global frame budget (`HardMaxFrames`, 200,000 weighted frames), so the stack is bounded even at the ceiling; the ceiling of 50,000 is reachable only for rules with a body at most two elements deep, and for deeper rule bodies the frame budget undercuts a large `MaxDepth` (the error then names the frame budget).
- **Memory caps.** `Options.MaxTokens` and `Options.MaxMemoSlots` bound what an untrusted input can cost (`ErrMaxTokens`, `ErrMaxMemoSlots`); both are off by default.
- **Tree helpers are iterative** (`WalkAST`, `FindNodes`, `CollectTokens`), so they are safe on trees of any depth.
- **Compiled grammars are cached per `*ParserGrammar` pointer** by the `NewGrammarParser*` constructors. Each entry has a fingerprint (rule count, first-rule address, rule names, top-level body shapes), so adding, removing or renaming rules is noticed and recompiled; an in-place edit deep inside a rule body is not, so call `parser.ForgetGrammar(g)` after one, or better, do not mutate a grammar after use. The cache holds at most 512 grammars and is cleared and refilled when full.
- **EOF semantics.** Reading past the end of the token list yields a synthetic EOF, so `p = A B EOF` accepts `[A, B]`. An EOF token in the middle of the list ends the parse and the rest is ignored: consumers that cannot allow that must reject EOF before the last position. A zero-width repetition iteration stops the repetition but its children are kept.
- **No error recovery, by design.** A parse returns a complete AST for the whole input or an error and a nil tree. It never skips tokens, never inserts synthetic ones and never returns a partial tree, so callers can fail closed.
- **Cheap memoization.** The packrat cache is two flat slices (no hashing, no per-entry allocation) pooled across parses of one `Compiled`.

- **No wasted left-recursion work.** A compile-time analysis (`leftrec.go`) marks the rules that can reach themselves without consuming a token; only those run the seed-growing loop after a successful match. Everything else skips it, which roughly halves allocations and doubles throughput (about 1.9x on the JSON, expression and strace benchmarks on an M2 Max). The analysis over-approximates (EOF and empty literals count as zero-width, which a tight `MaxDepth` can observe), so a wrong answer can only cost speed, never change a result. Expected-token sets are integer ids; the "Expected A or B" text is built only on error.

- **Flat tree, `ASTNode`s only for the survivors.** While parsing, the engine builds the tree in pooled `int32` arrays (a scratch stack of token indexes and `^node` references, a per-parse `kids` array, a `[]cnode`, and memo slots that store node indexes). `Parse` then materialises `*ASTNode`s for the nodes reachable from the root, into exact-size slabs (ASTNode chunks of at most 256, one cap-clipped children slab, each token boxed once). Failed alternatives, superseded left-recursion seeds and discarded lookaheads cost a few integer writes instead of heap allocations: allocations per parse fell 2.8x to 7x (strace batch 205 to 73, 400-record JSON file 44,816 to 16,039, the TypeScript sample 15,473 to 2,224) and throughput rose 1.18x to 1.52x (JSON 1.4 to 1.5x, strace 1.2x, TypeScript 1.2x; M-series laptop, see the CHANGELOG for the table). No API change: trees are byte-identical, including the legacy quirk that a memo hit is a distinct node sharing the first parse's `Children` with zero positions. `Children` slices are capacity-clipped, so appending to one never overwrites a sibling.

### Compact API: `Tree` and `ParseInto`

If you only need to look at the tree (walk it once, pull out a few fields), skip the `*ASTNode`s entirely. `ParseInto` runs the same engine and leaves the flat result in a `Tree` you can reuse for every parse; in steady state it allocates nothing.

```go
var t parser.Tree // reuse across parses
if err := c.ParseInto(&t, tokens, parser.Options{}); err != nil {
    return err // t is empty (t.Root() == parser.NoNode)
}
var walk func(n parser.NodeID)
walk = func(n parser.NodeID) {
    fmt.Println(t.Rule(n))
    for i := 0; i < t.NumChildren(n); i++ {
        if ch := t.Child(n, i); ch.IsToken() {
            fmt.Println("  token", t.Token(ch.Token()).Value)
        } else {
            walk(ch.Node())
        }
    }
}
walk(t.Root())
ast := t.ToAST() // byte-identical to what c.Parse would have returned
```

Accessors: `Root`, `Rule`, `RuleID`, `NumChildren`, `Child` (a `Child` is a token index or a `NodeID`), `Token`, `Span` (first and last leaf token index), `Position` (the `StartLine`, `StartColumn`, `EndLine`, `EndColumn` an `ASTNode` would have), `IsReplay`; `Compiled.RuleID` and `RuleName` map names and ids.

Things to know:

- **Memo hits are kept visible.** As in `Parse`'s tree, a packrat replay is a distinct node (`IsReplay`) that shares the replayed node's children, with a zero `Position`; `Span` still reports its real token range.
- **Errors leave the Tree empty.** Every refusal and error is `Parse`'s, with the same text and cause; a partial result is never visible.
- **Ownership.** A Tree owns its arrays (nothing aliases the pooled parse state). It keeps a reference to the token slice you passed (after any `PreParse` hooks) without copying it, so do not modify the tokens while you use the Tree. A Tree is not safe for concurrent use; distinct Trees on one `Compiled` are.
- **Bounded reuse.** Between parses a Tree keeps at most 131,072 entries per buffer; a larger one is dropped at the next `ParseInto` or `Reset`.
- **Cost of positions.** The first `Span` or `Position` call after a parse does one O(`NumNodes`) pass and allocates 8 bytes per node (zero allocation applies to the parse plus structural walks: `Root`, `Rule`, `Child`, `Token`). Keep `NodeID`s only until the next `ParseInto` or `Reset`; a replay's identity is its slot (parent, child index), not its `NodeID`. Do not copy a `Tree`.
- **No `PostParse`.** Those hooks transform `*ASTNode`s, so `ParseInto` refuses them; call `ToAST` and apply them yourself.

Run `go test -bench . -benchmem` to see the `BenchmarkParse*` numbers (`BenchmarkParseStraceLike` uses the real strace grammar from `testdata/`).

### Ordered token alternatives

The compiled matcher checks a leading contiguous run of direct token
alternatives with one lookahead. It preserves the grammar's choice order,
newline skipping, legacy token-enum fallback, failed-prefix diagnostics, and
sticky resource-limit behavior. Ordinary `go test ./...` compares the bounded
name/type/newline domain against the frozen parser engine. The optional
test-only SAT miter uses fresh observations from that Go test and the existing
TypeScript solver; it does not add a Node dependency to this package:

```sh
npm ci --prefix code/packages/typescript/test-bootstrap
npm run build --prefix code/packages/typescript/test-bootstrap
node code/packages/go/parser/verify_alt_token_run_sat.mjs
```

The SAT model covers only its explicit 32-row token-input domain. It is a finite
semantic cross-check, not a proof of the Go source or all parser executions.
