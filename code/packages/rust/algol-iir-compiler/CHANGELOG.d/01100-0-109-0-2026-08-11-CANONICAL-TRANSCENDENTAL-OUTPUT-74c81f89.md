## 0.109.0 — 2026-08-11 — canonical transcendental output

The formatter-free static real path now recognizes the exact identities
`sin(0)=0`, `cos(0)=1`, `ln(1)=0`, `exp(0)=1`, and `arctan(0)=0` without
invoking host or backend math libraries. Other transcendental inputs continue
to require runtime real formatting and fail closed.

