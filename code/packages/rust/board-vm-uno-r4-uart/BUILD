cargo test -p board-vm-uno-r4-uart -- --nocapture
rustup target add thumbv7em-none-eabihf
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf -p board-vm-uno-r4-uart
