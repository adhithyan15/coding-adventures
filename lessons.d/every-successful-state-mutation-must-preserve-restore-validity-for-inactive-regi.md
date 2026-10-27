---
category: Compiler / VM / language pipeline
---

# Every successful state mutation must preserve restore validity for inactive register banks

Validating only the currently selected stack pointer lets a limit write or
special-register instruction create a snapshot that the public restore path
later rejects. Treat restore validity as a closure property: every committed
mutation must validate every pointer/limit pair in the affected bank, including
inactive pointers. Seed the inactive pair near its limit and require both
direct setters and inherited instruction execution to reject and roll back the
complete state.
