## 0.88.0 — 2026-08-11 — procedure heading validation

Procedure headings now fail closed before IIR lowering when the formal list,
`value` part, or specification parts are inconsistent. Formal names and value
names must be unique, every value/specification name must belong to the formal
list, and each formal must receive exactly one specification. Valid headings
with multiple specification groups retain the existing typed direct-call ABI.

