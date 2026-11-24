## [Unreleased]

### Security

- The CMOS gate functions (`cmos_not`, `cmos_nand`, `cmos_nor`, `cmos_and`,
  `cmos_or`, `cmos_xor`, `cmos_xnor` and the TTL helpers) validated inputs with
  numeric `!=`, which numifies strings (`"1abc"` passed as 1; `"a"`, `""`,
  `undef` passed as 0). They now use `_check_bit`: defined, not a reference,
  and exactly `"0"` or `"1"`. Error messages are unchanged.
