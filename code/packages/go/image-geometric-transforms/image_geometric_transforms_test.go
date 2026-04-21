// Package imagegeometrictransforms_test contains the unit test suite for IMG04.
//
// Test philosophy:
//   - Lossless operations (flip, rotate 90/180, crop, pad) are verified by
//     checking exact byte equality after round-trip inversions.
//   - Continuous operations (scale, rotate arbitrary angle, affine, perspective)
//     are verified approximately: the identity transform should reproduce the
//     source image to within ±2 per channel to allow for floating-point rounding.
//   - Sampling tests verify mathematical properties of the interpolation kernels
//     directly, independent of the transform wrappers.
package imagegeometrictransforms_test

import (
	"math"
	"testing"

	igt "github.com/adhithyan15/coding-adventures/code/packages/go/image-geometric-transforms"
	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// ── Helpers ───────────────────────────────────────────────────────────────────

// newSolid creates a w×h image filled with a single RGBA colour.
func newSolid(w, h uint32, r, g, b, a byte) *pc.PixelContainer {
	img := pc.New(w, h)
	for i := 0; i < len(img.Data); i += 4 {
		img.Data[i] = r
		img.Data[i+1] = g
		img.Data[i+2] = b
		img.Data[i+3] = a
	}
	return img
}

// imagesEqual returns true when every channel of every pixel is equal.
func imagesEqual(a, b *pc.PixelContainer) bool {
	if a.Width != b.Width || a.Height != b.Height {
		return false
	}
	for i := range a.Data {
		if a.Data[i] != b.Data[i] {
			return false
		}
	}
	return true
}

// imagesNearlyEqual returns true when every channel of every pixel differs by
// at most tol.  Used for transforms that pass through floating-point arithmetic.
func imagesNearlyEqual(a, b *pc.PixelContainer, tol int) bool {
	if a.Width != b.Width || a.Height != b.Height {
		return false
	}
	for i := range a.Data {
		diff := int(a.Data[i]) - int(b.Data[i])
		if diff < -tol || diff > tol {
			return false
		}
	}
	return true
}

// newGradient creates a w×h image where R = x, G = y, B = 0, A = 255.
// Useful because each pixel has a unique colour tied to its position.
func newGradient(w, h uint32) *pc.PixelContainer {
	img := pc.New(w, h)
	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			// Clamp x, y to byte range for the test image.
			rv := byte(x % 256)
			gv := byte(y % 256)
			pc.SetPixel(img, x, y, rv, gv, 0, 255)
		}
	}
	return img
}

// absInt returns the absolute value of an integer.
func absInt(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

// ── FlipHorizontal ────────────────────────────────────────────────────────────

// TestFlipHorizontalPixelPosition verifies that the pixel at (0, 0) of the
// flipped image equals the pixel at (W-1, 0) of the source image.
func TestFlipHorizontalPixelPosition(t *testing.T) {
	src := newGradient(5, 3)
	got := igt.FlipHorizontal(src)
	wantR, wantG, wantB, wantA := pc.PixelAt(src, src.Width-1, 0)
	gotR, gotG, gotB, gotA := pc.PixelAt(got, 0, 0)
	if gotR != wantR || gotG != wantG || gotB != wantB || gotA != wantA {
		t.Errorf("FlipHorizontal: pixel(0,0)=(%d,%d,%d,%d) want (%d,%d,%d,%d)",
			gotR, gotG, gotB, gotA, wantR, wantG, wantB, wantA)
	}
}

// TestFlipHorizontalDimensions confirms that FlipHorizontal preserves image size.
func TestFlipHorizontalDimensions(t *testing.T) {
	src := newGradient(7, 4)
	got := igt.FlipHorizontal(src)
	if got.Width != src.Width || got.Height != src.Height {
		t.Errorf("FlipHorizontal: dimensions %dx%d want %dx%d", got.Width, got.Height, src.Width, src.Height)
	}
}

// TestFlipHorizontalDoubleIdentity applies FlipHorizontal twice and expects the
// result to be pixel-perfect equal to the source.
func TestFlipHorizontalDoubleIdentity(t *testing.T) {
	src := newGradient(6, 4)
	got := igt.FlipHorizontal(igt.FlipHorizontal(src))
	if !imagesEqual(src, got) {
		t.Error("FlipHorizontal applied twice should return the original image")
	}
}

// ── FlipVertical ─────────────────────────────────────────────────────────────

// TestFlipVerticalPixelPosition verifies the pixel at (0, 0) of the flipped
// image equals the pixel at (0, H-1) of the source.
func TestFlipVerticalPixelPosition(t *testing.T) {
	src := newGradient(5, 5)
	got := igt.FlipVertical(src)
	wantR, wantG, wantB, wantA := pc.PixelAt(src, 0, src.Height-1)
	gotR, gotG, gotB, gotA := pc.PixelAt(got, 0, 0)
	if gotR != wantR || gotG != wantG || gotB != wantB || gotA != wantA {
		t.Errorf("FlipVertical: pixel(0,0)=(%d,%d,%d,%d) want (%d,%d,%d,%d)",
			gotR, gotG, gotB, gotA, wantR, wantG, wantB, wantA)
	}
}

// TestFlipVerticalDoubleIdentity applies FlipVertical twice and expects the
// result to be pixel-perfect equal to the source.
func TestFlipVerticalDoubleIdentity(t *testing.T) {
	src := newGradient(5, 7)
	got := igt.FlipVertical(igt.FlipVertical(src))
	if !imagesEqual(src, got) {
		t.Error("FlipVertical applied twice should return the original image")
	}
}

// ── Rotate90CW / Rotate90CCW ──────────────────────────────────────────────────

// TestRotate90CWDimensions checks that Rotate90CW swaps width and height.
func TestRotate90CWDimensions(t *testing.T) {
	src := newGradient(8, 3)
	got := igt.Rotate90CW(src)
	if got.Width != src.Height || got.Height != src.Width {
		t.Errorf("Rotate90CW: dimensions %dx%d want %dx%d", got.Width, got.Height, src.Height, src.Width)
	}
}

// TestRotate90CCWDimensions checks that Rotate90CCW swaps width and height.
func TestRotate90CCWDimensions(t *testing.T) {
	src := newGradient(8, 3)
	got := igt.Rotate90CCW(src)
	if got.Width != src.Height || got.Height != src.Width {
		t.Errorf("Rotate90CCW: dimensions %dx%d want %dx%d", got.Width, got.Height, src.Height, src.Width)
	}
}

// TestRotate90CWThenCCWIsIdentity rotates 90° CW then 90° CCW and checks that
// the round-trip recovers the original image exactly.
func TestRotate90CWThenCCWIsIdentity(t *testing.T) {
	src := newGradient(5, 7)
	got := igt.Rotate90CCW(igt.Rotate90CW(src))
	if !imagesEqual(src, got) {
		t.Error("Rotate90CW followed by Rotate90CCW should be the identity")
	}
}

// TestRotate90CWFourTimesIsIdentity verifies that four 90° CW rotations return
// the original image (360° rotation is the identity).
func TestRotate90CWFourTimesIsIdentity(t *testing.T) {
	src := newGradient(4, 6)
	got := igt.Rotate90CW(igt.Rotate90CW(igt.Rotate90CW(igt.Rotate90CW(src))))
	if !imagesEqual(src, got) {
		t.Error("Four Rotate90CW rotations should return the original image")
	}
}

// TestRotate90CWPixelCorner checks a known corner pixel after a CW rotation.
// The top-left pixel of the source should appear at the top-right of the output.
//
// For a 4×3 source rotated CW:
//   - Output dimensions: 3×4 (W'=H_src=3, H'=W_src=4)
//   - Source (0,0) should end up at output (H-1, 0) = (2, 0) after CW rotation.
//
// Forward CW: (x,y) → (H-1-y, x).  So source (0,0) → (H-1, 0) = (2, 0).
func TestRotate90CWPixelCorner(t *testing.T) {
	src := newGradient(4, 3)
	got := igt.Rotate90CW(src)
	// Source top-left (0,0) should land at output (H_src-1, 0) = (2, 0)
	srcR, srcG, srcB, srcA := pc.PixelAt(src, 0, 0)
	dstR, dstG, dstB, dstA := pc.PixelAt(got, src.Height-1, 0)
	if srcR != dstR || srcG != dstG || srcB != dstB || srcA != dstA {
		t.Errorf("Rotate90CW corner: got (%d,%d,%d,%d) want (%d,%d,%d,%d)",
			dstR, dstG, dstB, dstA, srcR, srcG, srcB, srcA)
	}
}

// ── Rotate180 ─────────────────────────────────────────────────────────────────

// TestRotate180DimensionsPreserved checks that Rotate180 keeps the image size.
func TestRotate180DimensionsPreserved(t *testing.T) {
	src := newGradient(5, 7)
	got := igt.Rotate180(src)
	if got.Width != src.Width || got.Height != src.Height {
		t.Errorf("Rotate180: dimensions changed to %dx%d", got.Width, got.Height)
	}
}

// TestRotate180TwiceIsIdentity applies Rotate180 twice and verifies the result
// is pixel-perfect equal to the source.
func TestRotate180TwiceIsIdentity(t *testing.T) {
	src := newGradient(6, 4)
	got := igt.Rotate180(igt.Rotate180(src))
	if !imagesEqual(src, got) {
		t.Error("Rotate180 applied twice should return the original image")
	}
}

// TestRotate180EqualsFlipBoth verifies that Rotate180 is equivalent to applying
// FlipHorizontal and FlipVertical in sequence.
func TestRotate180EqualsFlipBoth(t *testing.T) {
	src := newGradient(5, 5)
	via180 := igt.Rotate180(src)
	viaFlips := igt.FlipHorizontal(igt.FlipVertical(src))
	if !imagesEqual(via180, viaFlips) {
		t.Error("Rotate180 should equal FlipHorizontal(FlipVertical(src))")
	}
}

// ── Crop ──────────────────────────────────────────────────────────────────────

// TestCropDimensions verifies that Crop produces exactly the requested size.
func TestCropDimensions(t *testing.T) {
	src := newGradient(20, 15)
	got := igt.Crop(src, 2, 3, 8, 5)
	if got.Width != 8 || got.Height != 5 {
		t.Errorf("Crop: dimensions %dx%d want 8x5", got.Width, got.Height)
	}
}

// TestCropPixelContent verifies that pixels within the crop region are taken
// from the correct source positions.
func TestCropPixelContent(t *testing.T) {
	src := newGradient(20, 15)
	x0, y0 := uint32(3), uint32(4)
	got := igt.Crop(src, x0, y0, 5, 5)
	// Pixel (0,0) in crop should equal pixel (x0, y0) in source.
	sr, sg, sb, sa := pc.PixelAt(src, x0, y0)
	gr, gg, gb, ga := pc.PixelAt(got, 0, 0)
	if sr != gr || sg != gg || sb != gb || sa != ga {
		t.Errorf("Crop pixel (0,0): got (%d,%d,%d,%d) want (%d,%d,%d,%d)",
			gr, gg, gb, ga, sr, sg, sb, sa)
	}
}

// TestCropOutOfBoundsFilledWithZero verifies that out-of-bounds crop areas are
// filled with transparent black.
func TestCropOutOfBoundsFilledWithZero(t *testing.T) {
	src := newSolid(4, 4, 200, 100, 50, 255)
	// Request a crop that extends beyond the source boundary.
	got := igt.Crop(src, 2, 2, 6, 6)
	// Pixel (5, 5) is completely outside the source.
	r, g, b, a := pc.PixelAt(got, 5, 5)
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("Crop OOB pixel: got (%d,%d,%d,%d) want (0,0,0,0)", r, g, b, a)
	}
}

// ── Pad ───────────────────────────────────────────────────────────────────────

// TestPadDimensions verifies the output is sized to src + border.
func TestPadDimensions(t *testing.T) {
	src := newSolid(10, 8, 100, 100, 100, 255)
	got := igt.Pad(src, 2, 3, 4, 5, igt.Rgba8{0, 0, 0, 255})
	wantW := uint32(5 + 10 + 3)
	wantH := uint32(2 + 8 + 4)
	if got.Width != wantW || got.Height != wantH {
		t.Errorf("Pad: dimensions %dx%d want %dx%d", got.Width, got.Height, wantW, wantH)
	}
}

// TestPadFillColor checks that border pixels carry the fill colour.
func TestPadFillColor(t *testing.T) {
	fill := igt.Rgba8{R: 255, G: 128, B: 64, A: 200}
	src := newSolid(4, 4, 10, 20, 30, 255)
	got := igt.Pad(src, 3, 3, 3, 3, fill)
	// Top-left corner pixel should be the fill colour.
	r, g, b, a := pc.PixelAt(got, 0, 0)
	if r != fill.R || g != fill.G || b != fill.B || a != fill.A {
		t.Errorf("Pad fill corner: got (%d,%d,%d,%d) want (%d,%d,%d,%d)",
			r, g, b, a, fill.R, fill.G, fill.B, fill.A)
	}
}

// TestPadOriginalPreserved checks that the interior pixels match the source.
func TestPadOriginalPreserved(t *testing.T) {
	src := newGradient(4, 4)
	top, left := uint32(2), uint32(3)
	got := igt.Pad(src, top, 2, 2, left, igt.Rgba8{0, 0, 0, 255})
	// Pixel at source (1, 1) should appear at output (left+1, top+1).
	sr, sg, sb, sa := pc.PixelAt(src, 1, 1)
	gr, gg, gb, ga := pc.PixelAt(got, left+1, top+1)
	if sr != gr || sg != gg || sb != gb || sa != ga {
		t.Errorf("Pad interior: got (%d,%d,%d,%d) want (%d,%d,%d,%d)",
			gr, gg, gb, ga, sr, sg, sb, sa)
	}
}

// ── Scale ─────────────────────────────────────────────────────────────────────

// TestScaleUpDimensions verifies that Scale produces the requested output size.
func TestScaleUpDimensions(t *testing.T) {
	src := newSolid(4, 4, 128, 128, 128, 255)
	got := igt.Scale(src, 8, 8, igt.Nearest)
	if got.Width != 8 || got.Height != 8 {
		t.Errorf("Scale 4→8: dimensions %dx%d want 8x8", got.Width, got.Height)
	}
}

// TestScaleDownDimensions verifies Scale produces the correct smaller output.
func TestScaleDownDimensions(t *testing.T) {
	src := newSolid(100, 50, 128, 128, 128, 255)
	got := igt.Scale(src, 25, 12, igt.Bilinear)
	if got.Width != 25 || got.Height != 12 {
		t.Errorf("Scale down: dimensions %dx%d want 25x12", got.Width, got.Height)
	}
}

// TestScaleReplicateNoPanic checks that Scale with Replicate OOB doesn't panic.
// This is a regression guard against any accidental out-of-bounds slice access.
func TestScaleReplicateNoPanic(t *testing.T) {
	defer func() {
		if r := recover(); r != nil {
			t.Errorf("Scale panicked: %v", r)
		}
	}()
	src := newGradient(3, 3)
	_ = igt.Scale(src, 7, 7, igt.Bilinear)
}

// TestScaleBicubicNoPanic checks that bicubic Scale doesn't panic.
func TestScaleBicubicNoPanic(t *testing.T) {
	defer func() {
		if r := recover(); r != nil {
			t.Errorf("Scale bicubic panicked: %v", r)
		}
	}()
	src := newGradient(5, 5)
	_ = igt.Scale(src, 10, 10, igt.Bicubic)
}

// ── Rotate (arbitrary angle) ──────────────────────────────────────────────────

// TestRotateZeroIsApproxIdentity checks that a 0-radian rotation leaves the
// image content approximately unchanged (within ±2 per channel for sRGB
// round-trip rounding).
func TestRotateZeroIsApproxIdentity(t *testing.T) {
	src := newSolid(8, 8, 100, 150, 200, 255)
	got := igt.Rotate(src, 0, igt.Bilinear, igt.CropBounds)
	if !imagesNearlyEqual(src, got, 2) {
		t.Error("Rotate(0) should be approximately the identity")
	}
}

// TestRotateFitDimensionsLarger checks that Fit mode produces a canvas at
// least as large as the source for a non-trivial angle.
func TestRotateFitDimensionsLarger(t *testing.T) {
	src := newSolid(10, 10, 200, 100, 50, 255)
	got := igt.Rotate(src, math.Pi/4, igt.Nearest, igt.Fit)
	// A 10×10 image rotated 45° needs a bounding box of ceil(10*√2) ≈ 15 pixels.
	if got.Width < src.Width || got.Height < src.Height {
		t.Errorf("Rotate Fit 45°: output %dx%d should not be smaller than source %dx%d",
			got.Width, got.Height, src.Width, src.Height)
	}
}

// TestRotateCropSameDimensions verifies Crop mode preserves source dimensions.
func TestRotateCropSameDimensions(t *testing.T) {
	src := newSolid(10, 8, 100, 100, 100, 255)
	got := igt.Rotate(src, math.Pi/6, igt.Nearest, igt.CropBounds)
	if got.Width != src.Width || got.Height != src.Height {
		t.Errorf("Rotate CropBounds: dimensions %dx%d want %dx%d", got.Width, got.Height, src.Width, src.Height)
	}
}

// ── Affine ────────────────────────────────────────────────────────────────────

// TestAffineIdentityIsApproxIdentity verifies that the identity matrix produces
// an image nearly identical to the source.
func TestAffineIdentityIsApproxIdentity(t *testing.T) {
	src := newSolid(8, 8, 100, 150, 200, 255)
	identity := [2][3]float64{{1, 0, 0}, {0, 1, 0}}
	got := igt.Affine(src, identity, src.Width, src.Height, igt.Bilinear, igt.Replicate)
	if !imagesNearlyEqual(src, got, 2) {
		t.Error("Affine with identity matrix should be approximately the identity")
	}
}

// TestAffineDimensions verifies that Affine produces the requested output size.
func TestAffineDimensions(t *testing.T) {
	src := newSolid(10, 10, 50, 50, 50, 255)
	identity := [2][3]float64{{1, 0, 0}, {0, 1, 0}}
	got := igt.Affine(src, identity, 15, 12, igt.Nearest, igt.Zero)
	if got.Width != 15 || got.Height != 12 {
		t.Errorf("Affine dimensions: got %dx%d want 15x12", got.Width, got.Height)
	}
}

// ── PerspectiveWarp ───────────────────────────────────────────────────────────

// TestPerspectiveWarpIdentityIsApproxIdentity verifies that the 3×3 identity
// homography reproduces the source image approximately.
func TestPerspectiveWarpIdentityIsApproxIdentity(t *testing.T) {
	src := newSolid(8, 8, 200, 100, 50, 255)
	hIdentity := [3][3]float64{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
	got := igt.PerspectiveWarp(src, hIdentity, src.Width, src.Height, igt.Bilinear, igt.Replicate)
	if !imagesNearlyEqual(src, got, 2) {
		t.Error("PerspectiveWarp with identity H should be approximately the identity")
	}
}

// TestPerspectiveWarpDimensions checks that PerspectiveWarp produces the
// requested output dimensions.
func TestPerspectiveWarpDimensions(t *testing.T) {
	src := newSolid(10, 10, 50, 50, 50, 255)
	hIdentity := [3][3]float64{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
	got := igt.PerspectiveWarp(src, hIdentity, 14, 9, igt.Nearest, igt.Zero)
	if got.Width != 14 || got.Height != 9 {
		t.Errorf("PerspectiveWarp dimensions: got %dx%d want 14x9", got.Width, got.Height)
	}
}

// ── Sample / interpolation unit tests ────────────────────────────────────────

// TestSampleNearestExactPixel verifies that Nearest-neighbour sampling at an
// exact integer pixel coordinate returns that pixel's values unchanged.
func TestSampleNearestExactPixel(t *testing.T) {
	src := pc.New(4, 4)
	pc.SetPixel(src, 2, 1, 10, 20, 30, 255)
	r, g, b, a := igt.Sample(src, 2, 1, igt.Nearest, igt.Zero)
	if r != 10 || g != 20 || b != 30 || a != 255 {
		t.Errorf("Sample Nearest at (2,1): got (%d,%d,%d,%d) want (10,20,30,255)", r, g, b, a)
	}
}

// TestSampleBilinearMidpointBlend verifies bilinear interpolation at the exact
// midpoint between two pixels on a horizontal gradient.
//
// We use a 2×1 image: pixel (0,0) = (0, 0, 0, 255) and (1,0) = (254, 0, 0, 255).
// The midpoint in continuous coordinates is u = 0.5, v = 0.
// Bilinear should blend the two pixels 50/50 in linear space.
//
// Decoded linear values: decode(0) = 0.0, decode(254) ≈ 0.9911
// Midpoint linear: 0.4956  → encode ≈ 186 in sRGB
// We allow ±4 to account for the specific LUT rounding.
func TestSampleBilinearMidpointBlend(t *testing.T) {
	src := pc.New(2, 1)
	pc.SetPixel(src, 0, 0, 0, 0, 0, 255)
	pc.SetPixel(src, 1, 0, 254, 0, 0, 255)

	r, _, _, _ := igt.Sample(src, 0.5, 0, igt.Bilinear, igt.Replicate)

	// Compute expected: 50% blend in linear light of 0 and 254.
	// linear(0) = 0.0, linear(254) ≈ 0.9911
	// mid_linear = 0.4956
	// encode(0.4956) ≈ 186 (sRGB)
	// Allow ±4 tolerance.
	expected := 186
	diff := absInt(int(r) - expected)
	if diff > 4 {
		t.Errorf("Bilinear midpoint blend: got R=%d want ~%d (diff=%d)", r, expected, diff)
	}
}

// TestSampleNearestOOBZeroReturnsBlack verifies that Nearest sampling outside
// image bounds with the Zero policy returns transparent black.
func TestSampleNearestOOBZeroReturnsBlack(t *testing.T) {
	src := newSolid(4, 4, 200, 100, 50, 255)
	r, g, b, a := igt.Sample(src, -1, -1, igt.Nearest, igt.Zero)
	if r != 0 || g != 0 || b != 0 || a != 0 {
		t.Errorf("Sample OOB Zero: got (%d,%d,%d,%d) want (0,0,0,0)", r, g, b, a)
	}
}

// TestSampleNearestOOBReplicateReturnsEdge verifies that Nearest sampling
// outside image bounds with Replicate returns the nearest edge pixel.
func TestSampleNearestOOBReplicateReturnsEdge(t *testing.T) {
	src := pc.New(4, 4)
	pc.SetPixel(src, 0, 0, 42, 84, 126, 255)
	r, g, b, a := igt.Sample(src, -5, -5, igt.Nearest, igt.Replicate)
	if r != 42 || g != 84 || b != 126 || a != 255 {
		t.Errorf("Sample OOB Replicate: got (%d,%d,%d,%d) want (42,84,126,255)", r, g, b, a)
	}
}

// TestSampleBicubicExactCenter verifies that bicubic sampling at an exact pixel
// centre returns approximately the pixel's own value.
// For a solid image, all 4×4 neighbours are identical so the spline collapses
// to the exact value.
func TestSampleBicubicExactCenter(t *testing.T) {
	src := newSolid(8, 8, 180, 90, 45, 255)
	r, g, b, a := igt.Sample(src, 4, 4, igt.Bicubic, igt.Replicate)
	if absInt(int(r)-180) > 2 || absInt(int(g)-90) > 2 || absInt(int(b)-45) > 2 || a != 255 {
		t.Errorf("Bicubic centre on solid: got (%d,%d,%d,%d) want ~(180,90,45,255)", r, g, b, a)
	}
}

// TestRotate360IsApproxIdentity rotates a full 2π and checks the result is
// approximately the identity (within ±2 per channel).
func TestRotate360IsApproxIdentity(t *testing.T) {
	src := newSolid(6, 6, 120, 80, 200, 255)
	got := igt.Rotate(src, 2*math.Pi, igt.Bilinear, igt.CropBounds)
	if !imagesNearlyEqual(src, got, 3) {
		t.Error("Rotate(2π) should be approximately the identity")
	}
}
