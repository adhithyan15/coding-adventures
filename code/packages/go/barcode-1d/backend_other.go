//go:build !(windows || (darwin && arm64))

package barcode1d

import (
	paintcodecpng "github.com/adhithyan15/coding-adventures/code/packages/go/paint-codec-png"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	paintvmraster "github.com/adhithyan15/coding-adventures/code/packages/go/paint-vm-raster"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func renderPixelsForCurrentBackend(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	return paintvmraster.Render(scene)
}

func renderPNGForCurrentBackend(scene paintinstructions.PaintScene) ([]byte, error) {
	pixels, err := renderPixelsForCurrentBackend(scene)
	if err != nil {
		return nil, err
	}
	return paintcodecpng.EncodePNG(pixels)
}
