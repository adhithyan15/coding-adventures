# board-vm-fixed-stream

Reusable no-heap byte stream for Board VM tests and firmware probes.

`FixedByteStream` feeds a fixed input byte slice into `DeviceStreamEndpoint` and
captures the response bytes into a fixed output buffer. It is useful for
scripted hardware probes, simulator harnesses, and unit tests that should not
carry a board-specific fake stream.
