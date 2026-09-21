# `macrooct-iir-compiler` (PREP01 slices 1-2)

The MacroOct frontend for the LANG VM AOT chain — and the composition PREP01
exists to test.

## What it does, in full

```text
    MacroOct source
        │
        ▼  macrooct-lexer                       ← the only new grammar
    [Token]  with AT_INCLUDE / AT_IF / AT_ELSE / AT_END / AT_DEFINE
        │
        ▼  source_preprocessor::preprocess(…, &MacroOctDialect, …)
    [Token]  pure Oct — every directive token consumed
        │
        ▼  Oct's parser grammar           UNCHANGED
        ▼  oct_type_checker::check_ast    UNCHANGED
        ▼  oct_iir_compiler::compile_ast  UNCHANGED
    IIRModule  →  all eight backends, none of which learns anything
```

That is the whole crate. **A preprocessor runs before the parser, so a dialect
that gains one needs no new parser, no new type checker and no new backend.**
If that composition did not work, the engine would not be a clean layer — and
[PREP01](../../../specs/PREP01-generic-source-preprocessor.md) §7 puts MacroOct
first specifically so that would be learned here, on a small language, rather
than in C. C is the motivating customer for the preprocessor engine, and a
customer is exactly what should not get to design the boundary.

## The oracle: identical IIR, not "it also works"

MacroOct is a dialect of Oct, so a MacroOct program's hand-expanded equivalent
is a **valid Oct program**. The acceptance criterion is therefore not that the
MacroOct program runs correctly — it is that the two lower to the *same IIR*:

| Claim | Cost to check | What it proves |
|---|---|---|
| "it runs correctly" | 8 backends × N programs | the selected branch behaves |
| **"identical IIR"** | one comparison per program | the preprocessor contributed *nothing* beyond selecting text — every backend result then follows for free |

`tests/iir_identity.rs` holds the oracle: 25 directive pairs (11 for
conditionals, 14 for `@define`) and 6 include pairs, each compiled twice and
compared field for field. `lang-aot`'s
`macrooct_rows_lower_to_iir_identical_to_hand_expanded_oct` applies the same
oracle to the fourteen matrix rows, and
`macrooct_compiles_every_oct_corpus_row_byte_identically` runs Oct's entire
existing corpus through the MacroOct frontend and demands byte-identical
output.

`IIRFunction::source_map` — per-instruction line/column provenance — is
excluded from most of those comparisons, because the two programs are different
text: `@if`, `@else` and `@end` occupy lines of their own, so their
instructions genuinely come from different lines. That exclusion is measured,
not assumed: `line_aligned_sources_produce_byte_identical_iir_including_provenance`
pads the Oct source until the lines coincide and then compares everything.

## Oct is not modified, and this crate keeps no copy of it

PREP01 holds Oct fixed: a reference you are free to edit is not a reference.
Note what that does *not* mean — Oct's **language** is untouched (grammar,
semantics, type rules, corpus, matrix rows, specs), while Oct's **crate** gained
exactly one additive entry point, `create_oct_parser_from_tokens`, which hands
back the parser Oct already builds over tokens the caller supplies.

An earlier draft avoided even that by embedding a second compiled copy of
`oct.grammar` and restating `MAX_RULE_DEPTH`. That was worse in a way a
byte-identity test only half covered: the copied *grammar* was guarded, but the
copied *constant* was not, so a retuned parser depth in Oct would have diverged
silently. Reusing Oct's builder removes both copies and makes "MacroOct reuses
Oct's parser unchanged" literally true rather than aspirational.

## The dialect

[`MacroOctDialect`](src/dialect.rs) answers the three questions the generic
engine refuses to answer itself:

- **`classify`** — is this line a directive? `@include "path"`, `@if <expr>`,
  `@else`, `@end`, `@define NAME body` and `@define NAME(a, b) body`. Nothing
  here is C-shaped, deliberately: if the engine had quietly hardcoded C's
  vocabulary anywhere, `@end` (rather than `@endif`) would break it.
- **`eval_condition`** — integers `0`–`255` in Oct's own decimal/hex/binary
  spellings, `true`/`false`, names (always 0 — see below), the six
  comparisons, `&&`, `||` and
  parentheses. Anything else is a diagnostic, never a guess. Implemented as an
  iterative two-stack evaluator, because in Rust a stack overflow is an
  *abort*, and a recursive one would turn the engine's depth bound into a
  process kill.
- **`lex`** — included text, through MacroOct's own grammar, so directives
  inside an include are directives.

`stringize` and `paste` are left at the trait's `None` defaults — including in
slice 2, the slice that lands macros. MacroOct genuinely has neither `#` nor
`##`: Oct has no string type for a stringize to produce, and no
identifier-building idiom a paste would serve. A full macro facility that still
declines two of C's operators is the strongest available form of the genericity
test.

## `@define`, and the one space that changes a line's meaning

```macrooct
@define LED_PORT 1              object-like
@define SHIFT(x)  x + 1         function-like, one parameter
@define SHIFT (x) x + 1         OBJECT-like, body `(x) x + 1`
```

The `(` counts as a parameter list only when it **touches** the name. That is
C's rule, adopted rather than invented, and the reason a language free to choose
differently still chooses it: without adjacency there is no way to give an
object-like macro a parenthesised body — and `@define MASK (0xF0)` exists
precisely so `MASK + 1` cannot reassociate at the use site.

Everything else about a definition is checked, and nothing about the body is: a
missing name, a non-identifier name, an unterminated or malformed parameter
list, and a **duplicate parameter name** are each a located diagnostic. The
duplicate is worth naming: substitution resolves a parameter by position, so
`@define F(x, x) x + x` would let the first `x` win both slots and `F(1, 2)`
would quietly mean `1 + 1`.

Expansion itself lives in the engine (`source_preprocessor::macros`), not here.
The dialect answers "what did the author write"; the engine answers "what does
it mean", and the second question has the same answer in every language.

## Two things that look like bugs and are not

**A controlling expression is not macro-expanded.** After `@define LED_PORT 1`,
the line `@if LED_PORT == 1` still evaluates `LED_PORT` as an undefined name —
zero — and takes the `@else` branch. This is the one place MacroOct diverges
from the C rule it otherwise follows, and it is an *interface* limitation rather
than a choice: `Dialect::eval_condition` receives a bare `&[Token]`, with no
macro table and no expansion applied, so no dialect can do better without the
trait changing shape. Closing it belongs in the engine (see
`dialect::operand_value`'s header for why doing it here would make the dialect
stateful, which is what today lets one instance be shared across translation
units).

**The module's `language` field says `"oct"`.** `compile_ast` is Oct's, and the
string is true: after preprocessing, the program being lowered is Oct. It is
also what makes the identity oracle a comparison of programs rather than of
labels.

## Usage

```rust
use coding_adventures_macrooct_iir_compiler::compile_source;

let module = compile_source(
    "@if 1\nfn main() { out(1, 42); }\n@else\nfn main() { out(1, 7); }\n@end\n",
    "demo",
).unwrap();
assert_eq!(module.functions.len(), 1);
```

`compile_source` takes a *string*, which has no directory, so an `@include`
cannot resolve against anything and fails with "no such included file". For
includes:

- `compile_source_with_includes(src, name, &[(path, text)])` — an in-memory
  include set, for tests and embedders with their own virtual filesystem.
- `preprocess_source(src, file, &mut fs, bounds)` with a
  [`RootedFs`](../source-preprocessor) over declared search roots — for real
  files. `RootedFs` is the *only* sanctioned implementation for real paths;
  the engine's security contract forbids a dialect supplying its own
  `SourceFs`, because containment, symlink, UNC, device-name and
  regular-file checks all live there and nowhere else.

## Spec

[PREP01](../../../specs/PREP01-generic-source-preprocessor.md) §7, slices 1
and 2. Slice 3 adds MacroNib in a third directive syntax; slice 4 is the real C
dialect.
