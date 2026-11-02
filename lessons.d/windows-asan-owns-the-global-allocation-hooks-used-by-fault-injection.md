---
category: Testing & coverage
---

# Windows ASan owns the global allocation hooks used by fault injection

NN51's Windows LLVM ASan link rejected duplicate replacement new/delete symbols
from the deterministic fault-injection test and the ASan dynamic runtime. No
sanitized executable ran in that attempt. Keep all normal GCC/Clang allocation
failure sweeps, but build the separate ASan variant without replacement hooks
and with NN51_NO_ALLOCATION_FAULTS for only that incompatible test. The native
library, checkpoint corpus, ownership, boundary, gradient and XOR tests remain
instrumented. Report the different check counts explicitly; do not suppress
the linker error or call the failed link a sanitizer pass.
