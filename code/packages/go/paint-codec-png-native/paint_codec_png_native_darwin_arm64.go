//go:build darwin && arm64

package paintcodecpngnative

/*
#cgo CFLAGS: -I${SRCDIR}/../../rust/paint-codec-png-c/include
#cgo LDFLAGS: -L${SRCDIR}/../../rust/target/release -lpaint_codec_png_c
#include "paint_codec_png_c.h"
*/
import "C"

import (
	"fmt"
	"unsafe"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func encode(pixels *pixelcontainer.PixelContainer) ([]byte, error) {
	if pixels == nil {
		return nil, fmt.Errorf("pixel container must not be nil")
	}
	if err := pixelcontainer.Validate(pixels); err != nil {
		return nil, err
	}
	if len(pixels.Data) == 0 {
		return nil, fmt.Errorf("pixel container must not be empty")
	}

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
