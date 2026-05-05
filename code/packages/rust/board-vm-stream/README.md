# board-vm-stream

`std::io` transport adapter for Board VM host clients.

The crate wraps any caller-supplied `Read + Write` byte stream, encodes outgoing
raw protocol frames as COBS-terminated wire frames, and decodes incoming wire
frames back into raw frames for `board-vm-client`.

This is intentionally below any OS serial dependency. A real UART, USB CDC,
TCP tunnel, test harness, or simulator link can own device setup and timeouts,
then hand its byte stream to `StreamTransport`.
