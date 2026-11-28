# storage-record-frame

A dependency-light DBRF v1 framing layer for opaque storage payloads. Frames include a fixed header, little-endian payload length and CRC-32C trailer. `Encode` and `Decode` return owned byte slices; callers keep input slices stable during each call.

Run `sh BUILD` from this directory. The Windows command list is in `BUILD_windows`.
