# board_vm_native (Lua)

Lua sugar over the Rust-owned Board VM protocol builders.

The Lua module does not hand-roll framing or bytecode. It loads a Rust C
extension backed by `board-vm-language-core`, then exposes a small `Session`
object that tracks request ids while Rust builds the COBS-framed wire messages
and BVM modules.

```lua
local board_vm = require("coding_adventures.board_vm_native")

local session = board_vm.session()
local frames = session:blink_upload_run_frames({
    pin = 13,
    high_ms = 250,
    low_ms = 250,
})

local uno = board_vm.detect_target("uno-r4-wifi")
print(uno.connection_options[1].display_name) -- USB/serial

local devices = board_vm.devices({
    "/dev/tty.usbserial-CP2102-esp32",
})
print(devices[1].target.board_id) -- esp32-devkit-v1
```
