// Package paintcodecpng encodes and decodes PixelContainer values as PNG.
package paintcodecpng

import (
	imagecodecpng "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-png"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

const Version = "0.2.0"

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
	return imagecodecpng.EncodePNG(c)
}

func Decode(data []byte) (*pixelcontainer.PixelContainer, error) {
	return DecodePNG(data)
}

// DecodePNG decodes PNG bytes into a PixelContainer.
func DecodePNG(data []byte) (*pixelcontainer.PixelContainer, error) {
	return imagecodecpng.DecodePNG(data)
}
