package zip

import (
	"bytes"
	"testing"
)

func TestLegacyPrivateDeflateWrapperUsesStrictCore(t *testing.T) {
	want := []byte("legacy compatibility")
	got, err := deflateDecompress(deflateCompress(want))
	if err != nil || !bytes.Equal(got, want) {
		t.Fatalf("legacy private wrapper: %v", err)
	}
}
