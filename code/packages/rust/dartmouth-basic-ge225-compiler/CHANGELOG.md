# Changelog — dartmouth-basic-ge225-compiler

## [0.1.0] — 2026-04-28

### Added

- Initial release: full Dartmouth BASIC → GE-225 four-stage compiled pipeline.
- `run_basic(source)` — compile and simulate a BASIC program with default options
  (4 096-word GE-225 memory, 100 000 instruction safety limit).
- `run_basic_with_options(source, memory_words, max_steps)` — explicit control over
  memory size and step limit.
- `RunResult` — structured output containing:
  - `output` — typewriter characters produced by `PRINT` statements
    (GE-225 carriage-return codes converted to Unix `\n`).
  - `var_values` — final 20-bit values of BASIC scalar variables A–Z,
    sign-extended to `i32`.
  - `steps` — number of GE-225 instructions executed.
  - `halt_address` — word address of the halt self-loop stub.
- `BasicError` — wraps failures from all four pipeline stages (parse, IR compile,
  GE-225 codegen, simulation) into a single error type.
- 36 integration tests covering:
  - `LET` (constant, arithmetic, chained variables, unary minus, complex expressions)
  - `PRINT` (strings, numbers, leading-zero suppression, negative numbers, mixed)
  - `FOR … TO [STEP] … NEXT` loops (sequence, sum, stride-2)
  - `IF … THEN` conditionals (< > = <> <=)
  - `GOTO`
  - Classic programs: Fibonacci, countdown, for-loop sum
  - Error cases: GOSUB unsupported, max-steps exceeded
  - `RunResult` field checks (steps > 0, halt_address > 0)
- Pipeline stages:
  1. `dartmouth-basic-ir-compiler` with `int_bits=20` (GE-225 20-bit word size).
  2. `ir-to-ge225-compiler` GE-225 backend.
  3. `coding-adventures-ge225-simulator` behavioural simulator.

### Fixed (discovered during integration testing)

- **GE-225 branch-test "inhibit" semantics** — The GE-225 uses *inhibit* semantics
  for conditional skip instructions (the named condition **prevents** the skip, not
  causes it). `ir-to-ge225-compiler` was using the wrong instruction in four places:

  | Site | Old instruction | Correct instruction |
  |------|-----------------|---------------------|
  | `BRANCH_Z` (jump when A==0) | `BNZ` | `BZE` |
  | `BRANCH_NZ` (jump when A≠0) | `BZE` | `BNZ` |
  | `CMP_EQ` (equal → result 1) | `BNZ` | `BZE` |
  | `CMP_NE` (not-equal → result 1) | `BZE` | `BNZ` |
  | `CMP_LT`/`CMP_GT` signed diff | `BPL` (skips when A<0) | `BMI` (skips when A≥0) |
  | `AND_IMM 1` odd-path layout | LDO at +3, LDZ at +5 | LDZ at +3, LDO at +5 |

  The double-inversion in CMP+BRANCH_NZ accidentally produced correct results for
  `IF … THEN` tests, masking the bug. BRANCH_Z with arithmetic (leading-zero
  suppression in `emit_print_number`) and NOT-bool via `AND_IMM` exposed it.
