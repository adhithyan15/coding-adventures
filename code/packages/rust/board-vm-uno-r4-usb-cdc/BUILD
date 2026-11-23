cargo test --locked -p board-vm-uno-r4-usb-cdc -- --nocapture
rustup target add thumbv7em-none-eabihf
RUSTC="$(rustup which rustc)" cargo build --locked --target thumbv7em-none-eabihf -p board-vm-uno-r4-usb-cdc
