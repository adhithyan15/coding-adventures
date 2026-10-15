---
category: Cross-platform & Windows BUILD_windows
---

# A complete dependency cone still needs explicit host applicability evidence

A fresh reverse cone added the older tcp-reactor through IOCP/native-event-core.
The first full Windows command failed before any target ran: tcp-reactor
explicitly refuses non-Unix hosts and uses Unix file descriptors. Metadata
membership alone did not imply Windows execution support.

Preserve that actual compile failure and the complete cone. Prove the Unix-only
package is unchanged, run all other applicable Windows packages, and keep actual
Unix validation pending for its supported hosts. Do not call the full cone passed,
invent a Windows port, or reduce required Unix/hosted checks to hide the failure.
