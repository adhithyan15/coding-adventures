---
category: Compiler / VM / language pipeline
---

# Preserve generic diagnostic stages when adding domain gates

The first full D3a compiler suite hit a real regression: NONE's existing generic
work-limit rejection moved from validate stage to source_domain because the new
gate wrapped both graph and shape errors alike. Keep the original failing full
receipt; do not relax its diagnostic or budget test. Classify generic and shape
errors privately so NONE retains validate and serialized formats retain snapshot,
while new shape/configuration refusals use source_domain. Validate configuration
before generic work and preserve the public compiler signature and fixed defaults.

The companion source-transfer helper initially compared current files against
D2's tested source60e rather than its published Markdown carrier7e1. README and
CHANGELOG therefore appeared as two unexpected deltas. Bind the baseline to the
actual published tree and prove those two accepted carrier files against its
source-transfer receipt; do not widen a four-Rust-file assertion indiscriminately.
