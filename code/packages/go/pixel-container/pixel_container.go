// Package pixelcontainer provides a fixed RGBA8 pixel buffer and the
// ImageCodec interface that every image-format encoder/decoder implements.
//
// # What Is a Pixel?
//
// A pixel (short for "picture element") is the smallest unit of a digital
// image. Each pixel stores colour as four independent channels:
//
//   - R — Red intensity, 0 (none) to 255 (full)
//   - G — Green intensity, 0 to 255
//   - B — Blue intensity, 0 to 255
//   - A — Alpha (opacity), 0 (transparent) to 255 (fully opaque)
//
// Combining these four values gives us 256⁴ ≈ 4.3 billion possible colours,
// which is why this encoding is called "32-bit RGBA" or "RGBA8".
//
// # Memory Layout
//
// Pixels are packed in row-major order (left-to-right, then top-to-bottom).
// For an image that is W pixels wide, the pixel at column x, row y starts at
// byte offset:
//
//	offset = (y*W + x) * 4
//
// The four bytes at that offset hold R, G, B, A in that order.
//
// # Why a Flat []byte?
//
// Using a single slice instead of a slice-of-slices ([][]byte) keeps the data
// contiguous in memory. Contiguous layouts are cache-friendly and much easier
// to pass to graphics APIs or write directly to a file.
package pixelcontainer

import "fmt"

// PixelContainer is a fixed RGBA8 pixel buffer.
//
// Width and Height record the image dimensions in pixels. Data holds Width *
// Height * 4 bytes: four bytes per pixel, in R-G-B-A order, row by row from
// the top-left corner.
//
// The zero value of PixelContainer is not useful; always construct one with
// New.
type PixelContainer struct {
	// Width is the number of pixel columns.
	Width uint32

	// Height is the number of pixel rows.
	Height uint32

	// Data is the flat byte slice. len(Data) == Width * Height * 4.
	Data []byte
}

// ImageCodec is the interface that every image-format encoder/decoder must
// satisfy.
//
// Separating the "what format" concern from the "pixel buffer" concern lets
// callers write generic code:
//
//	func save(img *PixelContainer, codec ImageCodec, path string) {
//	    bytes := codec.Encode(img)
//	    os.WriteFile(path, bytes, 0o644)
//	}
//
// The same save function works for BMP, PPM, QOI, PNG, or any future format
// as long as that format implements ImageCodec.
type ImageCodec interface {
	// MimeType returns the IANA media type for this format, e.g.
	// "image/bmp" or "image/x-portable-pixmap".
	MimeType() string

	// Encode serialises the pixel buffer to the codec's binary format.
	// The returned slice is a fresh allocation; the caller owns it.
	Encode(*PixelContainer) []byte

	// Decode parses encoded bytes and returns a freshly allocated
	// PixelContainer, or an error if the data is malformed.
	Decode([]byte) (*PixelContainer, error)
}

// New returns a zeroed PixelContainer with the given dimensions.
//
// All pixels start as (0, 0, 0, 0) — transparent black.
//
// Example:
//
//	img := pixelcontainer.New(640, 480)
//	// img is now a 640×480 blank canvas
func New(width, height uint32) *PixelContainer {
	// Allocate exactly width * height * 4 bytes, all zeroed.
	// Go's make zeroes the slice, so every channel starts at 0.
	return &PixelContainer{
		Width:  width,
		Height: height,
		Data:   make([]byte, width*height*4),
	}
}

// offset computes the byte index for the first channel (R) of pixel (x, y).
//
// The formula is the standard row-major index: row y starts at y*Width, then
// we step x columns, each column being 4 bytes wide.
func offset(c *PixelContainer, x, y uint32) uint32 {
	return (y*c.Width + x) * 4
}

// inBounds reports whether (x, y) is a valid pixel coordinate.
//
// A pixel is valid when 0 ≤ x < Width and 0 ≤ y < Height. Because x and y
// are unsigned, the lower-bound check (x >= 0) is always true — unsigned
// integers can never be negative in Go.
func inBounds(c *PixelContainer, x, y uint32) bool {
	return x < c.Width && y < c.Height
}

// PixelAt returns the RGBA values of the pixel at column x, row y.
//
// If (x, y) is outside the image bounds, all channels are returned as 0.
// This "null pixel" behaviour keeps callers simple — no error return needed
// for the common read path.
//
// Example:
//
//	r, g, b, a := pixelcontainer.PixelAt(img, 10, 20)
//	fmt.Printf("pixel at (10,20): r=%d g=%d b=%d a=%d\n", r, g, b, a)
func PixelAt(c *PixelContainer, x, y uint32) (r, g, b, a byte) {
	if !inBounds(c, x, y) {
		// Out-of-bounds reads silently return the zero pixel.
		return 0, 0, 0, 0
	}
	off := offset(c, x, y)
	return c.Data[off], c.Data[off+1], c.Data[off+2], c.Data[off+3]
}

// SetPixel writes RGBA values into the pixel at column x, row y.
//
// If (x, y) is outside the image bounds, the call is a no-op. This matches
// the silent-ignore convention in many graphics APIs (e.g. OpenGL fragment
// discarding) and avoids cluttering hot loops with error checks.
//
// Example:
//
//	// Paint pixel (5, 5) solid red
//	pixelcontainer.SetPixel(img, 5, 5, 255, 0, 0, 255)
func SetPixel(c *PixelContainer, x, y uint32, r, g, b, a byte) {
	if !inBounds(c, x, y) {
		return
	}
	off := offset(c, x, y)
	c.Data[off] = r
	c.Data[off+1] = g
	c.Data[off+2] = b
	c.Data[off+3] = a
}

// FillPixels sets every pixel in the buffer to the same RGBA value.
//
// This is faster than looping over SetPixel because it avoids the per-pixel
// bounds check. Typical uses:
//
//   - Fill with (0, 0, 0, 255) for a solid black background.
//   - Fill with (255, 255, 255, 255) for a solid white background.
//   - Fill with (0, 0, 0, 0) to reset to transparent black.
//
// Example:
//
//	// Make the whole image solid white
//	pixelcontainer.FillPixels(img, 255, 255, 255, 255)
func FillPixels(c *PixelContainer, r, g, b, a byte) {
	// Write four channels for each pixel, advancing by 4 bytes each time.
	// Iterating over Data directly is as fast as a plain for loop over indices.
	for i := 0; i < len(c.Data); i += 4 {
		c.Data[i] = r
		c.Data[i+1] = g
		c.Data[i+2] = b
		c.Data[i+3] = a
	}
}

// Validate checks that a PixelContainer is internally consistent.
//
// It confirms that len(Data) == Width * Height * 4. Codec implementations
// can call this after construction to catch programming errors early.
func Validate(c *PixelContainer) error {
	expected := uint64(c.Width) * uint64(c.Height) * 4
	if uint64(len(c.Data)) != expected {
		return fmt.Errorf(
			"pixelcontainer: data length %d does not match width %d * height %d * 4 = %d",
			len(c.Data), c.Width, c.Height, expected,
		)
	}
	return nil
}
