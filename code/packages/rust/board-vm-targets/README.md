# board-vm-targets

Host-side Board VM target registry for supported boards.

The registry gives generic front ends and language bindings one stable place to
ask which boards are known, what runtime id they use, what Rust target they
compile for, and where their onboard LED lives. Board-specific runtime crates
remain responsible for HAL behavior and pin validation.

Host-side validation:

```sh
cargo test -p board-vm-targets
```
