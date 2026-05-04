# twig-formatter

**Canonical Twig pretty-printer.**  The prettier / rustfmt
equivalent for Twig.  The first authoring-experience deliverable:
drop noisy whitespace arguments at PR time, give every Twig file
one canonical shape.

Built on [`format-doc`](../format-doc/) — layout decisions
(compact vs block) are made by the Wadler-style realiser, this
crate just describes the shape of every Twig form using
combinators.

---

## Architecture

```text
Twig source string
        │
        ▼ twig_parser::parse
  twig_parser::Program (AST)
        │
        ▼ this crate's emit_*  (AST → Doc)
  format_doc::Doc
        │
        ▼ format_doc::layout_doc  (width-aware realisation)
  format_doc::DocLayoutTree
        │
        ▼ format_doc::render_text  (or paint pipeline, future)
Canonical Twig source string
```

The same `Doc` builds drive ASCII output today and the paint-vm
pipeline tomorrow.

---

## Public API

| Entry point             | Description                                                                                                           |
|-------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `format(source)`        | `&str` → `Result<String, FormatError>` — parses and formats.  The common case.                                        |
| `format_program(p, c)`  | `&Program + &Config` → `String` — formats already-parsed AST.  Each form realised independently (see below).          |
| `program_to_doc(p)`     | `&Program` → `format_doc::Doc` — emit one Doc for the whole program (caveat: forces broken layout per form).          |
| `form_to_doc_pub(f)`    | `&Form` → `format_doc::Doc` — emit Doc for one top-level form.  Use this if integrating with your own paint pipeline. |
| `Config { print_width, indent_width }` | Default: 80 / 2.                                                                                          |
| `FormatError`           | `Parse(TwigParseError)` (`#[non_exhaustive]`)                                                                         |

## Guarantees

- **Idempotency.**  `format(&format(s)?)? == format(s)?`.
- **Semantic preservation.**  `parse(&format(s)?)? == parse(s)?` (modulo source positions).
- **Determinism.**  Same input + same config → same output, byte-for-byte.
- **Width-aware layout.**  80-column default; long forms break, short ones stay compact.

## Style rules

Twig is a Lisp; rules mirror canonical scheme/clojure layout:

- Atoms render as their source form (`42`, `#t`, `#f`, `nil`, `'foo`, `x`).
- Compound forms use `group` so the realiser picks compact when it fits, indented multi-line otherwise:
  - `(if cond then else)` — children indented 4 cols.
  - `(let ((x e1) (y e2)) body)` — bindings aligned; body indented 2.
  - `(begin e1 e2 …)` — exprs indented 2.
  - `(lambda (x y) body)` — params on the same line; body indented 2.
  - `(define name expr)` — name on the same line; expr indented 2 if it doesn't fit.
  - `(f a1 a2 …)` — first arg same line as fn; subsequent args indented under it when broken.
- Top-level forms separated by a single blank line.
- Output ends with a single trailing newline (POSIX convention).

## Top-level form realisation: why string concatenation, not Doc

`format_program` realises **each top-level form independently**
and joins the rendered strings with `"\n\n"` rather than placing
`hardline()` separators in a single Doc tree.

This is intentional: format-doc's `fits()` look-ahead would
otherwise see hardline separators in the pending stack and force
every form into broken mode, even short ones that should stay
compact.  Splitting at the top level lets each form pick the right
flat/broken shape independently — the same approach Prettier
takes for top-level statements.

`program_to_doc` (the single-Doc variant) is still provided for
callers wanting one Doc to pass into their own paint pipeline; it
documents the layout caveat.

## What this formatter does NOT preserve

- **Comments.**  The Twig grammar is comment-stripping at the lexer layer, so comments don't survive into the AST.  Don't run on commented files; rustfmt-style trivia preservation is the appropriate follow-up.
- **Whitespace and column layout.**  The whole point.
- **Surface form of `'foo` vs `(quote foo)`.**  Parser collapses both to `SymLit`; formatter emits `'foo`.

## Caller responsibilities

Inherits the non-guarantees of `twig-parser` and `format-doc`.
Adversarial input that produces an unbounded AST is bounded by
the parser's depth cap; the doc-building pass here is mutually
recursive on AST shape (matches twig-parser's depth bound) and
adds no separate cap.

## Example

```rust
use twig_formatter::{format, format_program, Config};

let s = format("(define   (square   x)\n  (*   x x))").unwrap();
assert!(s.starts_with("(define square (lambda (x) (* x x)))"));

// With a custom width
let p = twig_parser::parse("(if cond then else)").unwrap();
let s = format_program(&p, &Config { print_width: 10, indent_width: 2 });
// Realiser breaks the `if` because 10 cols isn't enough.
```

## Dependencies

- [`format-doc`](../format-doc/) — Doc algebra + width-aware realisation.
- [`twig-parser`](../twig-parser/) — Twig source → typed AST.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

## Tests

37 unit tests covering atoms, compact compound forms, block-form
breaks for every Twig construct (`if`, `let`, `begin`, `lambda`,
`apply`), `(define (f x) body)` lambda lowering round-trip,
multi-form programs with blank-line separation, whitespace
collapse, idempotency on atoms / compact forms / block forms,
semantic preservation (parse → format → parse same AST shape),
trailing-newline contract, narrow / wide / custom-indent config,
unparseable-input error path, `program_to_doc` smoke test, and
two real-world snippets (factorial, nested let).

```sh
cargo test -p twig-formatter
```

## Roadmap

- **Comment-preserving format.**  Add a "trivia channel" to the
  lexer + parser, then thread comments through this crate as
  Doc-level annotations.
- **Format range / partial format.**  LSP code-action support —
  format only the selected region while keeping the rest
  untouched.
- **`twig fmt` CLI.**  Standalone binary using this crate.
- **Integration with `format-doc-to-paint`.**  Once the paint
  bridge ships, expose a `format_to_paint_scene` entry point so
  editors can render formatted Twig directly without going
  through monospace text.
