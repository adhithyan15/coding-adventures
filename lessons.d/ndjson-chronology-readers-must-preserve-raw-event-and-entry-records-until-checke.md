---
category: Compiler / VM / language pipeline
---

# NDJSON chronology readers must preserve raw event and entry records until checked reconstruction

Three full-suite failures came from old readers treating _event frames as
CVEntry objects or expecting an obsolete partial-import diagnostic. Reconstruct
canonical NDJSON with separate entry/event phases and a final state-only footer.
Preserve raw bodies through the final bounded checked parser, so escaped duplicate
fields cannot disappear through Value normalization. Bound input, record counts,
line work and assembled bytes. A metadata id must not choose a graph entry key.
Test duplicate entry/event/footer evidence and reject records after the footer;
document the helper as a test adapter, not a production NDJSON import API.
