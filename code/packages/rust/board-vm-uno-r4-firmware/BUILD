cargo test -p board-vm-uno-r4-firmware -- --nocapture
RUSTC="$(rustup which --toolchain stable rustc)" cargo run -p board-vm-uno-r4-firmware --bin uno-r4-wifi-serialusb-artifact -- --print-only --core "$HOME/Library/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3" --arm-toolchain-bin /opt/homebrew/bin --bossac-path /tmp/arduino-bossa/bin --port /dev/cu.usbmodem... --upload --smoke
rustup target add thumbv7em-none-eabihf
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf -p board-vm-uno-r4-firmware --lib
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf -p board-vm-uno-r4-firmware --bin uno-r4-wifi-uart-server
