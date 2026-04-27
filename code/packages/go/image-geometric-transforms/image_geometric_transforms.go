// Package imagegeometrictransforms implements IMG04 — geometric transforms on
// PixelContainer images.
//
// # What Is a Geometric Transform?
//
// A geometric transform moves pixels spatially rather than changing their
// colour values.  Flipping a photo horizontally, scaling it to a thumbnail,
// rotating it 45°, or mapping it through a projective homography are all
// geometric transforms.  Unlike point operations (IMG03), which change what
// colour a pixel has, geometric transforms change where pixels live.
//
// # The Inverse-Warp Model
//
// A naive "forward warp" iterates over every source pixel and asks "where does
// this pixel land in the output?"  The problem: two source pixels may land on
// the same output location while other output locations receive nothing — leaving
// holes.
//
// The correct approach is the inverse warp (also called "pull" sampling):
//
//  1. Iterate over every output pixel (x', y').
//  2. Apply the inverse transform to obtain a source coordinate (u, v).
//  3. Sample the source image at (u, v) using the chosen interpolation filter.
//  4. Write the sampled colour into (x', y').
//
// Because we visit every output pixel exactly once, there are no holes and no
// double-writes.  Every function in this package follows this pattern.
//
// # The Pixel-Centre Model
//
// A pixel at integer coordinate x occupies the half-open interval [x, x+1) in
// continuous space.  Its centre is at x + 0.5.
//
// When mapping a W-pixel row to a W'-pixel row (scaling), we want the left
// edge of the source (continuous 0.0) to align with the left edge of the
// output, and similarly for the right edge.  Using pixel centres:
//
//	sx = float64(srcW) / float64(outW)   // scale factor
//	u = (float64(x') + 0.5) * sx - 0.5  // source centre for output pixel x'
//
// This formula ensures that the first and last output pixels map exactly onto
// the first and last source pixels, avoiding the half-pixel shift that occurs
// when using u = x' * sx.
//
// # Linear-Light Requirement
//
// Pixel bytes are stored in the sRGB colour space — a perceptual encoding with
// an approximate gamma of 2.2.  Mixing two sRGB values directly (e.g.
// averaging bytes 0 and 200 to get 100) produces the wrong answer because the
// perceptual scale is non-linear.
//
// Correct interpolation:
//  1. Decode sRGB bytes to linear-light floats (srgbToLinear LUT).
//  2. Perform all weighted sums in linear space.
//  3. Re-encode each result channel back to sRGB bytes (encode function).
//
// Lossless pixel-copy operations (flips, 90°/180° rotations, crop) bypass the
// LUT entirely — they copy bytes verbatim, so no colour-space error is possible.
//
// # Catmull-Rom Spline
//
// Bicubic interpolation reconstructs a smooth surface through a 4×4
// neighbourhood of source pixels.  Each axis independently weights four
// samples using the Catmull-Rom kernel with parameter α = 0.5:
//
//	Keys (1983):  w(d) = { (α+2)|d|³ − (α+3)|d|² + 1           for |d| ≤ 1
//	                     { α|d|³ − 5α|d|² + 8α|d| − 4α          for 1 < |d| ≤ 2
//	                     { 0                                      otherwise
//
// With α = 0.5 the kernel reduces to the standard Catmull-Rom formula and has
// C¹ continuity — it passes through the control points and has smooth
// derivatives, making it well suited for image upscaling.
package imagegeometrictransforms

import (
	"math"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// ── Type definitions ──────────────────────────────────────────────────────────

// Interpolation selects the filter used when sampling a source image at a
// non-integer coordinate.
//
// Higher-quality filters examine more neighbouring pixels and are therefore
// slower but produce smoother results, especially during upscaling.
type Interpolation int

const (
	// Nearest picks the closest source pixel with no blending.
	// Fastest; produces a blocky "pixel art" look when upscaling.
	Nearest Interpolation = iota

	// Bilinear blends the four nearest pixels in linear light using
	// bilinear interpolation.  Good general-purpose filter.
	Bilinear

	// Bicubic blends a 4×4 neighbourhood using the Catmull-Rom kernel.
	// Smoother than Bilinear, especially at large upscale ratios.
	Bicubic
)

// RotateBounds controls the output canvas size when rotating by an arbitrary
// angle.
type RotateBounds int

const (
	// Fit expands the canvas to contain the entire rotated source image.
	// No content is clipped; corners of the original image are always visible.
	Fit RotateBounds = iota

	// CropBounds keeps the canvas at the source dimensions.
	// The output has the same size as the input; corners may be clipped.
	CropBounds
)

// OutOfBounds determines how sampling behaves when the inverse warp maps an
// output pixel to a coordinate outside the source image boundary.
type OutOfBounds int

const (
	// Zero returns (0, 0, 0, 0) — transparent black — for out-of-bounds reads.
	// Use this when the source image has a natural "no data" region (e.g.
	// rotation with empty corners).
	Zero OutOfBounds = iota

	// Replicate clamps coordinates to the nearest edge pixel.
	// Equivalent to OpenGL's GL_CLAMP_TO_EDGE.
	Replicate

	// Reflect mirrors coordinates at the image boundary, creating a mirror-tile
	// effect.  Equivalent to OpenGL's GL_MIRRORED_REPEAT.
	Reflect

	// Wrap tiles the image by taking coordinates modulo the dimension.
	// Equivalent to OpenGL's GL_REPEAT.
	Wrap
)

// Rgba8 is a convenience struct for passing a fill colour to Pad.
type Rgba8 struct{ R, G, B, A uint8 }

// ── sRGB / linear-light LUT ───────────────────────────────────────────────────

// srgbToLinear is a 256-entry decode look-up table mapping sRGB byte values
// (0–255) to linear-light float64 values (0.0–1.0).
//
// The sRGB standard specifies a piecewise transfer function:
//
//	c_linear = c_srgb / 12.92                          for c_srgb ≤ 0.04045
//	c_linear = ((c_srgb + 0.055) / 1.055) ^ 2.4       otherwise
//
// where c_srgb = byte / 255.0.  Building the table once at init time means
// every interpolation call pays a single array lookup instead of a branch +
// power computation.
var srgbToLinear [256]float64

func init() {
	for i := range srgbToLinear {
		c := float64(i) / 255.0
		if c <= 0.04045 {
			srgbToLinear[i] = c / 12.92
		} else {
			srgbToLinear[i] = math.Pow((c+0.055)/1.055, 2.4)
		}
	}
}

// encode converts a linear-light value (0.0–1.0) back to an sRGB byte.
//
// The inverse transfer function applies the sRGB gamma curve:
//
//	c_srgb = c_linear * 12.92                          for c_linear ≤ 0.0031308
//	c_srgb = 1.055 * c_linear^(1/2.4) − 0.055         otherwise
//
// Values are clamped to [0, 1] before scaling to [0, 255] and rounding.
func encode(v float64) byte {
	var c float64
	if v <= 0.0031308 {
		c = v * 12.92
	} else {
		c = 1.055*math.Pow(v, 1.0/2.4) - 0.055
	}
	return byte(math.Round(math.Min(1, math.Max(0, c)) * 255))
}

// ── Out-of-bounds coordinate resolution ──────────────────────────────────────

// resolveCoord maps a potentially out-of-range integer coordinate x into the
// valid range [0, max-1] according to the OutOfBounds policy.
//
// Returns (coordinate, valid).  When oob == Zero and x is out of range, valid
// is false and the caller should use the zero pixel.  For all other policies
// the returned coordinate is always valid (within [0, max-1]).
//
// The four policies correspond directly to GPU texture wrapping modes:
//
//	Zero      — GL_CLAMP_TO_BORDER (border colour = 0)
//	Replicate — GL_CLAMP_TO_EDGE
//	Reflect   — GL_MIRRORED_REPEAT
//	Wrap      — GL_REPEAT
func resolveCoord(x, max int, oob OutOfBounds) (int, bool) {
	if x >= 0 && x < max {
		return x, true
	}
	switch oob {
	case Zero:
		return 0, false

	case Replicate:
		if x < 0 {
			return 0, true
		}
		return max - 1, true

	case Reflect:
		// Mirror-repeat: fold the coordinate back at both ends.
		// Period is 2*max; the pattern is: 0 1 2 … max-1 max-1 … 1 0 1 2 …
		period := 2 * max
		x = ((x % period) + period) % period // normalise to [0, 2*max)
		if x >= max {
			x = period - 1 - x
		}
		return x, true

	case Wrap:
		// Simple modulo tiling; handle negative x correctly in Go.
		x = ((x % max) + max) % max
		return x, true
	}
	return 0, false
}

// ── Catmull-Rom kernel ─────────────────────────────────────────────────────────

// catmullRom evaluates the Catmull-Rom cubic spline kernel at distance d.
//
// The Keys (1983) formula with α = 0.5:
//
//	|d| ≤ 1  →  1.5|d|³ − 2.5|d|² + 1
//	|d| ≤ 2  →  −0.5|d|³ + 2.5|d|² − 4|d| + 2
//	otherwise →  0
//
// This kernel has unit area, passes through control points at d = 0, 1, and
// has zero first derivatives at those points, giving a smooth C¹ spline.
func catmullRom(d float64) float64 {
	d = math.Abs(d)
	switch {
	case d < 1:
		return 1.5*d*d*d - 2.5*d*d + 1
	case d < 2:
		return -0.5*d*d*d + 2.5*d*d - 4*d + 2
	default:
		return 0
	}
}

// ── Sampling functions ────────────────────────────────────────────────────────

// sampleNN samples the source image at (u, v) using nearest-neighbour
// interpolation.  The continuous coordinate is snapped to the nearest integer
// pixel using floor(u + 0.5), i.e. conventional rounding.
func sampleNN(img *pc.PixelContainer, u, v float64, oob OutOfBounds) (byte, byte, byte, byte) {
	xi := int(math.Floor(u + 0.5))
	yi := int(math.Floor(v + 0.5))

	cx, validX := resolveCoord(xi, int(img.Width), oob)
	cy, validY := resolveCoord(yi, int(img.Height), oob)
	if !validX || !validY {
		return 0, 0, 0, 0
	}
	return pc.PixelAt(img, uint32(cx), uint32(cy))
}

// sampleBilinear samples the source image at (u, v) using bilinear
// interpolation in linear-light space.
//
// The four nearest pixels form a unit square.  Let (x0, y0) be the
// integer floor of (u, v), and let (tx, ty) be the fractional parts.
// The blend weights are:
//
//	w00 = (1-tx)(1-ty),  w10 = tx(1-ty)
//	w01 = (1-tx)ty,      w11 = tx·ty
//
// Each channel is decoded to linear light, blended by the four weights, then
// re-encoded to sRGB.  Alpha is blended directly in the byte domain (it is not
// gamma-encoded).
func sampleBilinear(img *pc.PixelContainer, u, v float64, oob OutOfBounds) (byte, byte, byte, byte) {
	x0 := int(math.Floor(u))
	y0 := int(math.Floor(v))
	tx := u - float64(x0)
	ty := v - float64(y0)

	// Helper to get a pixel, respecting OOB policy.
	get := func(xi, yi int) (float64, float64, float64, float64) {
		cx, vx := resolveCoord(xi, int(img.Width), oob)
		cy, vy := resolveCoord(yi, int(img.Height), oob)
		if !vx || !vy {
			return 0, 0, 0, 0
		}
		r, g, b, a := pc.PixelAt(img, uint32(cx), uint32(cy))
		return srgbToLinear[r], srgbToLinear[g], srgbToLinear[b], float64(a) / 255.0
	}

	r00, g00, b00, a00 := get(x0, y0)
	r10, g10, b10, a10 := get(x0+1, y0)
	r01, g01, b01, a01 := get(x0, y0+1)
	r11, g11, b11, a11 := get(x0+1, y0+1)

	lerp := func(a, b, t float64) float64 { return a*(1-t) + b*t }

	lr := lerp(lerp(r00, r10, tx), lerp(r01, r11, tx), ty)
	lg := lerp(lerp(g00, g10, tx), lerp(g01, g11, tx), ty)
	lb := lerp(lerp(b00, b10, tx), lerp(b01, b11, tx), ty)
	la := lerp(lerp(a00, a10, tx), lerp(a01, a11, tx), ty)

	return encode(lr), encode(lg), encode(lb), byte(math.Round(la * 255))
}

// sampleBicubic samples the source image at (u, v) using bicubic
// Catmull-Rom interpolation over a 4×4 neighbourhood in linear-light space.
//
// The 4×4 grid is centred on the integer floor of (u, v).  Each row is first
// collapsed to a single value using 1-D Catmull-Rom weights, and then the four
// row values are combined with another 1-D pass — the classic separable form.
//
// Intermediate values may exceed [0, 1]; the final encode() call clamps them.
func sampleBicubic(img *pc.PixelContainer, u, v float64, oob OutOfBounds) (byte, byte, byte, byte) {
	x0 := int(math.Floor(u))
	y0 := int(math.Floor(v))
	fx := u - float64(x0)
	fy := v - float64(y0)

	// Catmull-Rom weights for the 4 taps at distances -1, 0, 1, 2 from x0.
	wx := [4]float64{catmullRom(fx + 1), catmullRom(fx), catmullRom(1 - fx), catmullRom(2 - fx)}
	wy := [4]float64{catmullRom(fy + 1), catmullRom(fy), catmullRom(1 - fy), catmullRom(2 - fy)}

	var sumR, sumG, sumB, sumA float64

	for j := 0; j < 4; j++ {
		// Accumulate one horizontal pass for row y0-1+j.
		yi := y0 - 1 + j
		cy, vy := resolveCoord(yi, int(img.Height), oob)

		var rowR, rowG, rowB, rowA float64
		for i := 0; i < 4; i++ {
			xi := x0 - 1 + i
			cx, vx := resolveCoord(xi, int(img.Width), oob)

			var lr, lg, lb, la float64
			if vx && vy {
				r, g, b, a := pc.PixelAt(img, uint32(cx), uint32(cy))
				lr, lg, lb, la = srgbToLinear[r], srgbToLinear[g], srgbToLinear[b], float64(a)/255.0
			}
			rowR += wx[i] * lr
			rowG += wx[i] * lg
			rowB += wx[i] * lb
			rowA += wx[i] * la
		}

		sumR += wy[j] * rowR
		sumG += wy[j] * rowG
		sumB += wy[j] * rowB
		sumA += wy[j] * rowA
	}

	return encode(sumR), encode(sumG), encode(sumB),
		byte(math.Round(math.Min(1, math.Max(0, sumA))*255))
}

// Sample is the public sampling entry point.  It dispatches to the appropriate
// filter based on mode and handles all out-of-bounds cases.
//
// (u, v) are continuous source coordinates in the pixel-centre model: pixel
// (0, 0) has its centre at (0, 0), and pixel (w-1, h-1) has its centre at
// (w-1, h-1).
func Sample(img *pc.PixelContainer, u, v float64, mode Interpolation, oob OutOfBounds) (byte, byte, byte, byte) {
	switch mode {
	case Bilinear:
		return sampleBilinear(img, u, v, oob)
	case Bicubic:
		return sampleBicubic(img, u, v, oob)
	default:
		return sampleNN(img, u, v, oob)
	}
}

// ── Lossless transforms ───────────────────────────────────────────────────────

// FlipHorizontal mirrors the image left-to-right.
//
// Each row is reversed in place: output pixel at (x', y) copies from source
// pixel at (Width-1-x', y).  No colour-space conversion is needed because
// pixels are copied verbatim.
func FlipHorizontal(src *pc.PixelContainer) *pc.PixelContainer {
	w, h := src.Width, src.Height
	out := pc.New(w, h)
	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			r, g, b, a := pc.PixelAt(src, w-1-x, y)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// FlipVertical mirrors the image top-to-bottom.
//
// Output pixel at (x, y') copies from source pixel at (x, Height-1-y').
func FlipVertical(src *pc.PixelContainer) *pc.PixelContainer {
	w, h := src.Width, src.Height
	out := pc.New(w, h)
	for y := uint32(0); y < h; y++ {
		for x := uint32(0); x < w; x++ {
			r, g, b, a := pc.PixelAt(src, x, h-1-y)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// Rotate90CW rotates the image 90° clockwise.
//
// The output dimensions swap: output width = src.Height, output height = src.Width.
//
// Inverse warp derivation (image coords: x = column right, y = row down):
//
//	Forward CW:  (x_src, y_src) → (x'=H-1-y_src, y'=x_src)
//	Inverse:     (x', y') → x_src=y', y_src=H-1-x'
//
// So output pixel (x', y') copies from source pixel (col=y', row=H-1-x').
func Rotate90CW(src *pc.PixelContainer) *pc.PixelContainer {
	W, H := src.Width, src.Height
	// Output: width = H (src height), height = W (src width).
	out := pc.New(H, W)
	for y := uint32(0); y < W; y++ { // y' ∈ [0, W)
		for x := uint32(0); x < H; x++ { // x' ∈ [0, H)
			// Inverse: source col = y', source row = H-1-x'
			r, g, b, a := pc.PixelAt(src, y, H-1-x)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// Rotate90CCW rotates the image 90° counter-clockwise.
//
// Output dimensions also swap (width = src.Height, height = src.Width).
//
// Inverse warp derivation (image coords: x = column right, y = row down):
//
//	Forward CCW:  (x_src, y_src) → (x'=y_src, y'=W-1-x_src)
//	Inverse:      (x', y') → x_src=W-1-y', y_src=x'
//
// So output pixel (x', y') copies from source pixel (col=W-1-y', row=x').
func Rotate90CCW(src *pc.PixelContainer) *pc.PixelContainer {
	W, H := src.Width, src.Height
	// Output: width = H (src height), height = W (src width).
	out := pc.New(H, W)
	for y := uint32(0); y < W; y++ { // y' ∈ [0, W)
		for x := uint32(0); x < H; x++ { // x' ∈ [0, H)
			// Inverse: source col = W-1-y', source row = x'
			r, g, b, a := pc.PixelAt(src, W-1-y, x)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// Rotate180 rotates the image 180°.  Dimensions are preserved.
//
// Output pixel (x', y') copies from source pixel (W-1-x', H-1-y').
// This is equivalent to applying both FlipHorizontal and FlipVertical.
func Rotate180(src *pc.PixelContainer) *pc.PixelContainer {
	W, H := src.Width, src.Height
	out := pc.New(W, H)
	for y := uint32(0); y < H; y++ {
		for x := uint32(0); x < W; x++ {
			r, g, b, a := pc.PixelAt(src, W-1-x, H-1-y)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// Crop extracts the sub-image with top-left corner (x, y), width w, height h.
//
// Any portion of the requested crop rectangle that falls outside the source
// image is filled with transparent black (0, 0, 0, 0).  This matches common
// compositing behaviour: out-of-bounds areas are treated as empty space.
//
// x, y are the top-left corner in source pixel coordinates.
// w, h are the width and height of the output image in pixels.
func Crop(src *pc.PixelContainer, x, y, w, h uint32) *pc.PixelContainer {
	out := pc.New(w, h)
	for oy := uint32(0); oy < h; oy++ {
		for ox := uint32(0); ox < w; ox++ {
			sx := x + ox
			sy := y + oy
			r, g, b, a := pc.PixelAt(src, sx, sy)
			// PixelAt returns (0,0,0,0) for out-of-bounds, so no extra check needed.
			pc.SetPixel(out, ox, oy, r, g, b, a)
		}
	}
	return out
}

// Pad adds a border of fill pixels around the source image.
//
// top, right, bottom, left specify the number of pixels to add on each edge.
// fill specifies the colour of the added border pixels.
//
// The output dimensions are:
//
//	outW = left + src.Width  + right
//	outH = top  + src.Height + bottom
//
// The original image is placed at offset (left, top) in the output.
func Pad(src *pc.PixelContainer, top, right, bottom, left uint32, fill Rgba8) *pc.PixelContainer {
	outW := left + src.Width + right
	outH := top + src.Height + bottom
	out := pc.New(outW, outH)

	// Flood the entire output with the fill colour.
	for i := 0; i < len(out.Data); i += 4 {
		out.Data[i] = fill.R
		out.Data[i+1] = fill.G
		out.Data[i+2] = fill.B
		out.Data[i+3] = fill.A
	}

	// Copy the source image into the interior.
	for y := uint32(0); y < src.Height; y++ {
		for x := uint32(0); x < src.Width; x++ {
			r, g, b, a := pc.PixelAt(src, x, y)
			pc.SetPixel(out, left+x, top+y, r, g, b, a)
		}
	}
	return out
}

// ── Continuous transforms ─────────────────────────────────────────────────────

// Scale resizes the source image to outW×outH pixels using the pixel-centre
// model and the chosen interpolation filter.
//
// The scale factors are:
//
//	sx = float64(src.Width)  / float64(outW)
//	sy = float64(src.Height) / float64(outH)
//
// For output pixel (x', y'), the source coordinate is:
//
//	u = (x' + 0.5) * sx - 0.5
//	v = (y' + 0.5) * sy - 0.5
//
// The -0.5 shift converts from a pixel-corner model (where sample 0 starts at
// 0.0) to a pixel-centre model (where sample 0 is centred at 0.0).  Without
// this correction, a scale-by-2 would shift the content half a pixel.
//
// Out-of-bounds policy is Replicate: edge pixels extend to fill any coordinates
// that fall fractionally outside the image boundary.
func Scale(src *pc.PixelContainer, outW, outH uint32, mode Interpolation) *pc.PixelContainer {
	out := pc.New(outW, outH)
	sx := float64(src.Width) / float64(outW)
	sy := float64(src.Height) / float64(outH)

	for y := uint32(0); y < outH; y++ {
		for x := uint32(0); x < outW; x++ {
			u := (float64(x)+0.5)*sx - 0.5
			v := (float64(y)+0.5)*sy - 0.5
			r, g, b, a := Sample(src, u, v, mode, Replicate)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// Rotate rotates the source image by radians counter-clockwise (positive
// radians = CCW following the standard mathematical convention where y points
// up; note that in image coordinates y increases downward, so positive radians
// rotate CW visually on screen).
//
// The bounds parameter controls the output canvas size:
//
//	Fit        — canvas expands to contain all rotated pixels
//	CropBounds — canvas stays at src.Width × src.Height (corners may be clipped)
//
// The rotation is performed as an inverse warp.  Let:
//
//	cxIn  = (src.Width  - 1) / 2   source image centre x
//	cyIn  = (src.Height - 1) / 2   source image centre y
//	cxOut = (outW - 1) / 2         output image centre x
//	cyOut = (outH - 1) / 2         output image centre y
//
// For output pixel (x', y'), the inverse warp to source coordinate (u, v) is:
//
//	dx = x' - cxOut
//	dy = y' - cyOut
//	u  = cxIn + cos*dx + sin*dy
//	v  = cyIn - sin*dx + cos*dy
//
// Out-of-bounds coordinates use the Zero policy — they sample transparent black
// — creating the empty corner regions visible after a non-multiple-of-90 rotation.
func Rotate(src *pc.PixelContainer, radians float64, mode Interpolation, bounds RotateBounds) *pc.PixelContainer {
	W := float64(src.Width)
	H := float64(src.Height)
	cosA := math.Cos(radians)
	sinA := math.Sin(radians)
	absCos := math.Abs(cosA)
	absSin := math.Abs(sinA)

	var outW, outH uint32
	switch bounds {
	case Fit:
		// The bounding box of a W×H rectangle rotated by angle radians.
		outW = uint32(math.Ceil(W*absCos + H*absSin))
		outH = uint32(math.Ceil(W*absSin + H*absCos))
	default: // CropBounds
		outW = src.Width
		outH = src.Height
	}

	out := pc.New(outW, outH)

	// Continuous centres of source and output images.
	cxIn := (W - 1) / 2
	cyIn := (H - 1) / 2
	cxOut := (float64(outW) - 1) / 2
	cyOut := (float64(outH) - 1) / 2

	for y := uint32(0); y < outH; y++ {
		for x := uint32(0); x < outW; x++ {
			dx := float64(x) - cxOut
			dy := float64(y) - cyOut
			// Inverse rotation: rotate by -radians
			u := cxIn + cosA*dx + sinA*dy
			v := cyIn - sinA*dx + cosA*dy
			r, g, b, a := Sample(src, u, v, mode, Zero)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// Affine applies an arbitrary 2-D affine transform to the source image.
//
// matrix is a 2×3 row-major transform matrix in homogeneous coordinates:
//
//	[ m[0][0]  m[0][1]  m[0][2] ]   [u]   [m[0][0]*x' + m[0][1]*y' + m[0][2]]
//	[ m[1][0]  m[1][1]  m[1][2] ] × [y'] = [m[1][0]*x' + m[1][1]*y' + m[1][2]]
//	                                 [ 1 ]
//
// The matrix maps output coordinates (x', y') to source coordinates (u, v).
// This is already the inverse-warp direction, so no matrix inversion is needed.
//
// Common transform matrices:
//
//	Identity:    [[1,0,0],[0,1,0]]
//	Translate Δx, Δy:  [[1,0,-Δx],[0,1,-Δy]]   (negate because inverse)
//	Scale sx, sy:      [[1/sx,0,0],[0,1/sy,0]]
//	Rotate θ (CW):     [[cos,-sin,cx(1-cos)+cy·sin],[sin,cos,cy(1-cos)-cx·sin]]
//
// outW and outH specify the dimensions of the output image.
func Affine(src *pc.PixelContainer, matrix [2][3]float64, outW, outH uint32, mode Interpolation, oob OutOfBounds) *pc.PixelContainer {
	out := pc.New(outW, outH)
	m := matrix
	for y := uint32(0); y < outH; y++ {
		for x := uint32(0); x < outW; x++ {
			u := m[0][0]*float64(x) + m[0][1]*float64(y) + m[0][2]
			v := m[1][0]*float64(x) + m[1][1]*float64(y) + m[1][2]
			r, g, b, a := Sample(src, u, v, mode, oob)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}

// PerspectiveWarp applies a projective (perspective) homography to the source
// image.
//
// h is a 3×3 homogeneous transform matrix.  For each output pixel (x', y'),
// the source coordinate is computed via the homogeneous division:
//
//	[uh, vh, w] = H · [x', y', 1]ᵀ
//	u = uh / w
//	v = vh / w
//
// This is the standard perspective mapping that accounts for the foreshortening
// effect that occurs when projecting a 3-D plane onto a 2-D sensor.
//
// When w ≈ 0 (at the horizon line), the sample is treated as out-of-bounds.
// outW and outH specify the dimensions of the output image.
func PerspectiveWarp(src *pc.PixelContainer, h [3][3]float64, outW, outH uint32, mode Interpolation, oob OutOfBounds) *pc.PixelContainer {
	out := pc.New(outW, outH)
	for y := uint32(0); y < outH; y++ {
		for x := uint32(0); x < outW; x++ {
			xf := float64(x)
			yf := float64(y)
			uh := h[0][0]*xf + h[0][1]*yf + h[0][2]
			vh := h[1][0]*xf + h[1][1]*yf + h[1][2]
			w := h[2][0]*xf + h[2][1]*yf + h[2][2]
			if math.Abs(w) < 1e-10 {
				// At or near the horizon: treat as Zero regardless of policy.
				pc.SetPixel(out, x, y, 0, 0, 0, 0)
				continue
			}
			u := uh / w
			v := vh / w
			r, g, b, a := Sample(src, u, v, mode, oob)
			pc.SetPixel(out, x, y, r, g, b, a)
		}
	}
	return out
}
