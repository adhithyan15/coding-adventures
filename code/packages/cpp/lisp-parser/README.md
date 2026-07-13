# lisp-parser (C++)

A **Lisp parser** — token stream → S-expression AST — **header-only** in pure
ISO C++17 (namespace `ca::lisp_parser`). A faithful port of the Rust
[`lisp-parser`](../../rust/lisp-parser) crate. Builds on the sibling header-only
[`lisp-lexer`](../lisp-lexer).

## What it does

Lisp's grammar is famously tiny (6 rules), so this is a small recursive-descent
parser: `parse` tokenizes the source and turns the flat token stream into a
`std::vector<SExpr>` of top-level forms.

Node kinds (`SExprKind`): **Atom** (number / symbol / string), **List**,
**DottedPair** (`(a . b)`), and **Quoted** (`'x`).

## API

- `std::vector<SExpr> parse(const std::string& source)` — throws `ParseError`
  on a lexer or syntax error.
- `std::vector<SExpr> parse_tokens(std::vector<lisp_lexer::Token>)`.
- `SExpr`: `kind()`, `atom_kind()` / `atom_value()`, `find_atoms()`,
  `count_lists()`, `count_quoted()`. `SExpr` is move-only (it owns children via
  `std::unique_ptr` for the single-child forms).

## Dependency

Depends on `cpp/lisp-lexer` (header-only); `run.sh` adds `../lisp-lexer/include`
to the include path. Both packages ship together.

## Design notes

- **Recursive `std::variant` AST.** `SExpr` wraps a
  `std::variant<Atom, List, DottedPair, Quoted>`; Rust's `Result` becomes a
  thrown `ParseError`, so RAII unwinds partial trees with no manual cleanup.
- **Header-only.** `#include "lisp_parser.hpp"` and go.

## Usage

```cpp
#include "lisp_parser.hpp"
using namespace ca::lisp_parser;

auto program = parse("(+ (* 2 3) 4)");
// program.size() == 1; program[0].kind() == SExprKind::List
auto atoms = program[0].find_atoms();   // {"+", "*", "2", "3", "4"}
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
