cargo test --locked -p spice-mosaic-app -- --nocapture
rustup target add wasm32-unknown-unknown
RUSTC="$(rustup which rustc)" cargo build --locked -p spice-mosaic-app --target wasm32-unknown-unknown
node --test js/wasm.test.mjs
