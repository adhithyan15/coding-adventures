// Package paintcodecpng encodes and decodes PixelContainer values as PNG.
package paintcodecpng

import (
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/png"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

const Version = "0.1.0"

type PngCodec struct{}
type Codec = PngCodec

func (PngCodec) MimeType() string { return "image/png" }

func (PngCodec) Encode(c *pixelcontainer.PixelContainer) []byte {
	encoded, err := EncodePNG(c)
	if err != nil {
		panic(err)
	}
	return encoded
}

func (PngCodec) Decode(data []byte) (*pixelcontainer.PixelContainer, error) {
	return DecodePNG(data)
}

func Encode(c *pixelcontainer.PixelContainer) ([]byte, error) {
	return EncodePNG(c)
}

// EncodePNG serializes a PixelContainer into PNG bytes.
func EncodePNG(c *pixelcontainer.PixelContainer) ([]byte, error) {
	if c == nil {
		return nil, fmt.Errorf("pixel container must not be nil")
	}
	if err := pixelcontainer.Validate(c); err != nil {
		return nil, err
	}

	imageData := image.NewNRGBA(image.Rect(0, 0, int(c.Width), int(c.Height)))
	copy(imageData.Pix, c.Data)

	var buffer bytes.Buffer
	if err := png.Encode(&buffer, imageData); err != nil {
		return nil, err
	}
	return buffer.Bytes(), nil
}

func Decode(data []byte) (*pixelcontainer.PixelContainer, error) {
	return DecodePNG(data)
}

// DecodePNG decodes PNG bytes into a PixelContainer.
func DecodePNG(data []byte) (*pixelcontainer.PixelContainer, error) {
	img, err := png.Decode(bytes.NewReader(data))
	if err != nil {
		return nil, err
	}

	bounds := img.Bounds()
	width := bounds.Dx()
	height := bounds.Dy()
	result := pixelcontainer.New(uint32(width), uint32(height))

	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			nrgba := color.NRGBAModel.Convert(img.At(bounds.Min.X+x, bounds.Min.Y+y)).(color.NRGBA)
			pixelcontainer.SetPixel(result, uint32(x), uint32(y), nrgba.R, nrgba.G, nrgba.B, nrgba.A)
		}
	}

	return result, nil
}
