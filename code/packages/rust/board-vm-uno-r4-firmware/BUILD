cargo test -p board-vm-uno-r4-firmware -- --nocapture
RUSTC="$(rustup which rustc)" rustup run stable cargo run -p board-vm-uno-r4-firmware --bin uno-r4-wifi-serialusb-artifact -- --print-only --port /dev/cu.usbmodem... --upload --smoke
rustup target add thumbv7em-none-eabihf
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf -p board-vm-uno-r4-firmware --lib
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf -p board-vm-uno-r4-firmware --bin uno-r4-wifi-uart-server
