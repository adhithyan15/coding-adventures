cargo test -p spice-mosaic-app -- --nocapture
RUSTC="$(rustup which rustc)" rustup run stable cargo build -p spice-mosaic-app --target wasm32-unknown-unknown
node --test js/wasm.test.mjs
