# ruby-iir-compiler

The Ruby frontend of the LANG VM: Ruby source for one exact release, through
that release's grammar-driven parse tree, straight to InterpreterIR made of
LANG83 standard operations — which every backend (native x86-64/AArch64,
LLVM, WASM, JVM, CLR, BEAM, the `vm-core` interpreter and the JIT) lowers
natively. It implements LANG84 phase 5a (= LANG83 PR2) and supersedes the
LANG80 pilot's pipeline.

```text
source ──► GrammarLexer(ruby<release>.tokens) ──► GrammarParser(ruby<release>.grammar)
       ──► GrammarASTNode ──► this crate (walks the tree in place) ──► IIRModule
```

There is no hand-written lexer on this path: tokens come only from the
generic `GrammarLexer` over the release's `.tokens` file, via `ruby-parser`'s
release API. `tests/iir_path.rs` fails if the crate ever calls the legacy
`ruby-lexer` entry points. There is no typed Ruby AST either; the compiler
reads the CST directly.

## The `rb-int-puts` subset

```ruby
puts(1 + 2)              # 3
puts 7 / -2, 7 % -2      # -4, -1   (Ruby rounds toward negative infinity)
puts()                   # an empty line
puts(1_000 * (2 - 5))    # -3000
puts(1)
puts(2 / 0)              # prints 1, then stops: ZeroDivisionError
```

| Ruby                  | IIR                                      |
|-----------------------|------------------------------------------|
| integer literal       | `const` (`_` separators, a folded `+`)   |
| `-x`, `+`, `-`, `*`   | `neg`, `add`, `sub`, `mul`               |
| `/`, `%`              | `i64_div_floor`, `i64_mod_floor` (LANG83) |
| `puts(a, b, …)`       | evaluate all arguments, then `print_i64` each |
| `puts()`              | `putchar 10`                             |
| end of program        | `ret 0`                                  |

Every literal and intermediate value must fit in `i64`; the compiler proves
it while lowering (exact `i128` arithmetic) and refuses a program that would
need a Bignum. A zero divisor is not refused: as in Ruby, the program stops
at run time after its earlier output, with the LANG83 trap `Trap.DivZero`.

Refused on purpose: `puts -7` (whether that `-` is unary depends on the space
before it — LANG84 E6; write `puts(-7)`), bare `puts`, hex/octal/float
literals, strings, variables, and every other construct.

## Releases

```rust
use ruby_iir_compiler::{compile_source_at, FrontendBudgets, FrontendError};

let module = compile_source_at("puts(7 % 3)", Some("3.3.7"), "demo", &FrontendBudgets::default())?;
// None = DEFAULT_RELEASE (3.3.6). Unknown IDs, rows without a pair, and rows
// without an IIR profile are typed errors; nothing falls back.
assert!(matches!(
    compile_source_at("puts(1)", Some("0.49"), "demo", &FrontendBudgets::default()),
    Err(FrontendError::ReleaseNotAvailable { .. })
));
# Ok::<(), FrontendError>(())
```

`src/profiles.rs` maps releases to a `ReleaseProfile` (today `ruby33` for
3.3.5–3.3.7). Ruby 0.49 and 0.50 parse the subset but have no profile:
there `puts` is only an `IO` method and `Fixnum#/` truncates (both cited to
the tags' `io.c` and `numeric.c`). The compiler varies by release only
through the CST shape and the profile, never by comparing release IDs.

Errors (`FrontendError`) are the LANG84 C6 categories — `Lex`, `Parse`,
`Unsupported`, `Budget`, `UnknownRelease`, `ReleaseNotAvailable` — and each
names the release and a 1-based position (`Unsupported` also the CST rule).

## Running

`run_source_at(source, release)` compiles and runs on `vm-core`, returning
`RunOutput { stdout, trap }`. The `rubyvm` binary does the same for a file:

```sh
cargo run -p ruby-iir-compiler --bin rubyvm -- [--release 3.3.6] prog.rb
```

A trap prints the output so far, then `trap: Trap.DivZero` on stderr, and
exits 1. `lang-aot` compiles the same modules for every other backend
(`--lang ruby`, `.rb` files).

## Tests

- `src/tests.rs` — outputs, emitted ops, refusals, budgets, and forged
  caller-supplied trees (token types, spellings, shapes).
- `tests/release_subsets.rs` — LANG84 stage I: the
  `code/grammars/ruby/corpus/rb-int-puts` corpus on every release whose
  `iir_status` claims it, IIR identity across them, the CSV/profile
  agreement, and the host-`ruby` oracle (skips unless ruby 3.3.x).
- `tests/cst_contract.rs` — LANG84 C8 rule and token names per release.
- `tests/hostile.rs` — C5 hostile probes in a child process.
- `tests/iir_path.rs` — no hand-written lexer on the IIR path.

See [LANG84](../../../specs/LANG84-grammar-driven-versioned-frontends.md),
[LANG83](../../../specs/LANG83-iir-standard-operation-registry.md) and
[LANG80](../../../specs/LANG80-ruby-direct-iir-pilot.md).
