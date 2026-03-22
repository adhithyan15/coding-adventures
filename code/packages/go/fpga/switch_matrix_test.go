package fpga

import (
	"testing"
)

// =========================================================================
// SwitchMatrix Tests
// =========================================================================

func TestSwitchMatrix_ConnectAndRoute(t *testing.T) {
	sm := NewSwitchMatrix([]string{"north", "south", "east", "west", "clb_out"})

	sm.Connect("clb_out", "east")
	sm.Connect("north", "south")

	outputs := sm.Route(map[string]int{"clb_out": 1, "north": 0})
	if outputs["east"] != 1 {
		t.Errorf("Route east = %d, want 1", outputs["east"])
	}
	if outputs["south"] != 0 {
		t.Errorf("Route south = %d, want 0", outputs["south"])
	}
}

func TestSwitchMatrix_RouteUnconnected(t *testing.T) {
	sm := NewSwitchMatrix([]string{"a", "b", "c"})
	sm.Connect("a", "b")

	outputs := sm.Route(map[string]int{"a": 1})
	if _, ok := outputs["c"]; ok {
		t.Error("Unconnected port 'c' should not appear in outputs")
	}
}

func TestSwitchMatrix_RouteNoInput(t *testing.T) {
	sm := NewSwitchMatrix([]string{"a", "b"})
	sm.Connect("a", "b")

	// Source 'a' not in inputs → 'b' should not appear in outputs
	outputs := sm.Route(map[string]int{"b": 1})
	if _, ok := outputs["b"]; ok {
		t.Error("No source for 'b' in route, should not appear")
	}
}

func TestSwitchMatrix_Disconnect(t *testing.T) {
	sm := NewSwitchMatrix([]string{"a", "b", "c"})
	sm.Connect("a", "b")

	if sm.ConnectionCount() != 1 {
		t.Errorf("ConnectionCount() = %d, want 1", sm.ConnectionCount())
	}

	sm.Disconnect("b")

	if sm.ConnectionCount() != 0 {
		t.Errorf("After disconnect: ConnectionCount() = %d, want 0", sm.ConnectionCount())
	}

	outputs := sm.Route(map[string]int{"a": 1})
	if len(outputs) != 0 {
		t.Errorf("After disconnect: Route outputs = %v, want empty", outputs)
	}
}

func TestSwitchMatrix_Clear(t *testing.T) {
	sm := NewSwitchMatrix([]string{"a", "b", "c"})
	sm.Connect("a", "b")
	sm.Connect("a", "c")

	sm.Clear()

	if sm.ConnectionCount() != 0 {
		t.Errorf("After clear: ConnectionCount() = %d, want 0", sm.ConnectionCount())
	}
}

func TestSwitchMatrix_FanOut(t *testing.T) {
	// One source can drive multiple destinations
	sm := NewSwitchMatrix([]string{"src", "dst1", "dst2"})
	sm.Connect("src", "dst1")
	sm.Connect("src", "dst2")

	outputs := sm.Route(map[string]int{"src": 1})
	if outputs["dst1"] != 1 {
		t.Errorf("FanOut dst1 = %d, want 1", outputs["dst1"])
	}
	if outputs["dst2"] != 1 {
		t.Errorf("FanOut dst2 = %d, want 1", outputs["dst2"])
	}
}

func TestSwitchMatrix_Properties(t *testing.T) {
	sm := NewSwitchMatrix([]string{"a", "b", "c"})
	ports := sm.Ports()
	if len(ports) != 3 {
		t.Errorf("Ports count = %d, want 3", len(ports))
	}
	if !ports["a"] || !ports["b"] || !ports["c"] {
		t.Error("Missing expected port")
	}

	sm.Connect("a", "b")
	conns := sm.Connections()
	if conns["b"] != "a" {
		t.Errorf("Connections[b] = %q, want 'a'", conns["b"])
	}
}

func TestSwitchMatrix_Invalid(t *testing.T) {
	assertPanics(t, "empty ports", func() { NewSwitchMatrix([]string{}) })
	assertPanics(t, "empty string port", func() { NewSwitchMatrix([]string{""}) })

	sm := NewSwitchMatrix([]string{"a", "b", "c"})
	assertPanics(t, "unknown source", func() { sm.Connect("x", "a") })
	assertPanics(t, "unknown dest", func() { sm.Connect("a", "x") })
	assertPanics(t, "self connect", func() { sm.Connect("a", "a") })

	sm.Connect("a", "b")
	assertPanics(t, "already connected", func() { sm.Connect("c", "b") })

	assertPanics(t, "disconnect unknown", func() { sm.Disconnect("x") })
	assertPanics(t, "disconnect not connected", func() { sm.Disconnect("c") })
}
