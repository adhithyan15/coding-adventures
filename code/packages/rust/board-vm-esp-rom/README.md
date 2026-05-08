# board-vm-esp-rom

Rust-owned ESP ROM bootloader protocol helpers for Board VM.

This is the first slice of the zero outside dependency ESP flashing path. It
implements the built-in ROM protocol pieces needed to identify ESP-family chips
without shelling out to `esptool` or `espflash`:

- SLIP frame encoding and decoding.
- ROM command packet construction.
- `SYNC`, `READ_REG`, and `GET_SECURITY_INFO` exchanges.
- chip ID / magic-value mapping to `Xtensa` or `RISC-V`.

The immediate use is target auto-detection: a host can query the ESP ROM loader,
identify the exact chip family, and select the right Board VM runtime/backend and
compiler target.

Backlog:

1. Add flash attach/set-params and erase/write/verify commands.
2. Add image layout helpers for Board VM ESP firmware artifacts.
3. Replace temporary `esptool`/`espflash` compatibility calls in smoke helpers.
4. Share target detection through Ruby, Python, Lua, and future language sugar
   via the Rust bridge packages.

Host-side validation:

```sh
cargo test -p board-vm-esp-rom
```
