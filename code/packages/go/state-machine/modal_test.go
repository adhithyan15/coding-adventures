package statemachine

import "testing"

// =========================================================================
// Modal State Machine Test helpers
// =========================================================================

// makeDataModeDFA creates a DFA for HTML "data" mode.
//
//	text --char--> text
//	text --open_angle--> tag_start
//	tag_start --char--> text
//	tag_start --open_angle--> tag_start
func makeDataModeDFA() *DFA {
	return NewDFA(
		[]string{"text", "tag_start"},
		[]string{"char", "open_angle"},
		map[[2]string]string{
			{"text", "char"}:            "text",
			{"text", "open_angle"}:      "tag_start",
			{"tag_start", "char"}:       "text",
			{"tag_start", "open_angle"}: "tag_start",
		},
		"text",
		[]string{"text"},
		nil,
	)
}

// makeTagModeDFA creates a DFA for HTML "tag" mode.
//
//	name --char--> name
//	name --close_angle--> done
//	done --char--> name
//	done --close_angle--> done
func makeTagModeDFA() *DFA {
	return NewDFA(
		[]string{"name", "done"},
		[]string{"char", "close_angle"},
		map[[2]string]string{
			{"name", "char"}:         "name",
			{"name", "close_angle"}:  "done",
			{"done", "char"}:         "name",
			{"done", "close_angle"}:  "done",
		},
		"name",
		[]string{"done"},
		nil,
	)
}

// makeHTMLModal creates a modal state machine mimicking HTML tokenizing.
func makeHTMLModal() *ModalStateMachine {
	return NewModalStateMachine(
		map[string]*DFA{
			"data": makeDataModeDFA(),
			"tag":  makeTagModeDFA(),
		},
		map[[2]string]string{
			{"data", "enter_tag"}: "tag",
			{"tag", "exit_tag"}:   "data",
		},
		"data",
	)
}

// makeThreeMode creates a modal with three modes for more complex testing.
func makeThreeMode() *ModalStateMachine {
	simpleDFA := func(name string) *DFA {
		return NewDFA(
			[]string{"idle", "active"},
			[]string{"go", "stop"},
			map[[2]string]string{
				{"idle", "go"}:     "active",
				{"idle", "stop"}:   "idle",
				{"active", "go"}:   "active",
				{"active", "stop"}: "idle",
			},
			"idle",
			[]string{"active"},
			nil,
		)
	}

	return NewModalStateMachine(
		map[string]*DFA{
			"normal":  simpleDFA("normal"),
			"insert":  simpleDFA("insert"),
			"visual":  simpleDFA("visual"),
		},
		map[[2]string]string{
			{"normal", "i"}:  "insert",
			{"normal", "v"}:  "visual",
			{"insert", "esc"}: "normal",
			{"visual", "esc"}: "normal",
		},
		"normal",
	)
}

// =========================================================================
// Construction tests
// =========================================================================

func TestNewModalStateMachine_Valid(t *testing.T) {
	m := makeHTMLModal()

	if m.CurrentMode() != "data" {
		t.Errorf("initial mode = %q, want %q", m.CurrentMode(), "data")
	}

	modes := m.Modes()
	if len(modes) != 2 {
		t.Errorf("len(Modes) = %d, want 2", len(modes))
	}
}

func TestNewModalStateMachine_NoModes(t *testing.T) {
	assertPanics(t, "no modes", "at least one mode", func() {
		NewModalStateMachine(map[string]*DFA{}, nil, "data")
	})
}

func TestNewModalStateMachine_InvalidInitialMode(t *testing.T) {
	assertPanics(t, "invalid initial mode", "initial mode", func() {
		NewModalStateMachine(
			map[string]*DFA{"data": makeDataModeDFA()},
			nil,
			"bad",
		)
	})
}

func TestNewModalStateMachine_InvalidTransitionSource(t *testing.T) {
	assertPanics(t, "invalid transition source", "mode transition source", func() {
		NewModalStateMachine(
			map[string]*DFA{"data": makeDataModeDFA()},
			map[[2]string]string{{"bad", "x"}: "data"},
			"data",
		)
	})
}

func TestNewModalStateMachine_InvalidTransitionTarget(t *testing.T) {
	assertPanics(t, "invalid transition target", "mode transition target", func() {
		NewModalStateMachine(
			map[string]*DFA{"data": makeDataModeDFA()},
			map[[2]string]string{{"data", "x"}: "bad"},
			"data",
		)
	})
}

// =========================================================================
// SwitchMode tests
// =========================================================================

func TestModal_SwitchMode(t *testing.T) {
	m := makeHTMLModal()

	newMode := m.SwitchMode("enter_tag")
	if newMode != "tag" {
		t.Errorf("SwitchMode = %q, want %q", newMode, "tag")
	}
	if m.CurrentMode() != "tag" {
		t.Errorf("CurrentMode = %q, want %q", m.CurrentMode(), "tag")
	}
}

func TestModal_SwitchMode_InvalidTrigger(t *testing.T) {
	m := makeHTMLModal()
	assertPanics(t, "invalid trigger", "no mode transition", func() {
		m.SwitchMode("bad_trigger")
	})
}

func TestModal_SwitchMode_ResetsTargetDFA(t *testing.T) {
	m := makeHTMLModal()

	// Process some events in data mode to change its state
	m.Process("open_angle")

	// Switch to tag mode, then back
	m.SwitchMode("enter_tag")
	m.SwitchMode("exit_tag")

	// Data mode DFA should be reset to initial state
	if m.ActiveMachine().CurrentState() != "text" {
		t.Errorf("after switch back, DFA state = %q, want %q",
			m.ActiveMachine().CurrentState(), "text")
	}
}

func TestModal_SwitchMode_Trace(t *testing.T) {
	m := makeHTMLModal()

	m.SwitchMode("enter_tag")
	m.SwitchMode("exit_tag")

	trace := m.ModeTrace()
	if len(trace) != 2 {
		t.Fatalf("ModeTrace length = %d, want 2", len(trace))
	}

	if trace[0].FromMode != "data" || trace[0].Trigger != "enter_tag" || trace[0].ToMode != "tag" {
		t.Errorf("trace[0] = %+v, want data->enter_tag->tag", trace[0])
	}
	if trace[1].FromMode != "tag" || trace[1].Trigger != "exit_tag" || trace[1].ToMode != "data" {
		t.Errorf("trace[1] = %+v, want tag->exit_tag->data", trace[1])
	}
}

func TestModal_SwitchMode_ThreeMode(t *testing.T) {
	m := makeThreeMode()

	m.SwitchMode("i")
	if m.CurrentMode() != "insert" {
		t.Errorf("after 'i': mode = %q, want insert", m.CurrentMode())
	}

	m.SwitchMode("esc")
	if m.CurrentMode() != "normal" {
		t.Errorf("after 'esc': mode = %q, want normal", m.CurrentMode())
	}

	m.SwitchMode("v")
	if m.CurrentMode() != "visual" {
		t.Errorf("after 'v': mode = %q, want visual", m.CurrentMode())
	}

	m.SwitchMode("esc")
	if m.CurrentMode() != "normal" {
		t.Errorf("after second 'esc': mode = %q, want normal", m.CurrentMode())
	}
}

// =========================================================================
// Process tests
// =========================================================================

func TestModal_Process(t *testing.T) {
	m := makeHTMLModal()

	// Process in data mode
	state := m.Process("char")
	if state != "text" {
		t.Errorf("Process('char') in data mode = %q, want text", state)
	}

	state = m.Process("open_angle")
	if state != "tag_start" {
		t.Errorf("Process('open_angle') = %q, want tag_start", state)
	}
}

func TestModal_Process_InTagMode(t *testing.T) {
	m := makeHTMLModal()
	m.SwitchMode("enter_tag")

	state := m.Process("char")
	if state != "name" {
		t.Errorf("Process('char') in tag mode = %q, want name", state)
	}

	state = m.Process("close_angle")
	if state != "done" {
		t.Errorf("Process('close_angle') = %q, want done", state)
	}
}

func TestModal_Process_InvalidEvent(t *testing.T) {
	m := makeHTMLModal()
	assertPanics(t, "invalid event", "not in the alphabet", func() {
		m.Process("close_angle") // close_angle is not in data mode's alphabet
	})
}

func TestModal_Process_SwitchAndProcess(t *testing.T) {
	m := makeHTMLModal()

	// Process in data mode
	m.Process("char")
	m.Process("open_angle")

	// Switch to tag mode
	m.SwitchMode("enter_tag")

	// Process in tag mode
	m.Process("char")
	m.Process("close_angle")

	// Switch back
	m.SwitchMode("exit_tag")

	// Process in data mode again (DFA was reset)
	state := m.Process("char")
	if state != "text" {
		t.Errorf("after switch back, Process('char') = %q, want text", state)
	}
}

// =========================================================================
// ActiveMachine tests
// =========================================================================

func TestModal_ActiveMachine(t *testing.T) {
	m := makeHTMLModal()

	dm := m.ActiveMachine()
	if dm.Initial() != "text" {
		t.Errorf("data mode DFA initial = %q, want text", dm.Initial())
	}

	m.SwitchMode("enter_tag")
	tm := m.ActiveMachine()
	if tm.Initial() != "name" {
		t.Errorf("tag mode DFA initial = %q, want name", tm.Initial())
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestModal_Reset(t *testing.T) {
	m := makeHTMLModal()

	m.Process("open_angle")
	m.SwitchMode("enter_tag")
	m.Process("char")

	m.Reset()

	if m.CurrentMode() != "data" {
		t.Errorf("after reset: mode = %q, want data", m.CurrentMode())
	}
	if len(m.ModeTrace()) != 0 {
		t.Errorf("after reset: trace length = %d, want 0", len(m.ModeTrace()))
	}
	// All DFAs should be reset
	if m.ActiveMachine().CurrentState() != "text" {
		t.Errorf("after reset: DFA state = %q, want text", m.ActiveMachine().CurrentState())
	}
}

func TestModal_Reset_MultiCycle(t *testing.T) {
	m := makeHTMLModal()

	for i := 0; i < 3; i++ {
		m.SwitchMode("enter_tag")
		m.Process("char")
		m.SwitchMode("exit_tag")
		m.Reset()

		if m.CurrentMode() != "data" {
			t.Errorf("cycle %d: mode after reset = %q, want data", i, m.CurrentMode())
		}
	}
}

// =========================================================================
// Modes list tests
// =========================================================================

func TestModal_Modes_Sorted(t *testing.T) {
	m := makeHTMLModal()
	modes := m.Modes()

	if len(modes) != 2 {
		t.Fatalf("Modes length = %d, want 2", len(modes))
	}
	// Should be sorted
	if modes[0] != "data" || modes[1] != "tag" {
		t.Errorf("Modes = %v, want [data, tag]", modes)
	}
}

func TestModal_Modes_ThreeMode(t *testing.T) {
	m := makeThreeMode()
	modes := m.Modes()

	if len(modes) != 3 {
		t.Fatalf("Modes length = %d, want 3", len(modes))
	}
	expected := []string{"insert", "normal", "visual"}
	for i, exp := range expected {
		if modes[i] != exp {
			t.Errorf("Modes[%d] = %q, want %q", i, modes[i], exp)
		}
	}
}

// =========================================================================
// ModeTrace copy test
// =========================================================================

func TestModal_ModeTrace_Copy(t *testing.T) {
	m := makeHTMLModal()
	m.SwitchMode("enter_tag")

	trace := m.ModeTrace()
	if len(trace) != 1 {
		t.Fatalf("trace length = %d, want 1", len(trace))
	}

	// Switching again should not affect the returned copy
	m.SwitchMode("exit_tag")
	if len(trace) != 1 {
		t.Error("ModeTrace() should return a copy")
	}
}

// =========================================================================
// Edge cases
// =========================================================================

func TestModal_SingleMode(t *testing.T) {
	dfa := makeDataModeDFA()
	m := NewModalStateMachine(
		map[string]*DFA{"only": dfa},
		map[[2]string]string{},
		"only",
	)

	if m.CurrentMode() != "only" {
		t.Errorf("single mode = %q, want only", m.CurrentMode())
	}

	// Process should work
	m.Process("char")
	if m.ActiveMachine().CurrentState() != "text" {
		t.Error("single mode processing failed")
	}

	// No mode transitions possible
	assertPanics(t, "no transitions", "no mode transition", func() {
		m.SwitchMode("anything")
	})
}

func TestModal_SelfTransition(t *testing.T) {
	dfa := makeDataModeDFA()
	m := NewModalStateMachine(
		map[string]*DFA{"data": dfa},
		map[[2]string]string{
			{"data", "reload"}: "data",
		},
		"data",
	)

	// Process to change DFA state
	m.Process("open_angle")
	if m.ActiveMachine().CurrentState() != "tag_start" {
		t.Fatal("DFA should be in tag_start")
	}

	// Self-transition should reset the DFA
	m.SwitchMode("reload")
	if m.CurrentMode() != "data" {
		t.Errorf("mode = %q, want data", m.CurrentMode())
	}
	if m.ActiveMachine().CurrentState() != "text" {
		t.Errorf("after self-transition, DFA state = %q, want text (reset)",
			m.ActiveMachine().CurrentState())
	}
}

func TestModeTransitionRecord_Fields(t *testing.T) {
	// Verify field access works correctly
	m := makeHTMLModal()
	m.SwitchMode("enter_tag")

	trace := m.ModeTrace()
	if len(trace) != 1 {
		t.Fatalf("trace length = %d, want 1", len(trace))
	}

	rec := trace[0]
	if rec.FromMode != "data" {
		t.Errorf("FromMode = %q, want data", rec.FromMode)
	}
	if rec.Trigger != "enter_tag" {
		t.Errorf("Trigger = %q, want enter_tag", rec.Trigger)
	}
	if rec.ToMode != "tag" {
		t.Errorf("ToMode = %q, want tag", rec.ToMode)
	}
}
