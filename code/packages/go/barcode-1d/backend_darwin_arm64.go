//go:build darwin && arm64

package barcode1d

import (
	paintcodecpngnative "github.com/adhithyan15/coding-adventures/code/packages/go/paint-codec-png-native"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	paintvmmetalnative "github.com/adhithyan15/coding-adventures/code/packages/go/paint-vm-metal-native"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func renderPixelsForCurrentBackend(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	return paintvmmetalnative.Render(scene)
}

func renderPNGForCurrentBackend(scene paintinstructions.PaintScene) ([]byte, error) {
	pixels, err := renderPixelsForCurrentBackend(scene)
	if err != nil {
		return nil, err
	}
	return paintcodecpngnative.Encode(pixels)
}
