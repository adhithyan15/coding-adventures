//go:build !(cgo && (linux || darwin || (windows && amd64)))

package paintcodecpngnative

import (
	"fmt"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// nativeLinked is false: this build (CGO_ENABLED=0, or an OS/architecture the
// Rust static library is not built for) has no native codec.  Keep the tag above
// the exact negation of the tag in paint_codec_png_native_cgo.go.
const nativeLinked = false

func encodeNative(_ *pixelcontainer.PixelContainer) ([]byte, error) {
	return nil, fmt.Errorf("paint-codec-png-native is not available on this host")
}
