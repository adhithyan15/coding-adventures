//go:build windows

package paintvmgdidirect

import (
	"fmt"
	"syscall"
	"unsafe"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

const (
	biRGB        = 0
	dibRGBColors = 0
)

type colorRef uint32
type winHandle uintptr

type bitmapInfoHeader struct {
	Size          uint32
	Width         int32
	Height        int32
	Planes        uint16
	BitCount      uint16
	Compression   uint32
	SizeImage     uint32
	XPelsPerMeter int32
	YPelsPerMeter int32
	ClrUsed       uint32
	ClrImportant  uint32
}

type rgbQuad struct {
	Blue     byte
	Green    byte
	Red      byte
	Reserved byte
}

type bitmapInfo struct {
	Header bitmapInfoHeader
	Colors [1]rgbQuad
}

type rect struct {
	Left   int32
	Top    int32
	Right  int32
	Bottom int32
}

var (
	gdi32                  = syscall.NewLazyDLL("gdi32.dll")
	user32                 = syscall.NewLazyDLL("user32.dll")
	procCreateCompatibleDC = gdi32.NewProc("CreateCompatibleDC")
	procCreateDIBSection   = gdi32.NewProc("CreateDIBSection")
	procSelectObject       = gdi32.NewProc("SelectObject")
	procDeleteObject       = gdi32.NewProc("DeleteObject")
	procDeleteDC           = gdi32.NewProc("DeleteDC")
	procCreateSolidBrush   = gdi32.NewProc("CreateSolidBrush")
	procFillRect           = user32.NewProc("FillRect")
)

func render(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	if scene.Width <= 0 || scene.Height <= 0 {
		return nil, fmt.Errorf("scene dimensions must be positive")
	}

	hdc, err := createCompatibleDC()
	if err != nil {
		return nil, err
	}
	defer deleteDC(hdc)

	bmi := bitmapInfo{
		Header: bitmapInfoHeader{
			Size:        uint32(unsafe.Sizeof(bitmapInfoHeader{})),
			Width:       int32(scene.Width),
			Height:      -int32(scene.Height),
			Planes:      1,
			BitCount:    32,
			Compression: biRGB,
		},
	}

	var bits unsafe.Pointer
	bitmap, err := createDIBSection(hdc, &bmi, &bits)
	if err != nil {
		return nil, err
	}
	defer deleteObject(bitmap)

	oldObject, err := selectObject(hdc, bitmap)
	if err != nil {
		return nil, err
	}
	defer func() {
		if oldObject != 0 {
			_, _ = selectObject(hdc, oldObject)
		}
	}()

	if err := fillRect(hdc, 0, 0, scene.Width, scene.Height, scene.Background); err != nil {
		return nil, err
	}

	for _, instruction := range scene.Instructions {
		rectInstruction, ok := instruction.(paintinstructions.PaintRectInstruction)
		if !ok {
			return nil, fmt.Errorf("only rect paint instructions are supported right now")
		}
		if err := fillRect(
			hdc,
			rectInstruction.X,
			rectInstruction.Y,
			rectInstruction.Width,
			rectInstruction.Height,
			rectInstruction.Fill,
		); err != nil {
			return nil, err
		}
	}

	raw := unsafe.Slice((*byte)(bits), scene.Width*scene.Height*4)
	pixels := pixelcontainer.New(uint32(scene.Width), uint32(scene.Height))
	for i := 0; i < len(raw); i += 4 {
		pixels.Data[i] = raw[i+2]
		pixels.Data[i+1] = raw[i+1]
		pixels.Data[i+2] = raw[i]
		// GDI leaves the alpha byte unset for DIB-backed fills, so normalize
		// the exported pixel buffer to fully opaque RGBA like the Rust backend.
		pixels.Data[i+3] = 255
	}
	return pixels, nil
}

func createCompatibleDC() (winHandle, error) {
	result, _, callErr := procCreateCompatibleDC.Call(0)
	if result == 0 {
		return 0, callErr
	}
	return winHandle(result), nil
}

func createDIBSection(hdc winHandle, bmi *bitmapInfo, bits *unsafe.Pointer) (winHandle, error) {
	result, _, callErr := procCreateDIBSection.Call(
		uintptr(hdc),
		uintptr(unsafe.Pointer(bmi)),
		uintptr(dibRGBColors),
		uintptr(unsafe.Pointer(bits)),
		0,
		0,
	)
	if result == 0 {
		return 0, callErr
	}
	return winHandle(result), nil
}

func selectObject(hdc winHandle, object winHandle) (winHandle, error) {
	result, _, callErr := procSelectObject.Call(uintptr(hdc), uintptr(object))
	if result == 0 {
		return 0, callErr
	}
	return winHandle(result), nil
}

func deleteObject(handle winHandle) {
	if handle != 0 {
		procDeleteObject.Call(uintptr(handle)) //nolint:errcheck
	}
}

func deleteDC(handle winHandle) {
	if handle != 0 {
		procDeleteDC.Call(uintptr(handle)) //nolint:errcheck
	}
}

func fillRect(hdc winHandle, x int, y int, width int, height int, fill string) error {
	color, err := paintinstructions.ParseColorRGBA8(fill)
	if err != nil {
		return err
	}

	brush, err := createSolidBrush(toColorRef(color))
	if err != nil {
		return err
	}
	defer deleteObject(brush)

	r := rect{Left: int32(x), Top: int32(y), Right: int32(x + width), Bottom: int32(y + height)}
	result, _, callErr := procFillRect.Call(
		uintptr(hdc),
		uintptr(unsafe.Pointer(&r)),
		uintptr(brush),
	)
	if result == 0 {
		return callErr
	}
	return nil
}

func createSolidBrush(color colorRef) (winHandle, error) {
	result, _, callErr := procCreateSolidBrush.Call(uintptr(color))
	if result == 0 {
		return 0, callErr
	}
	return winHandle(result), nil
}

func toColorRef(color paintinstructions.PaintColorRGBA8) colorRef {
	return colorRef(uint32(color.R) | uint32(color.G)<<8 | uint32(color.B)<<16)
}
