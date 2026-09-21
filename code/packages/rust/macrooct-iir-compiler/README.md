# `macrooct-iir-compiler` (PREP01 slice 1)

The MacroOct frontend for the LANG VM AOT chain — and the composition PREP01
slice 1 exists to test.

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

`tests/iir_identity.rs` holds the oracle: 11 directive pairs and 5 include
pairs, each compiled twice and compared field for field. `lang-aot`'s
`macrooct_rows_lower_to_iir_identical_to_hand_expanded_oct` applies the same
oracle to the nine matrix rows, and
`macrooct_compiles_every_oct_corpus_row_byte_identically` runs Oct's entire
existing corpus through the MacroOct frontend and demands byte-identical
output.

`IIRFunction::source_map` — per-instruction line/column provenance — is
excluded from most of those comparisons, because the two programs are different
text: `@if`, `@else` and `@end` occupy lines of their own, so their
instructions genuinely come from different lines. That exclusion is measured,
not assumed: `line_aligned_sources_produce_byte_identical_iir_including_provenance`
pads the Oct source until the lines coincide and then compares everything.

## Oct is not modified, which costs this crate two copies

PREP01 holds Oct fixed: a reference you are free to edit is not a reference.
Two consequences, each guarded so the tax stays visible rather than becoming a
fork:

| Copy | Why | Guard |
|---|---|---|
| `src/_oct_grammar.rs` | `oct-parser` keeps `parser_grammar()` behind a private `mod _grammar`; exposing it would mean editing Oct. Generated from the same `oct.grammar` by the same `grammar-tools` invocation, so it is byte-identical. | `the_embedded_oct_grammar_is_byte_identical_to_oct_parsers` |
| `MAX_RULE_DEPTH` | `oct-parser`'s constant is private too. Must stay equal, or a MacroOct program could parse where its Oct twin did not — breaking the oracle exactly at the depth limit, on input nobody writes by hand. | `the_parser_depth_cap_matches_the_one_oct_parser_documents` |

## The dialect

[`MacroOctDialect`](src/dialect.rs) answers the three questions the generic
engine refuses to answer itself:

- **`classify`** — is this line a directive? `@include "path"`, `@if <expr>`,
  `@else`, `@end`, `@define NAME body`. Nothing here is C-shaped, deliberately:
  if the engine had quietly hardcoded C's vocabulary anywhere, `@end` (rather
  than `@endif`) would break it.
- **`eval_condition`** — integers `0`–`255` in Oct's own decimal/hex/binary
  spellings, `true`/`false`, names (undefined → 0, the only state possible
  before slice 2's macro table), the six comparisons, `&&`, `||` and
  parentheses. Anything else is a diagnostic, never a guess. Implemented as an
  iterative two-stack evaluator, because in Rust a stack overflow is an
  *abort*, and a recursive one would turn the engine's depth bound into a
  process kill.
- **`lex`** — included text, through MacroOct's own grammar, so directives
  inside an include are directives.

`stringize` and `paste` are left at the trait's `None` defaults. MacroOct has
neither `#` nor `##`, and declining them is itself a test that the engine does
not assume every dialect carries a C-shaped macro facility.

## Two things that look like bugs and are not

**`@define` is refused.** Slice 1 has no macro table, so the engine answers
`Directive::Define` with a located "not supported yet (PREP01 slice 2)"
diagnostic. The dialect still *classifies* it — otherwise `@define LED 1` would
reach Oct's parser and be rejected with a syntax error about `@`, which points
at the lexer rather than at the feature the author was reaching for.

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

[PREP01](../../../specs/PREP01-generic-source-preprocessor.md) §7, slice 1.
Slice 2 adds `@define` and the macro expander; slice 3 adds MacroNib in a third
directive syntax; slice 4 is the real C dialect.
