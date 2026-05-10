# Board VM TCP Transport

`board-vm-tcp` connects host clients to Board VM firmware over a TCP byte
stream. It reuses the shared COBS/CRC stream adapter, so Wi-Fi transports carry
the same Rust-owned binary protocol frames as USB/serial.
