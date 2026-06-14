# Changelog — constraint-instructions

## [0.1.0] — 2026-04-30

Initial release.  **LANG24 PR 24-B** — `ConstraintInstr` IR +
`Program` + text serialiser/parser for the generic Constraint-VM.

### Added

- `ConstraintInstr` enum (12 variants, `#[non_exhaustive]`):
  `DeclareVar`, `DeclareFn`, `Assert`, `CheckSat`, `GetModel`,
  `GetUnsatCore`, `PushScope`, `PopScope`, `Reset`, `SetLogic`,
  `Echo`, `SetOption`.  Mirrors LANG24 §"ConstraintIR shape"
  verbatim.

- `OptionValue` enum (`Bool / Int / Str`, `#[non_exhaustive]`) —
  values carried by `SetOption`.  `Int` is `i64` (SMT-LIB option
  ints fit comfortably); the parser explicitly rejects values that
  overflow.

- `ConstraintInstr::mnemonic()` — returns the stable text-format
  mnemonic for diagnostics.

- `Program` — validated `Vec<ConstraintInstr>`.  `Program::new`
  enforces:
  - **Scope balance.**  No `PopScope` without a matching prior
    `PushScope`.  `Reset` clears the scope stack.
  - **Identifier safety.**  Every name (variable, function, sort,
    quantifier binder, `Apply` head, `Uninterpreted` sort) is
    non-empty, contains no whitespace or s-expression delimiters
    (`(`, `)`, `;`, `"`), does not parse as an integer literal,
    and is not one of the format's reserved tokens.  This
    guarantees `parse_program(&p.to_string()) == p`.

  Plus `Program::new_unchecked` for callers that have already
  validated, `instructions()`, `into_instructions()`, `len()`,
  `is_empty()`.

- `ProgramError` (`UnmatchedPop`, `BadIdentifier`).

- `Display` for `ConstraintInstr`, `OptionValue`, `Program`,
  `ProgramError`, `ParseError` — emits the SMT-LIB-flavoured
  s-expression text format.

- `parse_program(input: &str) -> Result<Program, ParseError>` —
  recursive-descent parser over an s-expression tokenizer.
  Round-trips with `Display`.

- `ParseError` (`UnexpectedEof`, `UnexpectedCloseParen`,
  `UnknownOpcode`, `BadArgs`, `BadString`, `BadInt`, `BadSort`,
  `BadLogic`, `Program(ProgramError)`).

- 56 unit tests covering `Display` per opcode, mnemonic uniqueness,
  scope-balance validation, identifier-safety validation, round-trip
  per opcode, round-trip per `Predicate` variant (boolean, var, int,
  real, apply, all combinators, all comparisons, ite, quantifiers,
  arrays), round-trip for a realistic QF_LIA program, parser edge
  cases (comments, whitespace, unmatched parens, unterminated
  strings, bad escapes, integer overflow on `set-option`,
  validation-error propagation), UTF-8 in string literals, and
  invalid-UTF-8 rejection.

### Format choices vs strict SMT-LIB

Three divergences chosen for round-trip exactness without sort
information (strict SMT-LIB ships in `smt-lib-format`, PR 24-E):

- `Iff` is `(iff a b)` (not `(= a b)` overloaded with `Eq`).
- `Real(num/den)` is `(/ num den)` (not bare `num/den` which would
  tokenize as a single symbol).
- `BitVec(w)` is `(BitVec w)` (no leading `_`).

### Notes

- Pure data + algorithms.  Single dependency: `constraint-core`
  (path).  No I/O, no FFI, no unsafe.  See
  `required_capabilities.json` (empty capability set).
- All public enums are `#[non_exhaustive]` so future LANG24
  variants plug in without breaking downstream matchers.
- **Caller responsibilities** (documented as crate-level
  non-guarantees): parser and `Display` recurse on parenthesis
  / AST depth without an explicit guard.  Callers ingesting
  untrusted text should bound input length / depth at the
  boundary.  A `parse_program_with_limit(input, max_depth)`
  variant is filed as a follow-up.
- Security review caught:
  - **HIGH**: round-trip soundness — reserved-token / digit-only
    / whitespace-containing identifier names broke round-trip.
    Fixed via `Program::new` identifier validation + tests.
  - **HIGH**: non-ASCII string-literal round-trip — `read_string`
    decoded bytes per-character, truncating multi-byte UTF-8 to
    Latin-1.  Fixed by collecting bytes and decoding UTF-8 at
    end-of-literal; invalid UTF-8 now surfaces as
    `ParseError::BadString`.  Tests added for both Unicode
    round-trip and invalid-UTF-8 rejection.
