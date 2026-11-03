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

The public API only constructs grammar data. A guarded untrusted-source
lexer/parser, original UTF-8 byte spans and input/work/native-stack quotas are
P006b. Name/type checking, hostile-input/label/binding validation, solvers and
Rust generation are later layers. Grammar data and syntax acceptance provide
none of those results or release authorization.

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

Some parser positives deliberately have duplicate names, a Bool/Int mismatch
or an unreachable refinement. Those belong to later checkers; their presence
demonstrates why a parsed tree must not be labelled a checked specification.
