# build-tool: deps=rust/paint-codec-png-c
# Linux and macOS: build the Rust C-ABI crate, then run the real cgo tests.
# (Windows has its own recipe in BUILD_windows / tools\run-tests.ps1.)
(cd ../../rust/paint-codec-png-c && cargo build --release) && CGO_ENABLED=1 go test ./... -v -cover
