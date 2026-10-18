---
category: Cross-platform & Windows BUILD_windows
---

# On Windows, `closesocket` with an overlapped read pending resets the connection, and the peer loses unread data

`websocket-runtime`'s integration test sent a Close frame and waited for the
server's Close echo. On Linux and macOS it passed; on `windows-2025` the
client got `Tcp(ConnectionReset)` instead of the frame.

The server flushed the echo and then called `close_stream`, whose IOCP
implementation just dropped the `TcpStream`. The provider re-arms an
overlapped `WSARecv` after every read, so one was pending at that moment, and
Winsock makes `closesocket()` *abortive* (RST, not FIN) in that state. A peer
that receives an RST discards whatever it had not read yet, including the
frame that was sent a moment earlier. Linux keeps already-received data
readable, so the bug never showed there.

**Fix:** `shutdown(SD_SEND)` before dropping the socket. That queues a FIN
behind the flushed data, and the close stays graceful even with the read
pending. libuv's `uv_tcp_close` does exactly this for the same reason.

**Rule:** on Windows, "write then close" is only reliable as "write, shut
down for sending, then close". A test that writes, closes, and checks the
peer reads the bytes followed by EOF (`close_after_write_delivers_data_then_eof`
in `transport-platform`) catches this on every backend.

Found only because an unrelated change pulled `transport-platform`'s
dependents into a PR's Windows build; see
`build-windows-drifts-from-build-silently-because-almost-no-pr-runs-the.md`
for why Windows-only bugs sit unnoticed.
