# coding_adventures_board_vm

Ruby DSL for Board VM hardware sessions.

Ruby is syntax sugar over the Rust Board VM host core. It should not carry its
own implementation of request ids, CRCs, COBS framing, payload encoders, or
binary message layouts. Those pieces live in Rust crates, with
`board-vm-language-core` providing the shared Rust boundary that native bridge
packages wrap. Ruby should go through a `ruby-bridge` extension, Python through
`python-bridge`, and other languages through their repo-local bridge packages.

The first target is the Arduino Uno R4 WiFi. The DSL keeps the frontend shape
language-level and board-agnostic, while the implementation delegates the
board-specific pieces to the Rust packages that already know how to build,
flash, and speak the Board VM protocol.

```ruby
require "coding_adventures_board_vm"

CodingAdventures::BoardVM.uno_r4_wifi(
  port: "/dev/cu.usbmodem1101",
  flash: true,
  arduino_core: "#{Dir.home}/Library/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3",
  arm_toolchain_bin: "/opt/homebrew/bin",
  bossac_path: "/tmp/arduino-bossa/bin"
) do |board|
  board.led.blink
end
```

`flash: true` builds and uploads the Rust SerialUSB Board VM server firmware by
running the Uno R4 artifact helper:

```sh
cargo run -p board-vm-uno-r4-firmware --bin uno-r4-wifi-serialusb-artifact -- --upload ...
```

The helper output is inspected for Arduino CLI's `New upload port:` handoff so
subsequent Ruby DSL commands use the runtime CDC port when the board
re-enumerates.

The gem now ships a `board_vm_native` extension built with `ruby-bridge`. That
extension exposes `CodingAdventures::BoardVM::Native::Session`, whose methods
return binary Ruby strings produced by `board-vm-language-core`:

```ruby
session = CodingAdventures::BoardVM::Native::Session.new
frames = session.blink_upload_run_frames(1, 12, 13, 250, 250, 4)
```

Each element in `frames` is a COBS/CRC-framed Board VM request ready for a
transport to write to the board. Ruby owns the friendly shape; Rust owns the
wire bytes.

Inside the block, `board.led.blink` currently runs the shared Rust host smoke
command:

```sh
cargo run -p board-vm-cli --bin board-vm -- smoke ...
```

That smoke command sends `HELLO`, queries capabilities, uploads the standard
blink module, and starts it through the binary protocol. The next Ruby step is
to route `board.led.blink` through `BoardVM::Native::Session` plus a transport
instead of through this subprocess bridge. The DSL surface should stay
Ruby-shaped, but every board operation should still dispatch the Board VM
binary protocol produced by Rust.

The DSL also exposes the current board-agnostic blink ejection path:

```ruby
CodingAdventures::BoardVM.uno_r4_wifi(port: "/dev/cu.usbmodem1101") do |board|
  board.eject.blink(to: "src/ejected_blink.rs", boot_policy: :run_if_no_host)
end
```

That writes Rust constants for the blink MVP. Board-specific firmware, such as
the Uno R4 ejected runner, decides how to validate and execute those constants.
