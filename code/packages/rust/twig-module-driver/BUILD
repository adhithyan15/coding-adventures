# The self-compile tests interpret the Twig compiler, which recurses deeply.
# libtest's default 2 MiB test-thread stack overflows on macOS arm64 debug
# builds, so give every test thread 64 MiB (the deepest tests still spawn
# their own 768 MiB / 3 GiB threads via run_in_large_stack / run_in_xlarge_stack).
RUST_MIN_STACK=67108864 cargo test -p twig-module-driver -- --nocapture
