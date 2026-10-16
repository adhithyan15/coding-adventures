---
category: Rust
---

# A binary IR serializer cannot prove side-table lineage

Independent native18 test-first review withheld the complete-module assertion:
the binary serializer omits source_map, feedback/refinement tables and some
static instruction fields. Keep exact wire-byte comparison and add exhaustive
in-memory module/function/instruction companions without a rest pattern, plus
source-map lockstep and real-location witnesses. Do not change shared production
wire format to make an incomplete test appear complete.
