package main

import (
	"os"

	barcode1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-1d"
)

func main() {
	outputPath := os.Getenv("BARCODE_1D_OUTPUT")
	if outputPath == "" {
		outputPath = "go-code39.png"
	}

	pngBytes, err := barcode1d.RenderPNG("HELLO-123", nil)
	if err != nil {
		panic(err)
	}
	if err := os.WriteFile(outputPath, pngBytes, 0o644); err != nil {
		panic(err)
	}
}
