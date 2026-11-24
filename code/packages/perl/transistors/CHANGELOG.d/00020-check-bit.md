## [Unreleased]

### Security

- The CMOS gate functions (`cmos_not`, `cmos_nand`, `cmos_nor`, `cmos_and`,
  `cmos_or`, `cmos_xor`, `cmos_xnor` and the TTL helpers) validated inputs with
  numeric `!=`, which numifies strings (`"1abc"` passed as 1; `"a"`, `""`,
  `undef` passed as 0). They now use `_check_bit`: defined, not a reference,
  and exactly `"0"` or `"1"`. Error messages are unchanged.
- `_check_bit` now returns a fresh canonical integer 0/1 and the gates compute
  on it, so `dualvar(5, "1")` (string "1", number 5) can no longer pass
  validation and then compute as 5.
