## 0.4.0

- **Breaking:** every rejected gate argument now raises one identity,
  `CodingAdventures::LogicGates::InvalidBitError` (a subclass of
  `ArgumentError`) with `param` (the offending parameter name) and
  `value_description` (the integer itself for an Integer, otherwise
  "an instance of <ClassName>", capped at 64 characters; the value's own
  `to_s`/`inspect` is never called). Previously a non-Integer raised
  `TypeError`; `rescue TypeError` no longer catches a rejected bit, while
  `rescue ArgumentError` still does.
- A bit is valid iff it is the Integer 0 or 1, tested with `Integer === v`
  so objects defining `to_int`, `coerce`, `==` or `class` cannot pass.
  Rejected: true/false, nil, 1.0, Rational(1), Complex(1, 0), BigDecimal,
  strings, symbols, arrays, hashes, arbitrary objects.
- `nand_gate`, `nor_gate`, `xnor_gate` validate their own arguments instead
  of relying on the transistors validator, and `nand_not`/`nand_and`/
  `nand_or`/`nand_xor` validate before delegating, so the error always names
  `a` or `b` and never an inner NAND parameter.
- New `test/test_gates_exhaustive.rb`: literal truth tables for all 11
  gates, NAND-built equals direct, every Integer in -1024..1024 (except 0, 1)
  plus 2**31, 2**63, 2**64, 10**100 and negatives, and a type-confusion
  matrix, in every argument position, plus arity checks.
- Sequential and combinational validators are unchanged (still `TypeError`).
- Package is now `standardrb` clean (behaviour-neutral: gemspec spacing, a conditional assignment in `combinational.rb`, an unused local in `sequential.rb`).
- `value_description` for an Integer wider than 64 bits is now "an Integer of N bits" instead of the full decimal string, so a huge bignum cannot cost superlinear CPU or build a huge message. Removed a dead allocation in `sequential.rb` (shift register).
