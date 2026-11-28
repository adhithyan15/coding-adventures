# storage-crc32c

A dependency-free reference implementation of reflected CRC-32C using the Castagnoli polynomial. `Checksum` starts from a zero seed; `Update` continues an existing checksum and uses fixed scratch space without taking ownership of the input bytes. Callers must keep input bytes stable during each call.

Run `sh BUILD` from this directory. The Windows command list is in `BUILD_windows`.
