package barcodelayout1d

import "testing"

func TestRunsFromBinaryPattern(t *testing.T) {
	runs, err := RunsFromBinaryPattern("111001", '1', '0', "", 0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(runs) != 3 {
		t.Fatalf("expected 3 runs, got %d", len(runs))
	}
	if runs[0].Color != "bar" || runs[0].Modules != 3 {
		t.Fatalf("unexpected first run: %#v", runs[0])
	}
}

func TestLayoutBarcode1D(t *testing.T) {
	runs, err := RunsFromWidthPattern("WNW", []string{"bar", "space", "bar"}, "A", 0, 1, 3, "data", nil)
	if err != nil {
		t.Fatal(err)
	}
	scene, err := LayoutBarcode1D(runs, DefaultLayoutConfig, DefaultPaintOptions)
	if err != nil {
		t.Fatal(err)
	}
	if scene.Width != 27*DefaultLayoutConfig.ModuleUnit {
		t.Fatalf("unexpected scene width: %d", scene.Width)
	}
	if scene.Height != DefaultLayoutConfig.BarHeight {
		t.Fatalf("unexpected scene height: %d", scene.Height)
	}
}
