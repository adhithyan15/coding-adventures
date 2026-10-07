---
category: Rust
---

# A socket accepted from a non-blocking listener is itself non-blocking on macOS but not on Linux

`forme-shell-desktop`'s preview server sets its `TcpListener` non-blocking so
its accept loop can poll a shutdown flag. On macOS (and the BSDs) the stream
`accept()` returns inherits `O_NONBLOCK` from the listener; on Linux, Rust's
`accept4` gives a blocking stream. The request read therefore returned
`WouldBlock` on macOS whenever the test client had not yet written. The server
dropped the connection, and the client's `write!` failed with `BrokenPipe`
(EPIPE). That made "Forme release (macos)" fail some runs and pass others. It
went red on `main` and on unrelated PRs, and nothing in Forme had changed. It
could never reproduce on a Linux machine.

Fix: call `stream.set_nonblocking(false)` on every accepted stream before
setting read and write timeouts. Those timeouts only apply to a blocking
socket.

Do differently: whenever a listener is set non-blocking, set the blocking mode
of each accepted stream explicitly; never rely on the platform default. When a
socket test fails only on macOS with `WouldBlock`, `BrokenPipe` or a reset
connection, check for this first.
