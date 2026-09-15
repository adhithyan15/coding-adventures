---
category: Compiler / VM / language pipeline
---

# Fresh VM context per call

Same applies in any VM where the outer loop reads pc/code from VM state — re-read both on each step if handlers can swap them.
