---
category: Native extensions & FFI
---

# Closing a Windows socket does not complete the lifetime of its pending operation buffers

Independent review of imported Windows transport found that closing a socket
requested cancellation while default provider destruction freed the operation's
OVERLAPPED and buffer. Keeping storage in a map during close_stream only protects
it while the provider itself remains alive.

The repair contract requires a destructor completion barrier: close all owned
sockets, retain the port and operation allocations, then release each allocation
only after its matching terminal packet. A timeout is not a completion. Fatal
wait errors require explicit retention, never fabricated success. AcceptEx BOOL
must also be checked correctly so failed submissions cannot become phantom
pending operations. Verify actual terminal accounting through the same helper
used by destruction, and distinguish static review from executed UB evidence.
