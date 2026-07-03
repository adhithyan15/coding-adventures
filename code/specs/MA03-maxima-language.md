# MA03 — Maxima (a Macsyma reuse)

## Status

Active. Delivers **Maxima** — the GPL-licensed descendant of DOE Macsyma — as a
thin reuse of the existing `macsyma-runtime` stack, exactly as
[GNU Octave](MA01-matlab-language.md) was delivered as a thin reuse of
`matlab-runtime`. No new lexer, parser, compiler, or symbolic engine: Maxima's
surface syntax is, for the subset we support, *identical* to Macsyma's, so the
entire frontend-to-evaluator pipeline is shared verbatim.

This realizes **Wave 1 (Maxima)** of the historical-math roadmap
([HML00 §7](HML00-historical-math-languages-roadmap.md)): "≈ Macsyma + GPL-era
surface… reuses the Macsyma frontend/runtime almost wholesale."

## §1 Why Maxima is (almost) free

Maxima *is* Macsyma. When MIT's Project MAC handed DOE Macsyma to the Department
of Energy, William Schelter maintained a copy ("Maxima") and in 1998 released it
under the GPL. The two share the same algebraic surface: `:` for assignment, `;`
to display and `$` to suppress, `%i`/`%o` input/output history, `diff`/`integrate`/
`expand`/`factor`/`solve`, the `%pi`/`%e` constants, list/matrix syntax, and so on.
A program written for one runs on the other.

The repo already has a complete Macsyma vertical slice: `macsyma-lexer`,
`macsyma-parser`, `macsyma-compiler`, and `macsyma-runtime` (a session facade over
the Rust `symbolic-vm` and the twenty `cas-*` crates — simplification, calculus,
trig, solve, substitution, pretty-printing). Because the syntax matches, **Maxima
reuses all of it unchanged**. The only new code is:

1. `maxima-runtime` — a `MaximaSession` that wraps a `MacsymaSession` and exposes
   a string-in/string-out `feed`, formatting each displayed result as Maxima's
   `(%o«n») «text»` echo line.
2. `maxima-repl` — the interactive `maxima` binary: prompts (`(%i«n») ` / `... `),
   line-continuation until a statement terminator (`;` or `$`) with balanced
   brackets, and the standard quit/EOF handling. A sibling of `octave-repl`.

This is the symbolic-CAS analogue of the numeric Octave-over-MATLAB shim: a
second historical language for the cost of a façade plus a REPL.

## §2 What is supported

**Exactly what `macsyma-runtime` evaluates — no more, no less.** Maxima inherits
the wrapped evaluator's power verbatim, so this section is honest about where
that evaluator currently stops (convention 9). The whole Macsyma function
*surface* parses and is accepted; what actually *reduces* today is:

- **Arithmetic** over exact rationals and floats (`2/4` → `1/2`, `1 + 2*3` → `7`).
- **Differentiation** — `diff(x^3, x)` → `3*x^2`, `diff(sin(x), x)` → `cos(x)`.
- **Integration** — `integrate(x^2, x)` → `x^3/3`.
- **Factoring** — `factor(x^2 - 1)` → `(x - 1)*(x + 1)`.
- **Substitution** — `subst(3, x, x^2 + 1)`.
- **Bindings & history** — `x : 5$` then `x + 1;` → `6`; the `%i«n»`/`%o«n»`
  counters; `;` displays and `$` suppresses.

Other CAS verbs — `expand`, `ratsimp`, `solve`, `limit`, `taylor`, the `trig*`
family, list operations, and the allow-listed `load("orthopoly")` package — are
**parsed and accepted** but, in the current `macsyma-runtime`, echo back
symbolically rather than reducing. They are not errors; they simply pass through
unevaluated. As `macsyma-runtime`'s evaluator grows to reduce them, **Maxima
inherits that for free**, because this crate adds no evaluation logic of its own —
it is a pure presentation façade.

## §3 The `feed` contract

`MaximaSession::feed(src)` parses and evaluates `src` through the wrapped
`MacsymaSession::eval_source`, then concatenates an echo line **only for results
whose `display` flag is set** (i.e. statements terminated by `;`, not `$`):

```
(%o«output_index») «output_text»
```

`output_index` is Macsyma's 1-based `%o` counter, carried straight through; it is
shared session state, so a suppressed (`$`) statement still advances history but
prints nothing. A `CompileError` from the evaluator (a parse or surface error) is
surfaced verbatim via its `Display` impl as an `Err(String)`, so the REPL can show
it without a Rust backtrace. There is also a one-shot `maxima_runtime::eval(src)`
convenience on a fresh session.

## §4 The REPL

`maxima-repl` mirrors `octave-repl`'s single-threaded driver. A statement is
*incomplete* (the REPL asks for more, switching the prompt to `... `) when, over
the accumulated buffer with `"`-strings and `/* */`-free surface handled, **no
terminating `;` or `$` has appeared yet outside a string, or bracket/paren/brace
depth is still positive.** `quit;`, `quit()`, `exit`, and EOF (Ctrl-D) end the
session. Errors are printed and the session continues. Statement terminators are
detected only outside string literals so that `s : "a;b"; ` is one statement.

## §5 Out of scope (for now)

The same subset boundary as `macsyma-runtime`: no plotting, no `batch`/file I/O,
no arbitrary package `load` beyond the allow-list, no Lisp escape, no `tex`/GUI.
These are Macsyma-stack concerns; when that stack grows them, Maxima inherits them
for free.

## §6 Divergence from the HML00 sketch

HML00 §7 sketched Maxima as four items (M-1 grammar diff, M-2 lexer/parser, M-3
runtime alias, M-4 builtins). In practice the *grammar diff is empty* for the
supported subset — Maxima and Macsyma parse identically — so M-1/M-2 collapse
away and there is no `maxima.tokens`/`maxima.grammar`. The work is exactly M-3
(the runtime alias) plus the REPL, with M-4's "GPL-era builtins" already present
in the shared `cas-*` surface. This is a strict simplification, noted here so the
roadmap and the code agree (per repo convention 9).

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MA01`](MA01-matlab-language.md) (the Octave-over-MATLAB precedent),
`macsyma-runtime`, `symbolic-vm`, the `cas-*` crates.

External: Joel Moses & William Martin, *MACSYMA* (Project MAC, MIT); William
Schelter, *Maxima* (GPL release, 1998); the Maxima Manual.
