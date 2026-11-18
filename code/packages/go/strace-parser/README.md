# strace-parser

A grammar-driven parser for ONE LINE of `strace -f --quiet=attach -y -s 4096`
output: a line in, a syntax tree out, syntax only. Kiln reads its traces through
this package and `strace-lexer` (D24, KILN04 section 4.9).

## How it fits in the stack

```
code/grammars/strace/strace.grammar    the structure of a line
        |
   grammar-tools   (compile-grammar: grammar -> Go data, grammar_data.go)
        |
   parser          (CompileGrammar: compile once, parse many times; MaxDepth, MaxTokens)
        |
   strace-parser   (this package, over strace-lexer's tokens)
        |
   kiln internal/trace  (policy: pending calls, refusal reasons, interpreters)
```

## Usage

```go
ast, toks, err := straceparser.ParseLineTokens(line, straceparser.DefaultOptions())
if err != nil {
    var le *straceparser.LineError
    errors.As(err, &le)
    // le.Kind: KindSyntax, KindErrToken, KindBadAnnotation, KindSpace,
    //          KindNewline, KindLimit, KindBadQuoted.  le.Ambiguous() says "ambiguous path".
}
switch straceparser.BodyKind(ast) {
case "call_line":     // name(args) = result, or name(args <unfinished ...>
    args := straceparser.RawArgs(line, ast)          // raw text of each top-level argument
    rest := straceparser.TextAfterClose(line, ast)   // " = 3</etc/passwd>"
case "resumed_line":  // <... name resumed>rest-of-args) = result
case "exit_line":     // +++ exited with N +++, +++ killed by SIG... +++
case "signal_line":   // --- SIGCHLD {...} ---
case "dead_line":     // ???( <unfinished ...>, ???()  = ?   (exact spellings only)
case "dead_resumed_line": // <... ??? resumed>)  = ?   (exact)
}
```

The tree stops at arguments: an argument is a run of tokens and the consumer
interprets its raw text. Arguments, results and strings are not decoded here.

## Fail closed, with limits

There is no error recovery: a line parses completely or is an error. Every zero limit is filled from `DefaultOptions()` field by field (`Options` embeds
`parser.Options` and adds `MaxLineBytes`, 1 MiB, checked before lexing, and
`LenientQuoted`). Pass
explicit limits when you want tighter ones. `DefaultOptions()` is the recommended set: bracket nesting 64
(`MaxDepth` counts rule levels, 2 per bracket level plus 8), 65,536 tokens per
line, and a memo bound of rules times tokens. The zero `parser.Options` means no
token or memo limit. `ParseLine` with a zero `Options` takes `DefaultOptions()`.

Besides the grammar the package enforces what a grammar cannot see: no newline in
the line, no leading or trailing space (the lexer skips spaces), every `-y`
annotation and every quoted string decodes strictly (a decoded string is arbitrary
bytes: a path consumer must still reject NUL and invalid UTF-8), and any `ERR_*` token or bad annotation refuses the
line before the parse (so hostile input costs one pass).

## Compact path: `LineParser`

`ParseLineTokens` builds a `*parser.ASTNode` per rule match. A consumer that only
asks "which shape is this line and where are its arguments" can use a reusable
`LineParser` instead, which keeps the parse in the shared parser's flat
`parser.Tree` and walks it by `NodeID`. It accepts and refuses exactly what
`ParseLineTokens` does (same `*LineError`, same text), except that
`Options.PostParse` hooks (which transform `*ASTNode`s) are refused; the ASTNode
API is unchanged. `Tree()` is read-only (never call `Reset` or `ParseInto` on
it), `Tokens()` must not be mutated, and the last line, tokens and tree stay
pinned (up to `MaxLineBytes`) until the next `ParseLineInto` or `Reset`.

```go
var lp straceparser.LineParser // zero value is ready; one per goroutine
buf := make([]string, 0, 16)
for _, line := range lines {
    if err := lp.ParseLineInto(line, straceparser.DefaultOptions()); err != nil {
        continue // a *LineError, as ParseLineTokens returns
    }
    switch lp.BodyKind() {
    case "call_line":
        buf = lp.AppendRawArgs(buf[:0])   // substrings of line
        rest := lp.TextAfterClose()
        if prefix, ok := lp.PrefixBeforeUnfinished(); ok { _ = prefix }
    }
}
```

A `LineParser` is **not safe for concurrent use** and must not be copied.
`Tree()`, `Tokens()` and `Body()` are valid until the next `ParseLineInto` or
`Reset`; the strings from `RawArgs`, `AppendRawArgs`, `TextAfterClose` and
`PrefixBeforeUnfinished` are substrings of the line you passed, so they outlive
the next parse. The tree's arrays are reused but the parser package drops an
oversized one at the next parse, so one huge line does not pin memory. For
hook-free compact parses, the line parser also reuses its private token buffer
and lexer cursor; it clears occupied entries on reset and drops token-buffer
capacities above 4096 tokens. Calls with `PreParse` hooks use the owning lexer
path so retained hook inputs are not mutated. Token slices remain borrowed
until the next parse/reset. The
existing fixture throughput figures are historical, and this buffer change
alone does not establish a Linux throughput improvement. See KILN04 for the
measurement contract and the unchanged Linux acceptance threshold.

## What is pinned by tests

- the embedded grammar equals `strace.grammar` (drift test);
- the grammar has no literal except the one justified `"---"`, and never
  accepts an `ERR_*` token;
- every line of kiln's four fixture traces parses, and the lines of kiln's tests
  that the grammar refuses are pinned in `testdata/corpus_refused.txt`;
- the nesting depth limit is exactly 64 bracket levels;
- the stricter-only divergences from the old scanner (see KILN04 section 4.9);
- a fuzz target (`FuzzParseLine`), and a benchmark of lex plus parse on the
  fixtures.

## Regenerating the embedded data

After editing `strace.grammar`, from `code/packages/go`:

```
grammar-tools compile-grammar ../../grammars/strace/strace.grammar -o strace-parser/grammar_data.go -p straceparser
gofmt -w strace-parser/grammar_data.go
```
