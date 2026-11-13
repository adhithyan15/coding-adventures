//go:build cgo && (linux || darwin || (windows && amd64))

package paintcodecpngnative

// The Rust crate code/packages/rust/paint-codec-png-c must be built before this
// package compiles; its BUILD / BUILD_windows recipes (and the build tool's
// `deps=rust/paint-codec-png-c` directive) take care of the ordering.
//
// Each OS links the Rust library differently, so each gets its own LDFLAGS line
// in the cgo preamble below.  Everything else in this file is shared.
//
//   - darwin (arm64 verified; amd64 NOT verified locally, it reuses the arm64
//     flags and is exercised only by CI): `-L... -lpaint_codec_png_c` against the
//     host-built target/release.  These flags are the ones that have always
//     worked on macOS arm64; do not change them without a Mac to test on.
//
//   - linux: the staticlib by full path (so the linker cannot pick the cdylib
//     `.so` that `cargo build` also writes into the same directory), plus the
//     system libraries `cargo rustc ... -- --print native-static-libs` reports
//     for the crate on Rust 1.100.0: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc.
//
//   - windows: Go's cgo drives MinGW-w64 gcc, which needs a GNU-ABI archive, so
//     the recipe (tools\run-tests.ps1) builds the staticlib for
//     x86_64-pc-windows-gnu and this line names it by full path.  The libraries
//     are what rustc reports for the GNU target for this crate (-lkernel32
//     -lntdll -luserenv -lws2_32 -ldbghelp -lsynchronization) plus -lgcc_eh for
//     the _Unwind_* symbols and -static so libwinpthread is not a runtime DLL
//     dependency.  Same recipe as go/conduit.

/*
#cgo CFLAGS: -I${SRCDIR}/../../rust/paint-codec-png-c/include
#cgo darwin LDFLAGS: -L${SRCDIR}/../../rust/target/release -lpaint_codec_png_c
#cgo linux LDFLAGS: ${SRCDIR}/../../rust/target/release/libpaint_codec_png_c.a -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
#cgo windows LDFLAGS: ${SRCDIR}/../../rust/target/x86_64-pc-windows-gnu/release/libpaint_codec_png_c.a -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -lsynchronization -lgcc_eh -static
#include "paint_codec_png_c.h"
*/
import "C"

import (
	"fmt"
	"unsafe"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// nativeLinked is true: this build links the Rust codec.  Keep the tag above the
// exact negation of the tag in paint_codec_png_native_stub.go.
const nativeLinked = true

// encodeNative calls the C ABI.  Encode has already checked that pixels is
// non-nil, consistent and non-empty, so &pixels.Data[0] is valid.
func encodeNative(pixels *pixelcontainer.PixelContainer) ([]byte, error) {
	var outBytes C.paint_encoded_bytes_t
	result := C.paint_codec_png_encode_rgba8(
		C.uint32_t(pixels.Width),
		C.uint32_t(pixels.Height),
		(*C.uint8_t)(unsafe.Pointer(&pixels.Data[0])),
		C.size_t(len(pixels.Data)),
		&outBytes,
	)
	if result != 1 || outBytes.data == nil {
		return nil, fmt.Errorf("native PNG codec failed")
	}
	defer C.paint_codec_png_free_bytes(outBytes.data, outBytes.len)

	return C.GoBytes(unsafe.Pointer(outBytes.data), C.int(outBytes.len)), nil
}
