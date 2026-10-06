# Language Frontend Neutral Fixtures v1

## Purpose and ownership

The established Dart and Swift lanes lack twelve paired grammar-driven lexer
and parser families: C#, Excel, Java, JavaScript, Lattice, Python, Ruby, SQL,
Starlark, TypeScript, Verilog, and VHDL. This specification freezes a small,
language-neutral input/output contract before any of those 48 package slots
are implemented. Each later vertical owner must consume the same checked cases
through its native lexer and parser; this fixture tranche does not create those
packages, read host files at runtime, or claim that existing frontends already
conform.

The canonical grammar source remains `code/grammars/<family>/*.tokens` and
`*.grammar`. Every fixture grammar reference names an exact repository-relative
file and its SHA-256 over unmodified bytes. The validator compares every hash
against the checkout and rejects missing, extra, or duplicated grammar paths.
The checked-in manifest records every available version of the twelve families,
even when v1 has only a baseline behavioral case for that version. A later
native consumer must embed or bundle the exact canonical grammar according to
`compiled-embedded-grammars.md`; it must not read this fixture tree or walk up
to the monorepo grammar tree at runtime. The manifest is drift evidence, not an
alternate grammar source or permission to parse repository files at runtime.

## Closed case format

`code/specs/fixtures/language-frontends-v1/` contains `schema.json`,
`grammars.json`, and `cases.json`. Each case has a unique stable ID, a family,
an exact grammar version from the manifest, ASCII source text, and one outcome:
`ok` or `error`. Successful cases carry exact ordered tokens, including EOF,
and a compact complete AST. A token has `type`, `value`, one-based `line` and
`column`; only contract-relevant nonzero flags are included. AST nodes have a
rule name and ordered children; leaves are token projections. No AST source
spans are compared in v1 because empty-node positions differ among existing
engines. Error cases carry a stable category, never a host path or raw
exception message. The checked-in validator enforces schema closure, unique
IDs, bounded source and tree sizes, grammar hash integrity, case/manifest
membership, and at least one successful token-plus-AST case per family.

The oracle is independent of whichever implementation is being ported. A
normalizer may translate API spellings (`type_name` versus `type`) and omit
irrelevant zero/null flags; it must never rewrite expected token values,
ordering, AST structure, or error outcomes to match an implementation. For
multi-edition families, the case identifies the edition explicitly; a default
edition is not inferred from Rust or Python wrapper behavior.

## Portable subset and deliberately separate semantics

Baseline v1 cases use only grammar constructs and source forms already common
to the relevant generic engines. They avoid Dart/Swift divergences in soft
keywords, token-error-definition retention, separated-repetition syntax,
noncommon string escapes, and indentation/layout mode. Python and Starlark
layout behavior is owned by the existing Dart indentation-mode item before
their Dart frontend children can claim full parity. JavaScript's versioned
Rust reference and generic Python wrapper are not silently conflated; Ruby's
contextual/state-machine lexer is not replaced with the simpler Python
grammar wrapper. C# and Java nested generic `>>` disambiguation belongs to
the separately owned contextual-generic-closer contract. A v1 case may pin
the raw lexer token there, but must not claim the later parser rewrite.

Each family receives a smallest valid program with complete token and AST
expectations, plus targeted edition or lexical contrasts where established
oracles agree. Existing disagreements are recorded explicitly and remain
future work for the relevant substrate or vertical owner; a passing fixture
validator alone is not evidence that a Dart or Swift package conforms.

## Validation

The fixture validator and its tests must run without Dart or Swift packages.
Native validation later runs the six Dart/Swift `grammar-tools`, `lexer`, and
`parser` suites and consumes the same cases through each new vertical. This
fixture-only tranche additionally runs JSON Schema validation, manifest hash
checking, the repository parity/collision report, `git diff --check`, and
build-file validation. Grammar embedding regeneration checks remain separate
native gates and must be run if this tranche changes generated grammar data
(which it does not intend to do).
