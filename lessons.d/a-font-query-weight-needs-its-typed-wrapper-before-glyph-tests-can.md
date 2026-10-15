---
category: Rust
---

# A font-query weight needs its typed wrapper before glyph tests can run

The first Railroad labels test commit4206b9f0 compiled layout tests, but paint tests failed E0308 before execution: FontQuery.weight uses FontWeight, while the label font uses a numeric weight. Use FontWeight::BOLD in resolver-query goldens. Preserve the failed compilation receipt, then rerun and require actual glyph failures or passes; compilation failure is not evidence of the intended runtime regression.
