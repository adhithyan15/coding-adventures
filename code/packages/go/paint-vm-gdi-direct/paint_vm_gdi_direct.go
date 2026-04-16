package paintvmgdidirect

import (
	"runtime"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

const Version = "0.1.0"

func SupportedRuntime() bool {
	return runtime.GOOS == "windows"
}

func Available() bool {
	return SupportedRuntime()
}

func Render(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	return render(scene)
}
