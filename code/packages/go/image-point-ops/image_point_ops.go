// Package imagepointops implements IMG03 — per-pixel point operations on
// PixelContainer.
//
// # What Is a Point Operation?
//
// A point operation transforms each pixel independently using only that
// pixel's own value.  No neighbouring pixels are consulted, no frequency-
// domain transform is needed, no geometric mapping is performed.
//
// Contrast this with convolution (which reads a neighbourhood), scaling
// (which requires interpolation across pixels), or compositing (which blends
// two images).  Point operations are the simplest transformation possible.
//
// # Two Domains
//
// The pixel bytes in a PixelContainer are in the sRGB colour space — a
// piecewise-gamma encoding designed so that the human visual system perceives
// roughly equal steps between adjacent byte values.  This is great for
// storage but wrong for arithmetic.
//
// Operations in the u8 domain work directly on the byte values.  They are
// correct without any colour-space conversion because they are monotone
// remappings that never mix or average channel values (invert, threshold,
// posterize, channel extraction, additive brightness).
//
// Operations in the linear domain decode each byte to a linear-light float
// (decode: sRGB u8 → linear f32), perform the arithmetic, then re-encode
// the result (encode: linear f32 → sRGB u8).  Averaging in sRGB space is
// incorrect — see IMG00 §2 for a worked example.
//
// # sRGB ↔ Linear Round-Trip
//
//	Decode (u8 → f32):
//	  c = byte / 255.0
//	  if c <= 0.04045  →  c / 12.92
//	  else             →  ((c + 0.055) / 1.055)^2.4
//
//	Encode (f32 → u8):
//	  if linear <= 0.0031308  →  linear * 12.92
//	  else                    →  1.055 * linear^(1/2.4) − 0.055
//	  multiply by 255, round, clamp to [0, 255]
package imagepointops

import (
	"math"

	pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

// ── sRGB / linear LUT ─────────────────────────────────────────────────────

// srgbToLinear is a 256-entry decode LUT: sRGB byte → linear float64.
// Built once at package init and reused for every decode call.
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

func decode(b byte) float64 { return srgbToLinear[b] }

func encode(linear float64) byte {
	var c float64
	if linear <= 0.0031308 {
		c = linear * 12.92
	} else {
		c = 1.055*math.Pow(linear, 1.0/2.4) - 0.055
	}
	v := math.Round(math.Min(1, math.Max(0, c)) * 255)
	return byte(v)
}

// ── Iteration helper ───────────────────────────────────────────────────────

type pixelFn func(r, g, b, a byte) (byte, byte, byte, byte)

func mapPixels(src *pc.PixelContainer, fn pixelFn) *pc.PixelContainer {
	out := pc.New(src.Width, src.Height)
	for i := 0; i < len(src.Data); i += 4 {
		r, g, b, a := src.Data[i], src.Data[i+1], src.Data[i+2], src.Data[i+3]
		or, og, ob, oa := fn(r, g, b, a)
		out.Data[i], out.Data[i+1], out.Data[i+2], out.Data[i+3] = or, og, ob, oa
	}
	return out
}

// ── u8-domain operations ───────────────────────────────────────────────────

// Invert flips each RGB channel (255 − v).  Alpha is preserved.
//
// Applying Invert twice returns the original image exactly because
// 255 − (255 − v) == v for all integers in [0, 255].
func Invert(src *pc.PixelContainer) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return 255 - r, 255 - g, 255 - b, a
	})
}

// Threshold binarises on average luminance.  Pixels with (r+g+b)/3 >= value
// become white; all others become black.  Alpha is preserved.
func Threshold(src *pc.PixelContainer, value byte) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		luma := (int(r) + int(g) + int(b)) / 3
		var v byte
		if luma >= int(value) {
			v = 255
		}
		return v, v, v, a
	})
}

// ThresholdLuminance binarises on Rec. 709 luma Y = 0.2126 R + 0.7152 G + 0.0722 B.
// More perceptually accurate than Threshold.
func ThresholdLuminance(src *pc.PixelContainer, value byte) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		luma := 0.2126*float64(r) + 0.7152*float64(g) + 0.0722*float64(b)
		var v byte
		if luma >= float64(value) {
			v = 255
		}
		return v, v, v, a
	})
}

// Posterize reduces each channel to `levels` equally-spaced steps.
//
// levels = 2 gives a high-contrast poster look.
// levels = 256 is the identity.
func Posterize(src *pc.PixelContainer, levels int) *pc.PixelContainer {
	step := 255.0 / float64(levels-1)
	q := func(v byte) byte {
		return byte(math.Round(math.Round(float64(v)/step) * step))
	}
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return q(r), q(g), q(b), a
	})
}

// SwapRGBBGR swaps R and B channels (RGB ↔ BGR).
// Useful when an upstream codec emits BGR byte order.
func SwapRGBBGR(src *pc.PixelContainer) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return b, g, r, a
	})
}

// Channel selects which channel to extract.
type Channel int

const (
	ChannelR Channel = iota
	ChannelG
	ChannelB
	ChannelA
)

// ExtractChannel keeps only the nominated channel, zeroing the others.
// Alpha is always preserved.
func ExtractChannel(src *pc.PixelContainer, ch Channel) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		switch ch {
		case ChannelR:
			return r, 0, 0, a
		case ChannelG:
			return 0, g, 0, a
		case ChannelB:
			return 0, 0, b, a
		default:
			return r, g, b, a
		}
	})
}

// Brightness adds a signed offset to each RGB channel, clamped to [0, 255].
// Alpha is preserved.  This is a u8-domain operation.
func Brightness(src *pc.PixelContainer, offset int) *pc.PixelContainer {
	clamp := func(v int) byte {
		if v < 0 {
			return 0
		}
		if v > 255 {
			return 255
		}
		return byte(v)
	}
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return clamp(int(r) + offset), clamp(int(g) + offset), clamp(int(b) + offset), a
	})
}

// ── Linear-light operations ────────────────────────────────────────────────

// Contrast scales each linear channel around mid-grey (0.5 linear).
//
// factor = 1.0 → identity; < 1.0 → less contrast; > 1.0 → more.
// Formula: linear_out = 0.5 + factor * (linear_in − 0.5)
func Contrast(src *pc.PixelContainer, factor float64) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return encode(0.5 + factor*(decode(r)-0.5)),
			encode(0.5 + factor*(decode(g)-0.5)),
			encode(0.5 + factor*(decode(b)-0.5)),
			a
	})
}

// Gamma applies a power-law γ to each linear channel.
//
// g < 1 → brightens; g > 1 → darkens; g = 1 → identity.
// Formula: linear_out = linear_in ^ g
func Gamma(src *pc.PixelContainer, g float64) *pc.PixelContainer {
	return mapPixels(src, func(r, gv, b, a byte) (byte, byte, byte, byte) {
		return encode(math.Pow(decode(r), g)),
			encode(math.Pow(decode(gv), g)),
			encode(math.Pow(decode(b), g)),
			a
	})
}

// Exposure multiplies linear luminance by 2^stops.
// +1 stop → double the light; −1 stop → halve it.
func Exposure(src *pc.PixelContainer, stops float64) *pc.PixelContainer {
	factor := math.Pow(2, stops)
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return encode(decode(r) * factor),
			encode(decode(g) * factor),
			encode(decode(b) * factor),
			a
	})
}

// GreyscaleMethod selects the luminance weighting scheme.
type GreyscaleMethod int

const (
	// Rec709 uses Y = 0.2126 R + 0.7152 G + 0.0722 B (perceptually correct).
	Rec709 GreyscaleMethod = iota
	// BT601 uses Y = 0.2989 R + 0.5870 G + 0.1140 B (legacy SD-TV).
	BT601
	// Average uses Y = (R + G + B) / 3 (equal weights, fast).
	Average
)

// Greyscale converts to luminance in linear light, then re-encodes to sRGB.
func Greyscale(src *pc.PixelContainer, method GreyscaleMethod) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		lr, lg, lb := decode(r), decode(g), decode(b)
		var y float64
		switch method {
		case Rec709:
			y = 0.2126*lr + 0.7152*lg + 0.0722*lb
		case BT601:
			y = 0.2989*lr + 0.5870*lg + 0.1140*lb
		default:
			y = (lr + lg + lb) / 3
		}
		out := encode(y)
		return out, out, out, a
	})
}

// Sepia applies a classic warm sepia tone matrix in linear light.
func Sepia(src *pc.PixelContainer) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		lr, lg, lb := decode(r), decode(g), decode(b)
		return encode(0.393*lr + 0.769*lg + 0.189*lb),
			encode(0.349*lr + 0.686*lg + 0.168*lb),
			encode(0.272*lr + 0.534*lg + 0.131*lb),
			a
	})
}

// ColourMatrix multiplies linear [R, G, B] by a 3×3 matrix.
//
// The matrix is row-major: matrix[0] is the output-R row, etc.
// Identity: [[1,0,0],[0,1,0],[0,0,1]].
func ColourMatrix(src *pc.PixelContainer, matrix [3][3]float64) *pc.PixelContainer {
	m := matrix
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		lr, lg, lb := decode(r), decode(g), decode(b)
		return encode(m[0][0]*lr + m[0][1]*lg + m[0][2]*lb),
			encode(m[1][0]*lr + m[1][1]*lg + m[1][2]*lb),
			encode(m[2][0]*lr + m[2][1]*lg + m[2][2]*lb),
			a
	})
}

// Saturate scales saturation in linear RGB.
//
// factor = 0 → greyscale; 1 → identity; > 1 → hypersaturated.
// Uses Rec. 709 luminance weights.
func Saturate(src *pc.PixelContainer, factor float64) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		lr, lg, lb := decode(r), decode(g), decode(b)
		grey := 0.2126*lr + 0.7152*lg + 0.0722*lb
		return encode(grey + factor*(lr-grey)),
			encode(grey + factor*(lg-grey)),
			encode(grey + factor*(lb-grey)),
			a
	})
}

// ── HSV helpers ────────────────────────────────────────────────────────────

func rgbToHSV(r, g, b float64) (h, s, v float64) {
	mx := math.Max(r, math.Max(g, b))
	mn := math.Min(r, math.Min(g, b))
	delta := mx - mn
	v = mx
	if mx == 0 {
		s = 0
	} else {
		s = delta / mx
	}
	if delta == 0 {
		h = 0
		return
	}
	switch mx {
	case r:
		h = math.Mod((g-b)/delta, 6)
	case g:
		h = (b-r)/delta + 2
	default:
		h = (r-g)/delta + 4
	}
	h = math.Mod(h*60+360, 360)
	return
}

func hsvToRGB(h, s, v float64) (r, g, b float64) {
	c := v * s
	x := c * (1 - math.Abs(math.Mod(h/60, 2)-1))
	m := v - c
	switch {
	case h < 60:
		r, g, b = c, x, 0
	case h < 120:
		r, g, b = x, c, 0
	case h < 180:
		r, g, b = 0, c, x
	case h < 240:
		r, g, b = 0, x, c
	case h < 300:
		r, g, b = x, 0, c
	default:
		r, g, b = c, 0, x
	}
	return r + m, g + m, b + m
}

// HueRotate rotates the hue of each pixel by degrees.
// 360° is an identity.
func HueRotate(src *pc.PixelContainer, degrees float64) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		h, s, v := rgbToHSV(decode(r), decode(g), decode(b))
		nr, ng, nb := hsvToRGB(math.Mod(h+degrees+360, 360), s, v)
		return encode(nr), encode(ng), encode(nb), a
	})
}

// ── Colorspace utilities ───────────────────────────────────────────────────

// SRGBToLinearImage converts each sRGB byte to linear * 255.
// Useful for arithmetic-on-bytes pipelines.
func SRGBToLinearImage(src *pc.PixelContainer) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return byte(math.Round(decode(r) * 255)),
			byte(math.Round(decode(g) * 255)),
			byte(math.Round(decode(b) * 255)),
			a
	})
}

// LinearToSRGBImage is the inverse of SRGBToLinearImage.
func LinearToSRGBImage(src *pc.PixelContainer) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return encode(float64(r) / 255),
			encode(float64(g) / 255),
			encode(float64(b) / 255),
			a
	})
}

// ── 1D LUT operations ──────────────────────────────────────────────────────

// ApplyLUT1DU8 applies three 256-entry u8→u8 LUTs (one per channel).
// Alpha is always preserved.
func ApplyLUT1DU8(src *pc.PixelContainer, lutR, lutG, lutB *[256]byte) *pc.PixelContainer {
	return mapPixels(src, func(r, g, b, a byte) (byte, byte, byte, byte) {
		return lutR[r], lutG[g], lutB[b], a
	})
}

// BuildLUT1DU8 builds a 256-entry LUT from a linear-light mapping function
// f: [0,1] → [0,1].
func BuildLUT1DU8(fn func(float64) float64) *[256]byte {
	lut := new([256]byte)
	for i := range lut {
		lut[i] = encode(fn(decode(byte(i))))
	}
	return lut
}

// BuildGammaLUT builds a gamma LUT (equivalent to BuildLUT1DU8(v => v^g)).
func BuildGammaLUT(g float64) *[256]byte {
	return BuildLUT1DU8(func(v float64) float64 { return math.Pow(v, g) })
}
