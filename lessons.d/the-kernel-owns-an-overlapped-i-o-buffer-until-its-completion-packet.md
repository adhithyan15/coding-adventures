---
category: Cross-platform & Windows BUILD_windows
---

# The kernel owns an overlapped I/O buffer until its completion packet is dequeued, even after the socket is closed

A Windows reactor posts `WSARecv`/`WSASend`/`AcceptEx` with a buffer and an `OVERLAPPED`
record and gets them back from the completion port later. From the moment an
operation is posted until its completion packet has been *dequeued*, the kernel owns
that buffer and record. Closing the socket does not end that ownership: it cancels the
operation, but the cancelled operation still completes (with `ERROR_OPERATION_ABORTED`)
and the kernel may write into the buffer until then. Freeing it on `close` is a
use-after-free that the kernel performs for you, silently, and it survives a green test
run because nothing reads the freed memory.

What `tcp-reactor`'s Windows engine does, and what to do in any overlapped-I/O code:

- Keep every operation in a heap record with a stable address, with `OVERLAPPED` as the
  first field of a `#[repr(C)]` struct, and recover the record from the packet pointer.
- Track `in_flight` per operation. A closing connection stays in the table, with no new
  operations posted, until both its read and write operations have completed; only then
  free its records.
- If a drain deadline passes with operations still outstanding, leak them. A leak is
  harmless; freeing under the kernel is not.
- Do not set `FILE_SKIP_COMPLETION_PORT_ON_SUCCESS` unless you handle the synchronous
  path too: without it every successful post still queues a packet, so there is one
  completion path to reason about.
- An overlapped stream send completes whole or with an error, so a "partial send" branch
  cannot be provoked by a test. Mutate it away to confirm which branches your tests reach.

Validated by cross-compiling to `x86_64-pc-windows-gnu` and running under Wine, which
implements IOCP and `AcceptEx`.
