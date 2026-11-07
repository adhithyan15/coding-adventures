# warrant-grammar

Canonical Warrant v0 grammar data for [VER04 P006a](../../../specs/VER04-warrant-language.md#11-grammar-artifacts-and-syntax-corpus-staging-p006).
The existing grammar-tools compiler turns the repository's `.tokens` and
`.grammar` files into native Rust structures committed in `src/`.

```rust
use coding_adventures_warrant_grammar::{parser_grammar, token_grammar};
let tokens = token_grammar();
let rules = parser_grammar();
assert!(!tokens.definitions.is_empty());
assert!(!rules.rules.is_empty());
```

The additive [VER04a source stage](../../../specs/VER04a-warrant-source-admission.md)
also admits bytes, normalizes CRLF and maps normalized ranges to original UTF-8
bytes. LF input borrows its original buffer. Other input must fit the 1-MiB
source, 64-MiB owned-buffer and 3-MiB scan-work ceilings; callers may lower caps.
Malformed UTF-8, leading BOM, NUL and bare CR reject with fixed `W0001` errors.
Document Debug omits source. Range coordinates are one-based line/byte-column,
with zero-based raw byte offsets. Paths/revision binding remain a later layer.

```rust
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
let source = SourceDocument::admit(b"warrant x\r\n", SourceLimits::default()).unwrap();
assert_eq!(source.normalized(), "warrant x\n");
let newline = source.original_span(9, 10).unwrap();
assert_eq!(&source.original()[newline.start.byte_offset..newline.end.byte_offset], "\r\n");
```

The additive [VER04b layout stage](../../../specs/VER04b-warrant-layout-capture.md)
captures significant-layout tokens (`INDENT`, `DEDENT`, `NEWLINE`) over admitted
source documents. Synthetic layout tokens possess exact zero-width origins, mapping
via `doc.original_span(at, at)` to zero-width original source intervals without
fictitious positive-width whitespace spans:

```rust
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
let tokens = capture_layout(&source, LayoutLimits::default()).unwrap();
```

The additive [VER04c parser stage](../../../specs/VER04c-guarded-warrant-parser.md)
constructs a typed, bounded Abstract Syntax Tree (AST) conforming to `warrant.grammar`.
It operates directly on the `&[LayoutToken]` stream and enforces strict quotas:
maximum call-stack recursion depth (`max_depth: 64`, ceiling `128`) preventing CWE-674,
maximum AST node budget (`max_nodes: 65,536`, ceiling `1,048,576`), and work limits.
All diagnostics return closed `W0001` codes and never leak raw token text.

```rust
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
let ast = parse_document(&tokens, &source, ParserLimits::default()).unwrap();
```

The additive [VER04d type-checking and binding stage](../../../specs/VER04d-warrant-type-system.md)
performs semantic analysis, scope resolution, cycle detection, static type checking,
and host-language binding schema verification. It enforces strict subtyping (`Nat <: Int`,
`Bytes<N> <: Bytes`), exhaustive pattern matching, operation contracts (`labels`, `bound`,
`input`, `assume`), and stable diagnostics (`W0201`..`W0602`):

```rust
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::binding::{validate_bindings, BindingManifest};

let checked = check_warrant(&ast).unwrap();
validate_bindings(&checked, &manifest).unwrap();
```

Source admission, layout capture, grammar data, parser and type checking do not
establish solvers, Rust test generation, proof or release authorization (VER-P008+).

## Tests and regeneration

From the Rust workspace:

```sh
cargo test -p coding_adventures_warrant_grammar
cargo clippy -p coding_adventures_warrant_grammar --all-targets -- -D warnings
cargo run -p coding_adventures_warrant_grammar --example regenerate -- --check
```

Omit `-- --check` to regenerate the two embedded modules after editing the
canonical files in `code/grammars/warrant`. Do not format/edit generated files;
their modules are excluded from rustfmt and tests compare exact emitter bytes.

The authored [syntax corpus](../../../specs/fixtures/warrant-syntax-v1/README.md)
is projected from JSON to LF-only TFV by the standard-library Python gate.
Runtime tests embed TFV rather than parse JSON. They compare syntax verdicts,
whole subtree witnesses and runtime-loaded/compiled AST equality on 57 cases.
Three adapter controls reject forced acceptance, forgotten witnesses and a
wrong-precedence grammar. The latter still parses its source but loses the
independently authored `product=2 * 3` subtree. A separate unit test proves this
distinction and a shortened witness must fail exact membership.

```sh
cargo test -p coding_adventures_warrant_grammar --test self_corpus -- --sabotage always-pass
```

This command must exit nonzero with three failing controls. BUILD checks that
from outside the harness. Ordinary runs pass 60 L0 checks plus regeneration and
adapter unit tests. Test adapters suppress raw source/parser exception text;
they use a shared-parser rule cap on bounded embedded fixtures, which is not a
multi-shape stack-safety proof or an implementation of VER04's full budgets.

Eleven source-stage tests separately check authored raw byte/line coordinates,
all scalar-boundary ranges of small mixed sources, invalid UTF-8 and forbidden
bytes, UTF-8 query boundaries, exact lowered limits and a dense 1-MiB newline
input. None exposes an untrusted-source lexer/parser. The shared CV04a range
API still deliberately refuses indentation/layout synthesis.

Some parser positives deliberately have duplicate names, a Bool/Int mismatch
or an unreachable refinement. Those belong to later checkers; their presence
demonstrates why a parsed tree must not be labelled a checked specification.
