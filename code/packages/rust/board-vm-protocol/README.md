# board-vm-protocol

`no_std` frame and payload codecs for the Board VM binary protocol.

The crate owns the byte-exact pieces shared by firmware runtimes and host SDKs:
ULEB128, CRC-16/CCITT-FALSE, COBS stream framing, raw frame parsing, and compact
payload encoders for the first interactive hardware flows.

Golden vectors such as `GOLDEN_HELLO_PAYLOAD_BVM_V1`,
`GOLDEN_HELLO_RAW_FRAME_BVM_V1`, and `GOLDEN_HELLO_WIRE_FRAME_BVM_V1` are kept in
the crate so non-Rust SDKs can assert byte-for-byte compatibility without
depending on the Rust implementation.
