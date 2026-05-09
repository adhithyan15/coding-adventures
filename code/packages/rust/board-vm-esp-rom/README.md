# board-vm-esp-rom

Rust-owned ESP ROM bootloader protocol helpers for Board VM.

This is the first slice of the zero outside dependency ESP flashing path. It
implements the built-in ROM protocol pieces needed to identify ESP-family chips
without shelling out to `esptool` or `espflash`:

- SLIP frame encoding and decoding.
- ROM command packet construction.
- `SYNC`, `READ_REG`, and `GET_SECURITY_INFO` exchanges.
- chip ID / magic-value mapping to `Xtensa` or `RISC-V`.
- flash attach, flash parameter, erase/write, finish, and MD5 verify command
  builders.
- ESP boot image header, segment table, padding, and checksum builders for
  Board VM firmware artifacts.
- high-level image upload that erases, writes padded ROM blocks, verifies the
  ROM MD5 against the repo's Rust MD5 implementation, and only then exits the
  bootloader.

The immediate use is target auto-detection: a host can query the ESP ROM loader,
identify the exact chip family, and select the right Board VM runtime/backend and
compiler target.

Backlog:

1. Replace temporary `esptool`/`espflash` compatibility calls in smoke helpers
   with the native upload path.
2. Share target detection through Ruby, Python, Lua, and future language sugar
   via the Rust bridge packages.
3. Add end-to-end ESP upload helpers once the Board VM firmware image producer is
   ready to hand artifacts to this crate.

Host-side validation:

```sh
cargo test -p board-vm-esp-rom
```
