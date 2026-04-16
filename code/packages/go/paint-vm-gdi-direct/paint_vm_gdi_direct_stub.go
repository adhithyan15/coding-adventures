//go:build !windows

package paintvmgdidirect

import (
	"fmt"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func render(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	return nil, fmt.Errorf("paint-vm-gdi-direct is only available on Windows")
}
