package code128

import "testing"

func TestCode128(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("unexpected version %s", Version)
	}
	values := make([]int, 0, len("Code 128"))
	for _, ch := range "Code 128" {
		values = append(values, ValueForCode128BChar(ch))
	}
	if checksum := ComputeCode128Checksum(values); checksum != 64 {
		t.Fatalf("unexpected checksum %d", checksum)
	}
	encoded, err := EncodeCode128B("Code 128")
	if err != nil {
		t.Fatal(err)
	}
	if encoded[0].Role != "start" || encoded[len(encoded)-2].Role != "check" || encoded[len(encoded)-1].Role != "stop" {
		t.Fatalf("unexpected encoded roles %#v", encoded)
	}
	scene, err := DrawCode128("Code 128", DefaultRenderConfig)
	if err != nil {
		t.Fatal(err)
	}
	if scene.Metadata["symbology"] != "code128" {
		t.Fatalf("unexpected scene metadata %#v", scene.Metadata)
	}
}
