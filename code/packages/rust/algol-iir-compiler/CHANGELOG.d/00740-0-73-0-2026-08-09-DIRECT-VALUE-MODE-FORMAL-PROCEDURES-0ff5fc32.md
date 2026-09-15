## 0.73.0 — 2026-08-09 — direct value-mode formal procedures

`value procedure p` now accepts the same direct, statically resolvable actuals
as a name-mode formal. Specialisation retains the direct target rather than
materialising a procedure value in the IIR ABI, including when a value-mode
formal is forwarded into another direct wrapper. Dynamic procedure values
remain unsupported.

