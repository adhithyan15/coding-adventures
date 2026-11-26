## [Unreleased] - 2026-11-24

Ships with the first published release (0.1.0). The OCAML07 representative-chain
contract pins every representative package at 0.1.0, so no version bump is
claimed here.

### Changed

- All 11 basic gates validate every argument and report the first invalid
  parameter under their own name; `nand_or`/`nand_xor`/`nand_and` now validate
  up front so errors never name an inner construction wire (`nand_or 1 2`
  reports `b`).
- Tests: literal truth tables for all 11 gates, NAND-built equals direct, and
  exhaustive rejection over [-1024,1024] plus min_int/max_int extremes in every
  argument position.
- README documents input validation and compile-time rejection of non-int types.

### Security (LGE-OCAML-HARDEN, LGE-DOS-ALL)

- Size caps are enforced before any allocation: `demux ~n_outputs`,
  `register`/`shift_register` widths and every input/state list are capped at
  65536; `counter ~width` at 64; `decoder` stays at 16 input bits. Over-cap
  requests return `Invalid_width` (the reported value is the requested size, or
  the lower bound 65537 for an over-long list). `shift_register ~width:max_int`
  now fails immediately instead of exhausting memory.
- Hostile lists: all list lengths are measured with a bounded walk, so a cyclic
  list (`let rec l = 1 :: l`) is rejected promptly rather than looping forever
  in `validate_bits`, the n-ary gates, `mux_n`, `mux8`, `decoder`, `encoder`,
  `priority_encoder`, `register`, `shift_register` or `counter`.
- Total functions: `log2_exact` is total for values <= 1; `List.nth`,
  `List.hd`, `List.tl` and `assert false` are replaced by pattern matches that
  return errors; `increment_lsb` is iterative (constant stack).
- `shift_register` validates the caller's flip-flop snapshot first, so a corrupt
  `slave_q` is reported as `flip-flop state` instead of being misattributed to
  `data`.
- Ordering change: the size and length check now runs before bit validation.
  For `mux_n`, `demux`, `decoder`, `register` (and the other list-taking
  circuits) an oversized or mis-sized input reports the length or width error
  rather than `Invalid_bit`.
- Tests: cap boundaries, cyclic lists, former partial
  paths, shift_register attribution, wide ints on all non-bit positions.
