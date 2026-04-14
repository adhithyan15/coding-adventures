package paintcodecpngnative

import (
	"bytes"
	"testing"

	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

func TestSupportedRuntimeMatchesAvailability(t *testing.T) {
	if SupportedRuntime() != Available() {
		t.Fatalf("SupportedRuntime and Available should agree")
	}
}

func TestEncode(t *testing.T) {
	if !Available() {
		return
	}

	pixels := pixelcontainer.New(1, 1)
	pixelcontainer.SetPixel(pixels, 0, 0, 0, 0, 0, 255)

	encoded, err := Encode(pixels)
	if err != nil {
		t.Fatalf("Encode returned error: %v", err)
	}
	if !bytes.Equal(encoded[:8], []byte{0x89, 'P', 'N', 'G', '\r', '\n', 0x1a, '\n'}) {
		t.Fatalf("unexpected PNG signature: %v", encoded[:8])
	}
}
