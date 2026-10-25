## 0.603.0 - 2026-10-25

### Added

- LANG84 phase 7c slice 2: Python 2 integer literals and four more releases. `--lang python@2.4` and `python@2.6` select `py2-int-print`, and `python@3.5.10` and `python@3.6.0` select `py-int-print`. On every Python 2 release the subset reads octal (`010`), hex (`0x1F`) and `L`-suffixed literals, and from 2.6 on also `0o`/`0b`. `python_release_requests_select_record_and_refuse_exactly` checks that `print 06 * 0x7L` lowers to the same instructions as `print(6 * 7)` on all four Python 2 releases.
- The release sweep now gives Python four IIR-identity classes, and `lang_matrix` follows its representatives:
  - The 18 rows that ran on 3.12.3 now run on 3.6.0, the new earliest member of the class that accepts the whole `py-int-print` corpus. 3.5.10 joins 3.0a1's class.
  - Python 2 splits on `pep3127_literals` into {2.2, 2.4} and {2.6, 2.7.18}. 2.2 gains 4 rows: `octal_literals`, `long_suffix`, `hex_literals` and `overflow_long_literal`. 2.6 gets the whole 24-case corpus, ten rows of which are `StdoutThenTrap`.
  - Python is now 100 rows / 800 cells (LANG-VM-FEATURE-COVERAGE.md).
  - `release_inventory.py check --matrix` reports Python 11 claims: 100 cells direct, 118 by IIR class, 4 not applicable.
