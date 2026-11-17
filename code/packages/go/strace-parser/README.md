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
    //          KindNewline, KindLimit.  le.Ambiguous() says "ambiguous path".
}
switch straceparser.BodyKind(ast) {
case "call_line":     // name(args) = result, or name(args <unfinished ...>
    args := straceparser.RawArgs(line, ast)          // raw text of each top-level argument
    rest := straceparser.TextAfterClose(line, ast)   // " = 3</etc/passwd>"
case "resumed_line":  // <... name resumed>rest-of-args) = result
case "exit_line":     // +++ exited with N +++, +++ killed by SIG... +++
case "signal_line":   // --- SIGCHLD {...} ---
case "dead_line":     // ???( <unfinished ...>, ???()  = ?
}
```

The tree stops at arguments: an argument is a run of tokens and the consumer
interprets its raw text. Arguments, results and strings are not decoded here.

## Fail closed, with limits

There is no error recovery: a line parses completely or is an error. Pass
explicit limits. `DefaultOptions()` is the recommended set: bracket nesting 64
(`MaxDepth` counts rule levels, 2 per bracket level plus 8), 65,536 tokens per
line, and a memo bound of rules times tokens. The zero `parser.Options` means no
token or memo limit. `ParseLine` with a zero `Options` takes `DefaultOptions()`.

Besides the grammar the package enforces what a grammar cannot see: no newline in
the line, no leading or trailing space (the lexer skips spaces), every `-y`
annotation decodes strictly, and any `ERR_*` token or bad annotation refuses the
line before the parse (so hostile input costs one pass).

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
