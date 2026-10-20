# BF03 core text/data fixtures

The [`core-text-v1` profile](../../BF03-core-profile.md) freezes 25 named
operations, tagged operands, bounded integer IDs, ordered programs and exact
canonical text. It is a data-only component of the compiler-IR architecture.

| Table | Cases | Independent evidence |
|---|---:|---|
| programs | 4 | empty, all 25 operations, sparse/reordered ordinary IDs, numeric/data/order boundaries |
| invalid_programs | 20 | sentinel/duplicate/negative IDs, version, arity/tag, register/immediate/label/comment domains, duplicate declarations, seven out-of-profile extensions |
| invalid_texts | 22 | missing/duplicate/unsupported/misplaced directives, partial/missing IDs, decimal forms, opcode case/unknown names, LF/spacing/COMMENT syntax |
| id_sequences | 4 | independent initial/empty/suffix/final-legal allocation observations |

The 50 case names and their specific witnesses are fixed. The validator uses
a separately authored literal opcode/operand matrix and positive text goldens,
constructs negative typed witnesses from its own literal record, and pins
negative texts and generator inputs. Replacing malformed-version cases with
empty strings or boundary examples with empty programs must fail. Coordinated
ID/text edits cannot redefine the sparse-ID golden. Round trips alone would
not detect that loss of evidence.

`schema.json` describes closed structural envelopes and positive typed shapes.
Negative program payloads deliberately permit malformed data; their exact
reviewed purpose is enforced by the semantic validator. JSON Schema alone
does not preserve lexical integer tokens, unique IDs, exact case rosters,
operand roles, sentinel rules, canonical bytes, or cumulative work budgets.

Run from the repository root with Python 3.10 or newer:

```sh
python code/scripts/compiler_ir_conformance.py
python -m unittest discover -s code/scripts/tests -p test_compiler_ir_conformance.py
python -m coverage run --branch --include='*/code/scripts/compiler_ir_conformance.py' -m unittest discover -s code/scripts/tests -p test_compiler_ir_conformance.py
python -m coverage report -m
```

The validator uses only the standard library; schema tests use the repository
CI dependency `jsonschema==4.26.0`. Both commands run in the existing,
unconditional CI contracts metadata step. No native fixture routing is added
until an actual native reader is delivered. Missing/renamed/deleted fixture
data fails the default validator; arbitrary paths are accepted only through
an explicit `--fixture` CLI argument, subject to the same fixed oracle.

The reader bounds raw JSON before parsing, limits depth before recursive
decoding, rejects decoded duplicate keys/invalid scalars/nonnative integer
tokens, and separately bounds direct decoded trees, programs and text.
See the profile for exact limits and canonical grammar. JSON integer `-0`
decodes as integer zero; canonical IR text excludes negative zero.

Output explicitly reports `native_adopters_checked: 0`. No lane is qualified
by validating this corpus. Existing native parser permissiveness, Rust's
zero-operand padding, native numeric domains and extension opcodes remain
adoption work. Every adopter must exercise actual APIs with independent
ownership/error/resource tests, production coverage and real package/downstream
BUILD fronts. This foundation performs no instruction, syscall, source-map,
optimizer, code-generator, executable or host-authority operation.
