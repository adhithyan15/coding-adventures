package intel4004packager

import "testing"

func TestEncodeDecodeRoundTrip(t *testing.T) {
	text, err := EncodeHex([]byte{0xD5, 0xB2, 0x01}, 0)
	if err != nil {
		t.Fatalf("encode failed: %v", err)
	}
	decoded, err := DecodeHex(text)
	if err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	if decoded.Origin != 0 || len(decoded.Binary) != 3 {
		t.Fatalf("unexpected decode result: %#v", decoded)
	}
}
