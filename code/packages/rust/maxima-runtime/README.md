# coding-adventures-maxima-runtime

A **Maxima** runtime session, built as a thin reuse of the Macsyma stack.

[Maxima](https://maxima.sourceforge.io/) is the GPL-licensed descendant of DOE
Macsyma. For the subset we support the two languages share an identical algebraic
surface, so this crate adds **no interpreter of its own**: a `MaximaSession` owns
a [`macsyma_runtime::MacsymaSession`] and presents Maxima's string-in/string-out
console contract over it. The entire pipeline beneath — the `macsyma-lexer` →
`macsyma-parser` → `macsyma-compiler` frontend, the `symbolic-vm`, and the twenty
`cas-*` crates — is reused unchanged.

This is the symbolic-CAS analogue of how GNU Octave was delivered as a thin reuse
of `matlab-runtime`: a second historical language for the cost of a façade plus a
REPL, because the syntax already matched.

## Where it fits in the stack

```
maxima-repl  (the `maxima` binary)
    │
    ▼
maxima-runtime  ← you are here   (presentation façade)
    │
    ▼
macsyma-runtime  (MacsymaSession: eval_source → EvalResult)
    │
    ▼
macsyma-lexer / -parser / -compiler · symbolic-vm · cas-* crates
```

## Usage

```rust
use coding_adventures_maxima_runtime::{MaximaSession, eval};

// One-shot:
let echo = eval("diff(x^3, x);").unwrap();
assert!(echo.contains("3")); // (%o1) 3*x^2

// Persistent session — bindings and %o history carry across calls:
let mut s = MaximaSession::new();
assert_eq!(s.feed("x : 5$").unwrap(), "");      // $ suppresses output
assert!(s.feed("x + 1;").unwrap().contains("6")); // ; displays → (%o2) 6
```

`feed` returns the console echo: one `(%o«n») «text»` line per **displayed**
result (a statement terminated by `;`; a `$`-terminated statement runs and
advances the `%o` counter but prints nothing). A surface/parse error comes back
as `Err(String)`.

### Robustness at the trust boundary

`feed` takes arbitrary user text, so it defends against three failure modes of
the reused Macsyma stack:

- **Unwinding panics** — the `macsyma-lexer` *panics* on a character it cannot
  tokenize, so `feed` runs evaluation inside `catch_unwind` and returns a clean
  `Err` (a stray `@` errors instead of aborting).
- **Stack overflow** — the parser/VM recurse on nesting with no depth limit, so
  deeply nested input would overflow the stack and *abort the process
  uncatchably*. `feed` caps total size (`MAX_INPUT_LEN`) and per-statement token
  count (`MAX_STATEMENT_TOKENS`, counted from the **real** lexer so comment/string
  skip rules can't bypass it), and evaluates on a large-stack worker thread.
- **Mutex poisoning** — after any caught panic the wrapped session is rebuilt, so
  a panic in a lock-holding handler can't permanently brick it.

The proper upstream fixes are a `Result`-returning lexer and a parser/VM
recursion-depth limit; these are defensive shims in the façade meanwhile.

## What evaluates

Maxima inherits `macsyma-runtime`'s evaluation power exactly. Today that means
exact arithmetic, `diff`, `integrate`, `factor`, and `subst` genuinely reduce;
`expand`/`ratsimp`/`solve`/`limit`/`trig*` parse and echo symbolically until the
macsyma evaluator grows to reduce them — at which point Maxima inherits it for
free. See [`MA03-maxima-language.md`](../../../specs/MA03-maxima-language.md).

## Testing

```
cargo test -p coding-adventures-maxima-runtime
```
