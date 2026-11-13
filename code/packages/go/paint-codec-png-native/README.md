# paint-codec-png-native

Go wrapper over the Rust `paint-codec-png-c` crate (C ABI): encode an RGBA8
`pixel-container` to PNG bytes through cgo.

```go
import (
    native "github.com/adhithyan15/coding-adventures/code/packages/go/paint-codec-png-native"
    pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

pixels := pixelcontainer.New(1, 1)
pixelcontainer.SetPixel(pixels, 0, 0, 0, 0, 0, 255)
if native.Available() {
    png, err := native.Encode(pixels) // starts with the 8-byte PNG signature
    _ = png
    _ = err
}
```

## Where it runs

| Host | Native codec | How the Rust library is linked |
| --- | --- | --- |
| Linux (cgo) | yes, tested in CI | `target/release/libpaint_codec_png_c.a` by full path plus `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |
| macOS arm64 (cgo) | yes, tested in CI | `-L target/release -lpaint_codec_png_c` |
| macOS amd64 (cgo) | yes, same flags as arm64 (not run locally) | same as arm64 |
| Windows amd64 (cgo + MinGW-w64) | yes, tested in CI | the `x86_64-pc-windows-gnu` staticlib by full path plus `-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -lsynchronization -lgcc_eh -static` |
| anything else, or `CGO_ENABLED=0` | no: `Available()` is false and `Encode` returns an error | none |

`SupportedRuntime()` and `Available()` both report one constant, `nativeLinked`,
defined by the build tags of `paint_codec_png_native_cgo.go` (true) and
`paint_codec_png_native_stub.go` (false), so what they say cannot disagree with
what was linked. `Encode` validates its argument in Go (nil, inconsistent length
and empty containers are rejected on every host) before calling the C ABI.

## Building and testing

The Rust crate must be built first (`deps=rust/paint-codec-png-c` in `BUILD`):

- Linux / macOS: `BUILD` runs `cargo build --release` in the crate and then
  `CGO_ENABLED=1 go test ./... -v -cover`.
- Windows: `BUILD_windows` runs `tools\run-tests.ps1`, which uses the shared
  `code\scripts\build-conduit-capi-windows-gnu.ps1` helper
  (`-CrateDir paint-codec-png-c -ArchiveName libpaint_codec_png_c.a`) to build
  the GNU-ABI staticlib, put a MinGW-w64 `gcc` on `PATH`, and then runs the same
  `go test`. cgo on Windows drives MinGW gcc, never `cl.exe`, which is why the
  MSVC `.lib` is not used. See `code/specs/BUILD-002-native-support-on-every-os.md`.
