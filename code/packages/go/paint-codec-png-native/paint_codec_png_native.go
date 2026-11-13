// Package paintcodecpngnative encodes RGBA8 pixel containers to PNG by calling
// the reviewed Rust `paint-codec-png-c` C ABI through cgo.
//
// # Where the native codec exists
//
// Which hosts link the Rust static library is decided in exactly one place:
// the build tags of the two files that define `nativeLinked`.
//
//	paint_codec_png_native_cgo.go   cgo && (linux || darwin || (windows && amd64))
//	                                nativeLinked = true
//	paint_codec_png_native_stub.go  everything else, including CGO_ENABLED=0
//	                                nativeLinked = false
//
// SupportedRuntime and Available both report that constant, so the answer a
// caller sees can never drift from what the linker actually did.
package paintcodecpngnative

import (
	"fmt"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

const Version = "0.1.0"

// SupportedRuntime reports whether this build links the native PNG codec.
func SupportedRuntime() bool {
	return nativeLinked
}

// Available reports whether Encode can produce PNG bytes on this host.
func Available() bool {
	return nativeLinked
}

// Encode returns the PNG encoding of pixels (RGBA8, row-major, top-left origin).
//
// Argument validation happens here, once, in Go, so every host rejects bad input
// the same way; only a well-formed container ever reaches the C ABI.
func Encode(pixels *pixelcontainer.PixelContainer) ([]byte, error) {
	if pixels == nil {
		return nil, fmt.Errorf("pixel container must not be nil")
	}
	if err := pixelcontainer.Validate(pixels); err != nil {
		return nil, err
	}
	if len(pixels.Data) == 0 {
		return nil, fmt.Errorf("pixel container must not be empty")
	}
	return encodeNative(pixels)
}
