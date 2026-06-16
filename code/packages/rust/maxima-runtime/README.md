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

### Panic-safety

The underlying `macsyma-lexer` *panics* on characters it cannot tokenize. Because
`feed` is a trust boundary over arbitrary user text, it runs the evaluation inside
`catch_unwind` and converts any panic into a clean `Err` — a stray `@` returns an
error instead of aborting the session. (The proper upstream fix is for the lexer
to return a `Result`; this façade is defensive in the meantime.)

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
