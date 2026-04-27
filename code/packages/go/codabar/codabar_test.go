package codabar

import "testing"

func TestCodabar(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("unexpected version %s", Version)
	}
	normalized, err := NormalizeCodabar("40156")
	if err != nil {
		t.Fatal(err)
	}
	if normalized != "A40156A" {
		t.Fatalf("unexpected normalized value %s", normalized)
	}
	runs, err := ExpandCodabarRuns("40156")
	if err != nil {
		t.Fatal(err)
	}
	foundGap := false
	for _, run := range runs {
		if run.Role == "inter-character-gap" {
			foundGap = true
			break
		}
	}
	if !foundGap {
		t.Fatal("expected an inter-character-gap run")
	}
	scene, err := DrawCodabar("40156", DefaultRenderConfig)
	if err != nil {
		t.Fatal(err)
	}
	if scene.Metadata["symbology"] != "codabar" {
		t.Fatalf("unexpected scene metadata %#v", scene.Metadata)
	}
}
