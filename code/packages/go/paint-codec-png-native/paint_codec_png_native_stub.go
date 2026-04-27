//go:build !(darwin && arm64)

package paintcodecpngnative

import (
	"fmt"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func encode(pixels *pixelcontainer.PixelContainer) ([]byte, error) {
	return nil, fmt.Errorf("paint-codec-png-native is not available on this host")
}
