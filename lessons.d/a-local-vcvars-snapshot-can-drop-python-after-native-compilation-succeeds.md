---
category: Cross-platform & Windows BUILD_windows
---

# A local vcvars snapshot can drop Python after native compilation succeeds

NN48's actual Windows BUILD front compiled and executed the library suite and
native XOR under cl and clang-cl, then failed to locate Python for its command
validator. This local vcvars snapshot replaces PATH and had dropped the host
Python directory; it is not a numerical or package-source failure. Capture the
trusted installed Python's parent before importing vcvars and prepend it to
the process-only PATH afterward, alongside LLVM and the existing SDK bootstrap.
Do not commit machine paths or weaken the BUILD gate. Rerunning the unchanged
front completed both full command/resume validators and regression tests.
