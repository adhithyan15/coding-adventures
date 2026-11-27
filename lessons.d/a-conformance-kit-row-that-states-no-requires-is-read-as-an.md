---
category: Testing & coverage
---

# A conformance-kit row that states no requires is read as an S5 row, so a later-slice row that needs only ir.core must state it

S10d's kit tables mark a row of the earlier slice by leaving `Row.requires` unset, and `finish_case` then demands that the
oracle compute `["ir.core"]`.  A row of a later slice whose computed `requires` really is `["ir.core"]` (a `nat_shift`
descriptor, a `type` refusal, an `Int` model parameter, a chunk corpus part) passes that check without saying anything, so
33 of the 61 such rows were silently taken for S5 rows: the build was green, and only the test that splits S5 from S10d by
`Row.requires` noticed.  State `requires=["ir.core"]` on every such row, and keep a test that the unset rows are exactly the
first N rows of each table, with a digest of what they derive.
