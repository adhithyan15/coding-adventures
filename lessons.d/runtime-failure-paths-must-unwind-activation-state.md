---
category: Compiler / VM / language pipeline
---

# Runtime failure paths must unwind activation state

Inside a procedure, an array-bounds or heap-exhaustion guard that just `RET`s skips frame/heap restoration normally done by the success path. Add cleanup symmetric with the success return.
