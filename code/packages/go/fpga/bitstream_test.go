package fpga

import (
	"os"
	"path/filepath"
	"testing"
)

// =========================================================================
// Bitstream Tests
// =========================================================================

func TestFromMap_Basic(t *testing.T) {
	andTT := make([]int, 16)
	andTT[3] = 1
	zeroTT := make([]int, 16)

	bs := FromMap(
		map[string]CLBConfig{
			"clb_0": {
				Slice0: SliceConfig{LutA: andTT, LutB: zeroTT},
				Slice1: SliceConfig{LutA: zeroTT, LutB: zeroTT},
			},
		},
		map[string][]RouteConfig{
			"sw_0": {{Source: "a", Destination: "b"}},
		},
		map[string]IOConfig{
			"pin_A": {Mode: "input"},
			"pin_B": {Mode: "output"},
		},
		4,
	)

	if bs.LutK != 4 {
		t.Errorf("LutK = %d, want 4", bs.LutK)
	}
	if len(bs.CLBs) != 1 {
		t.Errorf("CLBs count = %d, want 1", len(bs.CLBs))
	}
	if len(bs.Routing) != 1 {
		t.Errorf("Routing count = %d, want 1", len(bs.Routing))
	}
	if len(bs.IO) != 2 {
		t.Errorf("IO count = %d, want 2", len(bs.IO))
	}
}

func TestFromMap_Defaults(t *testing.T) {
	bs := FromMap(nil, nil, nil, 0)
	if bs.LutK != 4 {
		t.Errorf("Default LutK = %d, want 4", bs.LutK)
	}
	if bs.CLBs == nil {
		t.Error("CLBs should not be nil")
	}
	if bs.Routing == nil {
		t.Error("Routing should not be nil")
	}
	if bs.IO == nil {
		t.Error("IO should not be nil")
	}
}

func TestFromJSONBytes_Basic(t *testing.T) {
	jsonData := []byte(`{
		"lut_k": 4,
		"clbs": {
			"clb_0": {
				"slice0": {
					"lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0],
					"lut_b": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
					"ff_a": true,
					"ff_b": false,
					"carry": false
				}
			}
		},
		"routing": {
			"sw_0": [
				{"src": "clb_out", "dst": "east"}
			]
		},
		"io": {
			"pin_A": {"mode": "input"},
			"pin_B": {"mode": "output"}
		}
	}`)

	bs, err := FromJSONBytes(jsonData)
	if err != nil {
		t.Fatalf("FromJSONBytes error: %v", err)
	}

	if bs.LutK != 4 {
		t.Errorf("LutK = %d, want 4", bs.LutK)
	}

	clb, ok := bs.CLBs["clb_0"]
	if !ok {
		t.Fatal("CLB 'clb_0' not found")
	}
	if clb.Slice0.LutA[3] != 1 {
		t.Errorf("Slice0.LutA[3] = %d, want 1", clb.Slice0.LutA[3])
	}
	if !clb.Slice0.FFAEnabled {
		t.Error("Slice0.FFAEnabled should be true")
	}

	routes, ok := bs.Routing["sw_0"]
	if !ok || len(routes) != 1 {
		t.Fatal("Routing 'sw_0' not found or wrong length")
	}
	if routes[0].Source != "clb_out" || routes[0].Destination != "east" {
		t.Errorf("Route = %v, want src=clb_out dst=east", routes[0])
	}
}

func TestFromJSONBytes_DefaultLutK(t *testing.T) {
	jsonData := []byte(`{"clbs": {}}`)
	bs, err := FromJSONBytes(jsonData)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	if bs.LutK != 4 {
		t.Errorf("Default LutK = %d, want 4", bs.LutK)
	}
}

func TestFromJSONBytes_MissingSliceFields(t *testing.T) {
	// slice0 exists but has no lut_a/lut_b → should use defaults
	jsonData := []byte(`{
		"clbs": {
			"clb_0": {
				"slice0": {"ff_a": true}
			}
		}
	}`)

	bs, err := FromJSONBytes(jsonData)
	if err != nil {
		t.Fatalf("error: %v", err)
	}

	clb := bs.CLBs["clb_0"]
	if len(clb.Slice0.LutA) != 16 {
		t.Errorf("Default LutA length = %d, want 16", len(clb.Slice0.LutA))
	}
	if len(clb.Slice1.LutA) != 16 {
		t.Errorf("Default Slice1 LutA length = %d, want 16", len(clb.Slice1.LutA))
	}
}

func TestFromJSONBytes_EmptyJSON(t *testing.T) {
	bs, err := FromJSONBytes([]byte(`{}`))
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	if len(bs.CLBs) != 0 {
		t.Errorf("CLBs count = %d, want 0", len(bs.CLBs))
	}
}

func TestFromJSONBytes_InvalidJSON(t *testing.T) {
	_, err := FromJSONBytes([]byte(`{invalid`))
	if err == nil {
		t.Error("expected error for invalid JSON")
	}
}

func TestFromJSON_File(t *testing.T) {
	// Create a temp file with JSON content
	dir := t.TempDir()
	path := filepath.Join(dir, "test.json")

	jsonData := []byte(`{
		"lut_k": 4,
		"clbs": {},
		"io": {"pin_0": {"mode": "input"}}
	}`)

	if err := os.WriteFile(path, jsonData, 0644); err != nil {
		t.Fatalf("Failed to write temp file: %v", err)
	}

	bs, err := FromJSON(path)
	if err != nil {
		t.Fatalf("FromJSON error: %v", err)
	}

	if len(bs.IO) != 1 {
		t.Errorf("IO count = %d, want 1", len(bs.IO))
	}
}

func TestFromJSON_FileNotFound(t *testing.T) {
	_, err := FromJSON("/nonexistent/path.json")
	if err == nil {
		t.Error("expected error for nonexistent file")
	}
}
