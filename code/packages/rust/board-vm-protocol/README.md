# board-vm-protocol

`no_std` frame and payload codecs for the Board VM binary protocol.

The crate owns the byte-exact pieces shared by firmware runtimes and host SDKs:
ULEB128, CRC-16/CCITT-FALSE, COBS stream framing, raw frame parsing, and compact
payload encoders for the first interactive hardware flows.
