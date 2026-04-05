// Package pixelcontainer_test exercises every exported function in the
// pixelcontainer package.
//
// Tests are organised in three groups:
//  1. Construction — New() creates valid, zero-filled containers.
//  2. PixelAt / SetPixel — reads and writes including out-of-bounds behaviour.
//  3. FillPixels — bulk-fill and dimension checks.
package pixelcontainer_test

import (
	"testing"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// ── Construction ──────────────────────────────────────────────────────────────

// TestNew_Dimensions checks that Width and Height are stored correctly.
func TestNew_Dimensions(t *testing.T) {
	img := pc.New(10, 20)
	if img.Width != 10 {
		t.Errorf("Width: got %d, want 10", img.Width)
	}
	if img.Height != 20 {
		t.Errorf("Height: got %d, want 20", img.Height)
	}
}

// TestNew_DataLength verifies that the backing slice is exactly W*H*4 bytes.
func TestNew_DataLength(t *testing.T) {
	img := pc.New(7, 3)
	want := 7 * 3 * 4
	if len(img.Data) != want {
		t.Errorf("len(Data): got %d, want %d", len(img.Data), want)
	}
}

// TestNew_ZeroFilled checks that all bytes start at zero.
func TestNew_ZeroFilled(t *testing.T) {
	img := pc.New(4, 4)
	for i, b := range img.Data {
		if b != 0 {
			t.Errorf("Data[%d] = %d, want 0", i, b)
		}
	}
}

// TestNew_1x1 checks a single-pixel image.
func TestNew_1x1(t *testing.T) {
	img := pc.New(1, 1)
	if len(img.Data) != 4 {
		t.Errorf("1×1 image should have 4 bytes, got %d", len(img.Data))
	}
}

// TestNew_LargeImage ensures no panic on a large allocation.
func TestNew_LargeImage(t *testing.T) {
	img := pc.New(1920, 1080)
	want := uint32(1920)
	if img.Width != want {
		t.Errorf("Width: got %d, want %d", img.Width, want)
	}
	if uint64(len(img.Data)) != uint64(1920)*uint64(1080)*4 {
		t.Error("Data length mismatch for 1920x1080 image")
	}
}

// ── PixelAt ──────────────────────────────────────────────────────────────────

// TestPixelAt_ZeroAfterNew confirms all pixels read as (0,0,0,0) after New.
func TestPixelAt_ZeroAfterNew(t *testing.T) {
	img := pc.New(5, 5)
	r, g, b, a := pc.PixelAt(img, 2, 3)
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("PixelAt on new image: got (%d,%d,%d,%d), want (0,0,0,0)", r, g, b, a)
	}
}

// TestPixelAt_OutOfBoundsX checks that reading past the right edge returns zeros.
func TestPixelAt_OutOfBoundsX(t *testing.T) {
	img := pc.New(4, 4)
	r, g, b, a := pc.PixelAt(img, 4, 0) // x == Width is out of bounds
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("out-of-bounds x: got (%d,%d,%d,%d), want (0,0,0,0)", r, g, b, a)
	}
}

// TestPixelAt_OutOfBoundsY checks that reading past the bottom edge returns zeros.
func TestPixelAt_OutOfBoundsY(t *testing.T) {
	img := pc.New(4, 4)
	r, g, b, a := pc.PixelAt(img, 0, 4) // y == Height is out of bounds
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("out-of-bounds y: got (%d,%d,%d,%d), want (0,0,0,0)", r, g, b, a)
	}
}

// TestPixelAt_FarOutOfBounds uses large coordinates to confirm no panic.
func TestPixelAt_FarOutOfBounds(t *testing.T) {
	img := pc.New(4, 4)
	r, g, b, a := pc.PixelAt(img, 9999, 9999)
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("far out-of-bounds: got (%d,%d,%d,%d), want (0,0,0,0)", r, g, b, a)
	}
}

// ── SetPixel ─────────────────────────────────────────────────────────────────

// TestSetPixel_Basic writes a pixel and reads it back.
func TestSetPixel_Basic(t *testing.T) {
	img := pc.New(8, 8)
	pc.SetPixel(img, 3, 2, 10, 20, 30, 255)
	r, g, b, a := pc.PixelAt(img, 3, 2)
	if r != 10 || g != 20 || b != 30 || a != 255 {
		t.Errorf("SetPixel/PixelAt: got (%d,%d,%d,%d), want (10,20,30,255)", r, g, b, a)
	}
}

// TestSetPixel_AllChannels verifies each channel is stored independently.
func TestSetPixel_AllChannels(t *testing.T) {
	img := pc.New(3, 3)
	pc.SetPixel(img, 0, 0, 1, 2, 3, 4)
	r, g, b, a := pc.PixelAt(img, 0, 0)
	if r != 1 {
		t.Errorf("R: got %d, want 1", r)
	}
	if g != 2 {
		t.Errorf("G: got %d, want 2", g)
	}
	if b != 3 {
		t.Errorf("B: got %d, want 3", b)
	}
	if a != 4 {
		t.Errorf("A: got %d, want 4", a)
	}
}

// TestSetPixel_MaxValues checks 255 in every channel (maximum RGBA values).
func TestSetPixel_MaxValues(t *testing.T) {
	img := pc.New(2, 2)
	pc.SetPixel(img, 1, 1, 255, 255, 255, 255)
	r, g, b, a := pc.PixelAt(img, 1, 1)
	if r != 255 || g != 255 || b != 255 || a != 255 {
		t.Errorf("max values: got (%d,%d,%d,%d), want (255,255,255,255)", r, g, b, a)
	}
}

// TestSetPixel_DoesNotClobberNeighbour sets one pixel and confirms the
// adjacent pixel is unaffected.
func TestSetPixel_DoesNotClobberNeighbour(t *testing.T) {
	img := pc.New(4, 4)
	pc.SetPixel(img, 1, 1, 100, 100, 100, 100)
	r, g, b, a := pc.PixelAt(img, 2, 1) // right neighbour
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("neighbour contaminated: got (%d,%d,%d,%d), want (0,0,0,0)", r, g, b, a)
	}
}

// TestSetPixel_OutOfBoundsX ensures no panic and no write for x == Width.
func TestSetPixel_OutOfBoundsX(t *testing.T) {
	img := pc.New(4, 4)
	// Should be a no-op — must not panic.
	pc.SetPixel(img, 4, 0, 99, 99, 99, 99)
	// Verify data is still all zeros.
	for _, b := range img.Data {
		if b != 0 {
			t.Error("out-of-bounds SetPixel wrote to Data")
			break
		}
	}
}

// TestSetPixel_OutOfBoundsY ensures no panic and no write for y == Height.
func TestSetPixel_OutOfBoundsY(t *testing.T) {
	img := pc.New(4, 4)
	pc.SetPixel(img, 0, 4, 99, 99, 99, 99)
	for _, b := range img.Data {
		if b != 0 {
			t.Error("out-of-bounds SetPixel wrote to Data")
			break
		}
	}
}

// TestSetPixel_OverwriteExisting verifies a pixel can be updated.
func TestSetPixel_OverwriteExisting(t *testing.T) {
	img := pc.New(3, 3)
	pc.SetPixel(img, 0, 0, 10, 20, 30, 40)
	pc.SetPixel(img, 0, 0, 50, 60, 70, 80) // overwrite
	r, g, b, a := pc.PixelAt(img, 0, 0)
	if r != 50 || g != 60 || b != 70 || a != 80 {
		t.Errorf("overwrite: got (%d,%d,%d,%d), want (50,60,70,80)", r, g, b, a)
	}
}

// TestSetPixel_BottomRightCorner exercises the last pixel in the buffer.
func TestSetPixel_BottomRightCorner(t *testing.T) {
	img := pc.New(5, 5)
	pc.SetPixel(img, 4, 4, 11, 22, 33, 44)
	r, g, b, a := pc.PixelAt(img, 4, 4)
	if r != 11 || g != 22 || b != 33 || a != 44 {
		t.Errorf("bottom-right corner: got (%d,%d,%d,%d), want (11,22,33,44)", r, g, b, a)
	}
}

// ── FillPixels ───────────────────────────────────────────────────────────────

// TestFillPixels_White fills with white and checks every pixel.
func TestFillPixels_White(t *testing.T) {
	img := pc.New(3, 3)
	pc.FillPixels(img, 255, 255, 255, 255)
	for y := uint32(0); y < img.Height; y++ {
		for x := uint32(0); x < img.Width; x++ {
			r, g, b, a := pc.PixelAt(img, x, y)
			if r != 255 || g != 255 || b != 255 || a != 255 {
				t.Errorf("pixel (%d,%d): got (%d,%d,%d,%d), want (255,255,255,255)", x, y, r, g, b, a)
			}
		}
	}
}

// TestFillPixels_Transparent fills with transparent black.
func TestFillPixels_Transparent(t *testing.T) {
	img := pc.New(2, 2)
	// First paint something
	pc.FillPixels(img, 100, 100, 100, 100)
	// Then reset to transparent black
	pc.FillPixels(img, 0, 0, 0, 0)
	for _, b := range img.Data {
		if b != 0 {
			t.Errorf("expected all zeros after fill(0,0,0,0), got %d", b)
			break
		}
	}
}

// TestFillPixels_DistinctChannels fills with unequal channel values and
// checks every pixel to make sure no channel bleeds into another.
func TestFillPixels_DistinctChannels(t *testing.T) {
	img := pc.New(4, 3)
	pc.FillPixels(img, 11, 22, 33, 44)
	for y := uint32(0); y < img.Height; y++ {
		for x := uint32(0); x < img.Width; x++ {
			r, g, b, a := pc.PixelAt(img, x, y)
			if r != 11 || g != 22 || b != 33 || a != 44 {
				t.Errorf("pixel (%d,%d): got (%d,%d,%d,%d), want (11,22,33,44)", x, y, r, g, b, a)
			}
		}
	}
}

// ── Validate ─────────────────────────────────────────────────────────────────

// TestValidate_Valid checks that a properly constructed container passes.
func TestValidate_Valid(t *testing.T) {
	img := pc.New(10, 10)
	if err := pc.Validate(img); err != nil {
		t.Errorf("Validate on valid container: %v", err)
	}
}

// TestValidate_TruncatedData checks that a container with a short Data slice fails.
func TestValidate_TruncatedData(t *testing.T) {
	img := pc.New(4, 4)
	img.Data = img.Data[:10] // deliberately wrong length
	if err := pc.Validate(img); err == nil {
		t.Error("Validate should have returned an error for truncated Data")
	}
}
