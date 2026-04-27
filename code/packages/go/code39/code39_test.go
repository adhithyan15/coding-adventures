package code39

import "testing"

func TestEncodeAndDraw(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("unexpected version %s", Version)
	}
	encoded, err := EncodeCode39("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(encoded) != 3 || encoded[1].Pattern != "WNNNNWNNW" {
		t.Fatalf("unexpected encoding: %#v", encoded)
	}
	runs, err := ExpandCode39Runs("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(runs) != 29 {
		t.Fatalf("expected 29 runs, got %d", len(runs))
	}
	if runs[9].Role != "inter-character-gap" {
		t.Fatalf("unexpected run role: %#v", runs[9])
	}
	scene, err := DrawCode39("A", DefaultRenderConfig)
	if err != nil {
		t.Fatal(err)
	}
	if scene.Metadata["symbology"] != "code39" {
		t.Fatalf("unexpected scene metadata: %#v", scene.Metadata)
	}
	if scene.Height != DefaultRenderConfig.BarHeight {
		t.Fatalf("unexpected scene height: %d", scene.Height)
	}
}
