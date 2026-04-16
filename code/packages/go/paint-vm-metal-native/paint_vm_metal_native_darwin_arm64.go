//go:build darwin && arm64

package paintvmmetalnative

/*
#cgo CFLAGS: -I${SRCDIR}/../../rust/paint-vm-metal-c/include
#cgo LDFLAGS: -L${SRCDIR}/../../rust/target/release -lpaint_vm_metal_c -framework Metal -framework CoreGraphics -framework CoreText -framework CoreFoundation -framework AppKit -lobjc
#include "paint_vm_metal_c.h"
*/
import "C"

import (
	"fmt"
	"unsafe"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func render(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	rects := make([]C.paint_rect_instruction_t, 0, len(scene.Instructions))
	for _, instruction := range scene.Instructions {
		rect, ok := instruction.(paintinstructions.PaintRectInstruction)
		if !ok {
			return nil, fmt.Errorf("only rect paint instructions are supported right now")
		}

		fill, err := paintinstructions.ParseColorRGBA8(rect.Fill)
		if err != nil {
			return nil, err
		}
		rects = append(rects, C.paint_rect_instruction_t{
			x:      C.uint32_t(rect.X),
			y:      C.uint32_t(rect.Y),
			width:  C.uint32_t(rect.Width),
			height: C.uint32_t(rect.Height),
			fill: C.paint_rgba8_color_t{
				r: C.uint8_t(fill.R),
				g: C.uint8_t(fill.G),
				b: C.uint8_t(fill.B),
				a: C.uint8_t(fill.A),
			},
		})
	}

	background, err := paintinstructions.ParseColorRGBA8(scene.Background)
	if err != nil {
		return nil, err
	}

	var rectPointer *C.paint_rect_instruction_t
	if len(rects) > 0 {
		rectPointer = &rects[0]
	}

	var outBuffer C.paint_rgba8_buffer_t
	result := C.paint_vm_metal_render_rect_scene(
		C.uint32_t(scene.Width),
		C.uint32_t(scene.Height),
		C.paint_rgba8_color_t{
			r: C.uint8_t(background.R),
			g: C.uint8_t(background.G),
			b: C.uint8_t(background.B),
			a: C.uint8_t(background.A),
		},
		rectPointer,
		C.size_t(len(rects)),
		&outBuffer,
	)
	if result != 1 || outBuffer.data == nil {
		return nil, fmt.Errorf("native Metal Paint VM failed")
	}
	defer C.paint_vm_metal_free_buffer_data(outBuffer.data, outBuffer.len)

	pixels := pixelcontainer.New(uint32(outBuffer.width), uint32(outBuffer.height))
	copy(pixels.Data, C.GoBytes((unsafe.Pointer(outBuffer.data)), C.int(outBuffer.len)))
	return pixels, nil
}
