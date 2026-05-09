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

The higher-level connection helpers are still just Lua sugar over Rust-owned
metadata. Lua asks the native bridge for known targets, discovered devices, and
connection options, then keeps the chosen values on a small `Connection` object:

```lua
local board = board_vm.connect("esp32")
print(board:board_id())              -- esp32-devkit-v1
print(board:connection_transport())  -- serial

local wifi_board = board_vm.connect("uno-r4-wifi", {
    via = "Wi-Fi",
    transport = wifi_endpoint,
})
wifi_board:smoke()
```

`via` accepts friendly names such as `serial`, `Wi-Fi`, and `BLE`. Wireless
choices require an injected Lua transport object with `write(frame)` or
`transact(frame, options)` until repo-native Wi-Fi/Bluetooth host endpoints are
implemented; the module will not silently fall back to serial for those paths.
