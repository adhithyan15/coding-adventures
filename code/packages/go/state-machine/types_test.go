package statemachine

import "testing"

// =========================================================================
// Tests for core types
// =========================================================================
//
// The types in this package are mostly simple structs and type aliases,
// so these tests focus on verifying that TransitionRecord fields work
// correctly and that the type aliases are usable as expected.

func TestTransitionRecordFields(t *testing.T) {
	tests := []struct {
		name       string
		record     TransitionRecord
		wantSource string
		wantEvent  string
		wantTarget string
		wantAction string
	}{
		{
			name:       "basic transition",
			record:     TransitionRecord{Source: "locked", Event: "coin", Target: "unlocked", ActionName: ""},
			wantSource: "locked",
			wantEvent:  "coin",
			wantTarget: "unlocked",
			wantAction: "",
		},
		{
			name:       "transition with action",
			record:     TransitionRecord{Source: "q0", Event: "a", Target: "q1", ActionName: "logTransition"},
			wantSource: "q0",
			wantEvent:  "a",
			wantTarget: "q1",
			wantAction: "logTransition",
		},
		{
			name:       "epsilon transition (empty event)",
			record:     TransitionRecord{Source: "q0", Event: "", Target: "q1", ActionName: ""},
			wantSource: "q0",
			wantEvent:  "",
			wantTarget: "q1",
			wantAction: "",
		},
		{
			name:       "self loop",
			record:     TransitionRecord{Source: "idle", Event: "nop", Target: "idle", ActionName: ""},
			wantSource: "idle",
			wantEvent:  "nop",
			wantTarget: "idle",
			wantAction: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.record.Source != tt.wantSource {
				t.Errorf("Source = %q, want %q", tt.record.Source, tt.wantSource)
			}
			if tt.record.Event != tt.wantEvent {
				t.Errorf("Event = %q, want %q", tt.record.Event, tt.wantEvent)
			}
			if tt.record.Target != tt.wantTarget {
				t.Errorf("Target = %q, want %q", tt.record.Target, tt.wantTarget)
			}
			if tt.record.ActionName != tt.wantAction {
				t.Errorf("ActionName = %q, want %q", tt.record.ActionName, tt.wantAction)
			}
		})
	}
}

func TestStateAndEventAreStrings(t *testing.T) {
	// Type aliases should be usable as plain strings
	var s State = "q0"
	var e Event = "coin"

	if s != "q0" {
		t.Errorf("State value = %q, want %q", s, "q0")
	}
	if e != "coin" {
		t.Errorf("Event value = %q, want %q", e, "coin")
	}

	// Should be assignable to string
	var str string = s
	if str != "q0" {
		t.Errorf("string(State) = %q, want %q", str, "q0")
	}
}

func TestActionCallable(t *testing.T) {
	// Verify that Action type can be used as a callback
	var called bool
	var capturedSource, capturedEvent, capturedTarget string

	var action Action = func(source, event, target string) {
		called = true
		capturedSource = source
		capturedEvent = event
		capturedTarget = target
	}

	action("q0", "a", "q1")

	if !called {
		t.Error("Action was not called")
	}
	if capturedSource != "q0" {
		t.Errorf("source = %q, want %q", capturedSource, "q0")
	}
	if capturedEvent != "a" {
		t.Errorf("event = %q, want %q", capturedEvent, "a")
	}
	if capturedTarget != "q1" {
		t.Errorf("target = %q, want %q", capturedTarget, "q1")
	}
}
