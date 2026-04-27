package barcode1d

import (
	"bytes"
	"testing"
)

func TestBuildSceneUsesDefaultCode39(t *testing.T) {
	scene, err := BuildScene("HELLO-123", nil)
	if err != nil {
		t.Fatal(err)
	}
	if scene.Metadata["symbology"] != "code39" {
		t.Fatalf("unexpected symbology metadata: %#v", scene.Metadata)
	}
	if scene.Height != DefaultLayoutConfig.BarHeight {
		t.Fatalf("unexpected scene height: %d", scene.Height)
	}
}

func TestCurrentBackendProbe(t *testing.T) {
	backend := CurrentBackend()
	if backend != "metal" && backend != "gdi" && backend != "raster" {
		t.Fatalf("unexpected backend: %s", backend)
	}
}

func TestRenderPNGForCodabar(t *testing.T) {
	pngBytes, err := RenderPNG("40156", &Options{Symbology: "codabar"})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasPrefix(pngBytes, []byte{0x89, 'P', 'N', 'G'}) {
		t.Fatalf("expected PNG signature, got %v", pngBytes[:4])
	}
}

func TestRenderPixelsForEAN13(t *testing.T) {
	pixels, err := RenderPixels("400638133393", &Options{Symbology: "ean-13"})
	if err != nil {
		t.Fatal(err)
	}
	if pixels.Width == 0 || pixels.Height == 0 {
		t.Fatalf("unexpected empty image: %dx%d", pixels.Width, pixels.Height)
	}
}

func TestUnsupportedSymbology(t *testing.T) {
	if _, err := BuildScene("HELLO-123", &Options{Symbology: "qr"}); err == nil {
		t.Fatal("expected unsupported symbology error")
	}
}
