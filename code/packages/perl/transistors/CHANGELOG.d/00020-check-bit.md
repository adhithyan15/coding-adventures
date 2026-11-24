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
- The cmos_* and ttl_* gates enforce arity with `_args` (leading invocant stripped only when it is exactly the package name); `cmos_nand(0, 1, 1)` and `cmos_not(0, 1)` now die instead of silently dropping or misreading an argument.

- BREAKING: class-method call style removed from all cmos_*/ttl_* functions; a leading package-name argument is data and is rejected. Version bumped to 0.02.
- `nmos`, `pmos`, `npn` and `pnp` no longer guess an invocant from `/::/` in the first argument (any value containing `::` used to be shifted off, and `nmos()` ran a regex on undef). They enforce arity 1 to 2 with `_args` and validate every analog argument with `_num`: defined, not a reference, strict decimal number, finite; the result is a fresh number computed from the validated string. An explicit undef for the optional second argument still means "use the default".
