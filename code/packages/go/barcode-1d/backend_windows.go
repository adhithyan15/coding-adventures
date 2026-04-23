//go:build windows

package barcode1d

import (
	paintcodecpng "github.com/adhithyan15/coding-adventures/code/packages/go/paint-codec-png"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	paintvmgdidirect "github.com/adhithyan15/coding-adventures/code/packages/go/paint-vm-gdi-direct"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func renderPixelsForCurrentBackend(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	return paintvmgdidirect.Render(scene)
}

func renderPNGForCurrentBackend(scene paintinstructions.PaintScene) ([]byte, error) {
	pixels, err := renderPixelsForCurrentBackend(scene)
	if err != nil {
		return nil, err
	}
	return paintcodecpng.Encode(pixels)
}
