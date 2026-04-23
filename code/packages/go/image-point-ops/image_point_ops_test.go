package imagepointops_test

import (
	"testing"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
	ops "github.com/adhithyan15/coding-adventures/code/packages/go/image-point-ops"
)

// solid creates a 1×1 PixelContainer with the given colour.
func solid(r, g, b, a byte) *pc.PixelContainer {
	img := pc.New(1, 1)
	img.Data[0], img.Data[1], img.Data[2], img.Data[3] = r, g, b, a
	return img
}

func pxAt(img *pc.PixelContainer, x, y int) (byte, byte, byte, byte) {
	offset := (y*int(img.Width) + x) * 4
	return img.Data[offset], img.Data[offset+1], img.Data[offset+2], img.Data[offset+3]
}

func abs(a, b byte) byte {
	if a >= b {
		return a - b
	}
	return b - a
}

func TestDimensionsPreserved(t *testing.T) {
	img := pc.New(3, 5)
	out := ops.Invert(img)
	if out.Width != 3 || out.Height != 5 {
		t.Fatalf("expected 3×5, got %d×%d", out.Width, out.Height)
	}
}

func TestInvertRGB(t *testing.T) {
	out := ops.Invert(solid(10, 100, 200, 255))
	r, g, b, a := pxAt(out, 0, 0)
	if r != 245 || g != 155 || b != 55 || a != 255 {
		t.Fatalf("got %d,%d,%d,%d", r, g, b, a)
	}
}

func TestInvertPreservesAlpha(t *testing.T) {
	out := ops.Invert(solid(10, 100, 200, 128))
	_, _, _, a := pxAt(out, 0, 0)
	if a != 128 {
		t.Fatalf("expected alpha=128, got %d", a)
	}
}

func TestDoubleInvertIdentity(t *testing.T) {
	img := solid(30, 80, 180, 255)
	out := ops.Invert(ops.Invert(img))
	r, g, b, a := pxAt(out, 0, 0)
	ir, ig, ib, ia := pxAt(img, 0, 0)
	if r != ir || g != ig || b != ib || a != ia {
		t.Fatal("double invert != identity")
	}
}

func TestThresholdAbove(t *testing.T) {
	out := ops.Threshold(solid(200, 200, 200, 255), 128)
	r, g, b, _ := pxAt(out, 0, 0)
	if r != 255 || g != 255 || b != 255 {
		t.Fatalf("expected white, got %d,%d,%d", r, g, b)
	}
}

func TestThresholdBelow(t *testing.T) {
	out := ops.Threshold(solid(50, 50, 50, 255), 128)
	r, g, b, _ := pxAt(out, 0, 0)
	if r != 0 || g != 0 || b != 0 {
		t.Fatalf("expected black, got %d,%d,%d", r, g, b)
	}
}

func TestThresholdLuminanceWhite(t *testing.T) {
	out := ops.ThresholdLuminance(solid(255, 255, 255, 255), 128)
	r, _, _, _ := pxAt(out, 0, 0)
	if r != 255 {
		t.Fatalf("expected white, got %d", r)
	}
}

func TestPosterizeTwoLevels(t *testing.T) {
	out := ops.Posterize(solid(50, 50, 50, 255), 2)
	r, _, _, _ := pxAt(out, 0, 0)
	if r != 0 && r != 255 {
		t.Fatalf("expected 0 or 255, got %d", r)
	}
}

func TestSwapRGBBGR(t *testing.T) {
	out := ops.SwapRGBBGR(solid(255, 0, 0, 255))
	r, g, b, _ := pxAt(out, 0, 0)
	if r != 0 || g != 0 || b != 255 {
		t.Fatalf("expected B=255, got %d,%d,%d", r, g, b)
	}
}

func TestExtractChannelRed(t *testing.T) {
	out := ops.ExtractChannel(solid(100, 150, 200, 255), ops.ChannelR)
	r, g, b, _ := pxAt(out, 0, 0)
	if r != 100 || g != 0 || b != 0 {
		t.Fatalf("got %d,%d,%d", r, g, b)
	}
}

func TestBrightnessClamps(t *testing.T) {
	out := ops.Brightness(solid(250, 10, 10, 255), 20)
	r, g, _, _ := pxAt(out, 0, 0)
	if r != 255 {
		t.Fatalf("expected r=255 (clamped), got %d", r)
	}
	if g != 30 {
		t.Fatalf("expected g=30, got %d", g)
	}
}

func TestContrastIdentity(t *testing.T) {
	img := solid(100, 150, 200, 255)
	out := ops.Contrast(img, 1.0)
	r, g, b, _ := pxAt(out, 0, 0)
	ir, ig, ib, _ := pxAt(img, 0, 0)
	if abs(r, ir) > 1 || abs(g, ig) > 1 || abs(b, ib) > 1 {
		t.Fatalf("contrast identity failed: %d,%d,%d vs %d,%d,%d", r, g, b, ir, ig, ib)
	}
}

func TestGammaIdentity(t *testing.T) {
	img := solid(100, 150, 200, 255)
	out := ops.Gamma(img, 1.0)
	r, _, _, _ := pxAt(out, 0, 0)
	ir, _, _, _ := pxAt(img, 0, 0)
	if abs(r, ir) > 1 {
		t.Fatalf("gamma identity failed: %d vs %d", r, ir)
	}
}

func TestGammaBrightensMidtones(t *testing.T) {
	img := solid(128, 128, 128, 255)
	out := ops.Gamma(img, 0.5)
	r, _, _, _ := pxAt(out, 0, 0)
	if r <= 128 {
		t.Fatalf("expected brighter, got %d", r)
	}
}

func TestExposurePlusOne(t *testing.T) {
	img := solid(100, 100, 100, 255)
	out := ops.Exposure(img, 1.0)
	r, _, _, _ := pxAt(out, 0, 0)
	ir, _, _, _ := pxAt(img, 0, 0)
	if r <= ir {
		t.Fatalf("expected brighter, got %d vs %d", r, ir)
	}
}

func TestGreyscaleWhiteStaysWhite(t *testing.T) {
	for _, method := range []ops.GreyscaleMethod{ops.Rec709, ops.BT601, ops.Average} {
		out := ops.Greyscale(solid(255, 255, 255, 255), method)
		r, g, b, _ := pxAt(out, 0, 0)
		if r != 255 || g != 255 || b != 255 {
			t.Fatalf("method %d: expected white, got %d,%d,%d", method, r, g, b)
		}
	}
}

func TestGreyscaleBlackStaysBlack(t *testing.T) {
	out := ops.Greyscale(solid(0, 0, 0, 255), ops.Rec709)
	r, g, b, _ := pxAt(out, 0, 0)
	if r != 0 || g != 0 || b != 0 {
		t.Fatalf("expected black, got %d,%d,%d", r, g, b)
	}
}

func TestSepiaPreservesAlpha(t *testing.T) {
	out := ops.Sepia(solid(128, 128, 128, 200))
	_, _, _, a := pxAt(out, 0, 0)
	if a != 200 {
		t.Fatalf("expected alpha=200, got %d", a)
	}
}

func TestColourMatrixIdentity(t *testing.T) {
	img := solid(80, 120, 200, 255)
	id := [3][3]float64{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
	out := ops.ColourMatrix(img, id)
	r, g, b, _ := pxAt(out, 0, 0)
	ir, ig, ib, _ := pxAt(img, 0, 0)
	if abs(r, ir) > 1 || abs(g, ig) > 1 || abs(b, ib) > 1 {
		t.Fatalf("identity matrix diverged: %d,%d,%d vs %d,%d,%d", r, g, b, ir, ig, ib)
	}
}

func TestSaturateZeroGivesGrey(t *testing.T) {
	out := ops.Saturate(solid(200, 100, 50, 255), 0)
	r, g, b, _ := pxAt(out, 0, 0)
	if r != g || g != b {
		t.Fatalf("expected equal channels, got %d,%d,%d", r, g, b)
	}
}

func TestHueRotate360Identity(t *testing.T) {
	img := solid(200, 80, 40, 255)
	out := ops.HueRotate(img, 360)
	r, g, b, _ := pxAt(out, 0, 0)
	ir, ig, ib, _ := pxAt(img, 0, 0)
	if abs(r, ir) > 2 || abs(g, ig) > 2 || abs(b, ib) > 2 {
		t.Fatalf("360° hue rotate diverged: %d,%d,%d vs %d,%d,%d", r, g, b, ir, ig, ib)
	}
}

func TestSRGBLinearRoundtrip(t *testing.T) {
	img := solid(100, 150, 200, 255)
	out := ops.LinearToSRGBImage(ops.SRGBToLinearImage(img))
	r, g, b, _ := pxAt(out, 0, 0)
	ir, ig, ib, _ := pxAt(img, 0, 0)
	if abs(r, ir) > 2 || abs(g, ig) > 2 || abs(b, ib) > 2 {
		t.Fatalf("round-trip diverged: %d,%d,%d vs %d,%d,%d", r, g, b, ir, ig, ib)
	}
}

func TestApplyLUT1DInvert(t *testing.T) {
	var lut [256]byte
	for i := range lut {
		lut[i] = byte(255 - i)
	}
	out := ops.ApplyLUT1DU8(solid(100, 0, 200, 255), &lut, &lut, &lut)
	r, g, b, _ := pxAt(out, 0, 0)
	if r != 155 || g != 255 || b != 55 {
		t.Fatalf("got %d,%d,%d", r, g, b)
	}
}

func TestBuildLUT1DU8Identity(t *testing.T) {
	lut := ops.BuildLUT1DU8(func(v float64) float64 { return v })
	for i := 0; i < 256; i++ {
		if abs(lut[i], byte(i)) > 1 {
			t.Fatalf("index %d: got %d", i, lut[i])
		}
	}
}

func TestBuildGammaLUTIdentity(t *testing.T) {
	lut := ops.BuildGammaLUT(1.0)
	for i := 0; i < 256; i++ {
		if abs(lut[i], byte(i)) > 1 {
			t.Fatalf("index %d: got %d", i, lut[i])
		}
	}
}
