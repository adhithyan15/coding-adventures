## Unreleased - S6a fixes from the TypeScript lane's review (CV2-S6a fix)

The independent TypeScript lane ran the same crafted inputs as this one; where the two disagreed, VER04r decided.
These are the seven places this lane was wrong or lenient. The kit is unchanged: 782 cases run, 0 failed, `v2-binding`
skipped whole, all mutants reported.

- **Types table limits (section 2.2, "again by every consumer").** `wtypes.check_types_table(types, code)` refuses an
  error type with 0 or more than 64 cases or a duplicate case, a record with 0 or more than 32 fields, a variant with
  fewer than 2 or more than 64 constructors, a constructor with more than 8 fields, a field or constructor name
  outside `[A-Za-z_][A-Za-z0-9_]*`, and a declared type or constructor named like a built-in (`U8`, `Int`, `Bytes`,
  ...). It runs whether or not anything reaches the row. Codes: `type` (`v2-value`, after `malformed-json`),
  `ill-typed` (`Program`, so `v2-ir`), `descriptor` (`v2-domain`), `corpus` (a `v2-chunk` corpus part and a fault).
- **Unknown tag inside a `match` arm (section 4.2 rule 7, R69 (d)).** The tag scan followed `node[1:]`, so it never
  saw inside the arm list of a `match`, the operand list of a `tuple` or the argument list of a `ctor`; the
  enclosing tag's level was refused first. `ir.operands` now reads the scrutinee and each arm body of a `match`
  (never a pattern), and the lists of `tuple`, `list` and `ctor`. Tag-name checks win over level scoping there.
- **Undefined level names (section 4.2 rule 7).** `ir.DEFINED_LEVELS` is the seven names of section 4.1 and
  `ir.check_requires` refuses anything else. Read in a suite row (`build_suite`, `corpus`), a fault row
  (`corpus`, before the level check) and an op row (`Program.op`, `ill-typed`). `level-missing` stays for a defined
  level the runner lacks.
- **Claim names (section 5.5).** `chunks.chunk_file_name` derives the file name and refuses a claim that is not one
  or two `[A-Za-z_][A-Za-z0-9_]{0,63}` names joined by one `/` (`a/b/c`, `../x`, empty, 65 characters, a space, a
  backslash, NUL) or whose segment is a Windows device name. Section 5.5 puts the *derivation* in the corpus reader
  (slice S6b); this library never opens the file but reads the suite row, so `build_suite` applies the same check.
  The case-insensitive duplicate rule is about two rows and stays the corpus reader's.
- **Bytes literal type (Appendix A, R51, R73).** `v2-ir` printed the written type `Bytes` for a bare bytes literal.
  `expect.type` is the static type, `Bytes<len>`; `Compiled.written` is no longer reported.
- **Duplicate model parameter names.** Already `ill-typed`; now covered by a test.
- **Not changed, reported.** A chunk file whose `calls` do not sum to the row's `expected_calls`: section 5.4 defines
  `calls` as the chunk's share of `expected_calls` but no rule of sections 5.5, 11 or 14 lists the sum as a
  chunk-file cross-check, so the behaviour stands. A `tuple` descriptor against a scalar type is `descriptor` (section
  11, `v2-domain`: "a descriptor that is not one for `type`"); tested.
