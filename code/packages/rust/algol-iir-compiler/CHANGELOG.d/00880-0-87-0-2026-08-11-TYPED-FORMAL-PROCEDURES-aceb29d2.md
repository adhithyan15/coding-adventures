## 0.87.0 — 2026-08-11 — typed formal procedures

Report-style typed procedure formals such as `integer procedure p` now retain
their expected result type during direct call-site specialization. Declared and
forwarded procedure actuals plus supported standard functions are checked
before lowering; proper procedures and mismatched result types fail closed.
The static direct-call IIR ABI is unchanged.

