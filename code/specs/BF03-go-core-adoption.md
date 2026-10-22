# BF03 Go core profile adoption

This lane adopts the frozen [core-text-v1](BF03-core-profile.md) data/text
component through real public Go APIs in `code/packages/go/compiler-ir`.
The existing broad `Parse`, `Print`, opcode enumeration, mutable Add methods
and unchecked `IDGenerator` retain their established behavior. In particular,
Nib's colon-bearing `IrLabel` COMMENT payloads remain valid legacy debug data.
This is one native component adoption, not source-map, optimizer, codegen,
runtime, target execution or whole-language qualification.

## Production boundary

- `IrText{Value string}` is an additional sealed value operand. Core COMMENT
  accepts exactly this tag; a label with the same characters is a wrong tag.
- `ValidateCoreProfile(*IrProgram) error` checks all frozen version, entry,
  declaration, instruction, tag, name, numeric, ID, uniqueness and work limits
  without changing input. Native nil/empty slices both project to empty vectors.
  Only value operands are admitted: pointers, typed nils and nil interfaces
  return controlled errors. Unknown opcodes fail before any unsafe printer use.
- `PrintCoreProfile(*IrProgram) (string, error)` validates before rendering
  the exact frozen bytes, including unpadded zero-operand operations.
- `ParseCoreProfile(string) (*IrProgram, error)` bounds source bytes before
  splitting/decoding and accepts only the canonical language. Bounded legacy
  parsing may be reused internally, with COMMENT deliberately constructed as
  `IrText` in production, followed by complete validation and byte-identical
  canonical reprint. No transport reader may retag a supplied label as text.
  Missing/duplicate/reordered directives, defaulted IDs, aliases and discarded
  text must fail the exact reprint comparison. Successful outputs own their
  slices; mutation of one result cannot affect another call or source input.
- `CoreIDSequence(start int64, count int) ([]int64, int64, error)` preflights
  start0..2147483647, count0..256 and the whole allocation before allocating.
  Failure returns no values and the unchanged supplied start. The exhausted
  next observation2147483648 is representable without an `int` narrowing.
  This does not change legacy generator behavior or claim global uniqueness.

The maximum canonical text is65536bytes, declarations32, instructions256 and
operands3/instruction. Validate bounds before traversing operand slices and
before printing; nil programs and malformed LABELs must never panic. Labels
and comments follow the exact distinct ASCII domains in the parent profile.
Sparse/nonmonotone ordinary IDs and unresolved references remain admissible.
The APIs perform no IR execution, callbacks, filesystem, network or host loading.

## Tests and native evidence

Commit tests before production implementation and demonstrate an actual RED
result for absent strict APIs. Consume every named50-case row and all25literal
opcode roles from the existing neutral corpus. Test transport separately:
read at most1048577bytes, reject malformed UTF8/BOM/surrogate escapes, duplicate
decoded keys, unknown/missing fields, numeric aliases/floats/booleans/overflow,
trailing data, excessive depth16/cases128/aggregate instructions4096,
declarations512/operands12288 and decoded value/string budgets. Reader failures
are not production rejection or production coverage credit.

Keep independently authored case-name/operation rosters, role/category rows,
sparse42/7/99 golden and frozen corpus digest. A changed corpus requires a
deliberate independent review of those anchors; never regenerate expected
answers from the production renderer. Corrupt valid/invalid groups, tags,
IDs, order and golden bytes in reader tests and require failure. Unsupported
operation spellings must fail actual `ParseOp` admission and strict native
opcode validation, not be skipped as a reader error.

Test strict admission and legacy compatibility side by side, ownership,
nil/pointer operands, both memory-offset tags, maximal boundaries, budget
failures and atomic ID exhaustion. Run package BUILD, Go statement coverage
at least95% on production files, vet/build/gofmt, race tests when supported,
all18actual module consumers and the current transitive affected closure.
Go's coverage metric is statements, not a measured branch percentage.

The exact external path `code/specs/fixtures/compiler-ir-v1/cases.json` seeds
`go/compiler-ir` before the detector's external-fixture early return. Honor
all/go filters; fail closed if the Go reader package is missing. Preserve
normal source union, renamed/deleted old path selection and platform plans;
lookalike filenames, backslash strings and other fixture families do not seed.
Run real unforced detection and native BUILD execution as well as unit plans.
Finish metadata/build-plan/parity/diff and independent security review before
publication. Full build-tool adapters and all other IR lanes remain pending.

## Delivery plan

1. Incorporate postmerge inventory/ownership findings, then select this single
   owner in the fresh clean worktree. Commit this specification first.
2. Add independent native corpus/compatibility/resource tests; record RED.
3. Implement the additive production boundary and exact CI selection relation.
4. Validate package, all consumers, build detector, quality and security;
   update changelogs/README/roadmap/journal and open one ready PR.
5. Monitor actual checks; enable auto-merge only after all terminal acceptable
   checks and no conflict. Never manually merge or start a second active PR.
