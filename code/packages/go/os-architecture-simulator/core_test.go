package ossimulator

import (
	"bytes"
	"encoding/json"
	"errors"
	"math"
	"testing"
)

func TestStepBudgetsAndSyntheticClock(t *testing.T) {
	machine, err := NewMachine(BatchMonitorProfile{})
	if err != nil {
		t.Fatal(err)
	}
	if events, err := machine.Run(0); err != nil || len(events) != 0 || machine.Tick() != 0 {
		t.Fatalf("zero run: events=%v tick=%d err=%v", events, machine.Tick(), err)
	}
	if _, err := machine.Run(-1); err == nil {
		t.Fatal("negative budget accepted")
	}
	if _, err := machine.Run(MaxRunBudget + 1); err == nil {
		t.Fatal("oversized budget accepted")
	}
	if events, err := machine.Step(); err != nil || len(events) != 0 || machine.Tick() != 1 {
		t.Fatalf("idle step: events=%v tick=%d err=%v", events, machine.Tick(), err)
	}
}

func TestProfileErrorsAndPublicReplayValidation(t *testing.T) {
	if _, err := NewMachine(nil); err == nil {
		t.Fatal("nil profile accepted")
	}
	machine, err := NewMachine(errorProfile{})
	if err != nil {
		t.Fatal(err)
	}
	before, _ := machine.Serialize()
	if _, err := machine.Run(1); err == nil {
		t.Fatal("profile error swallowed")
	}
	after, _ := machine.Serialize()
	if !bytes.Equal(before, after) {
		t.Fatal("profile error committed a transition")
	}
	if _, err := Replay(BatchMonitorProfile{}, []Stimulus{{Kind: "", Data: map[string]any{}}}); err == nil {
		t.Fatal("invalid replay input accepted")
	}
	if machine.ProfileID() != "profile-error-test" {
		t.Fatalf("profile ID = %q", machine.ProfileID())
	}
}

type errorProfile struct{}

func (errorProfile) ID() string        { return "profile-error-test" }
func (errorProfile) InitialState() any { return map[string]any{"count": 0} }
func (errorProfile) Transition(state any, _ uint64, _ Stimulus) (any, []Effect, error) {
	state.(map[string]any)["count"] = 1
	return state, nil, errors.New("intentional reducer error")
}

func TestNativeNumbersAndSnapshotTampering(t *testing.T) {
	profile := BatchMonitorProfile{}
	machine, _ := NewMachine(profile)
	if err := machine.Inject(Stimulus{Kind: "job.submit", Data: map[string]any{"id": "native", "serviceTicks": int16(1)}}); err != nil {
		t.Fatal(err)
	}
	if _, err := machine.Step(); err != nil {
		t.Fatal(err)
	}
	encoded, _ := machine.Serialize()
	var forged map[string]any
	if err := json.Unmarshal(encoded, &forged); err != nil {
		t.Fatal(err)
	}
	forged["tick"] = float64(99)
	tampered, _ := json.Marshal(forged)
	if _, err := Restore(profile, tampered); err == nil {
		t.Fatal("forged tick accepted")
	}
	if _, err := machine.Run(math.MaxInt); err == nil {
		t.Fatal("unbounded run accepted")
	}
}

func TestNestedJSONAndStructuredMessageBody(t *testing.T) {
	machine, err := NewMachine(ProtectedMessageSchedulerProfile{})
	if err != nil {
		t.Fatal(err)
	}
	body := map[string]any{"request": "lookup", "arguments": []any{"alpha", 4, nil}}
	for _, stimulus := range []Stimulus{
		{Kind: "task.create", Data: map[string]any{"id": "worker", "quantum": 1}},
		{Kind: "message.send", Data: map[string]any{"to": "worker", "capability": "cap:worker", "body": body}},
	} {
		if err := machine.Inject(stimulus); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := machine.Run(3); err != nil {
		t.Fatal(err)
	}
	events := machine.Events()
	if events[1].Kind != "ipc.enqueued" || events[3].Kind != "ipc.delivered" {
		t.Fatalf("unexpected event order: %#v", events)
	}
	assertJSONEqual(t, events[3].Data["body"], body)
	// Mutating the caller's original body cannot alter the trace after injection.
	body["request"] = "changed"
	if events[3].Data["body"].(map[string]any)["request"] != "lookup" {
		t.Fatal("trace aliased caller data")
	}
}

type failingProfile struct{}

func (failingProfile) ID() string        { return "failure-test" }
func (failingProfile) InitialState() any { return map[string]any{"count": 0} }
func (failingProfile) Transition(state any, _ uint64, _ Stimulus) (any, []Effect, error) {
	state.(map[string]any)["count"] = 9
	return state, []Effect{{Kind: "bad", Data: map[string]any{"value": math.NaN()}}}, nil
}

func TestInvalidReducerOutputCannotPartiallyCommit(t *testing.T) {
	machine, err := NewMachine(failingProfile{})
	if err != nil {
		t.Fatal(err)
	}
	before, err := machine.Serialize()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := machine.Step(); err == nil {
		t.Fatal("invalid effect accepted")
	}
	after, _ := machine.Serialize()
	if !bytes.Equal(before, after) || machine.Tick() != 0 {
		t.Fatalf("failed transition committed: %s", after)
	}
}

func TestSnapshotPendingStimulusAndProfileMismatch(t *testing.T) {
	profile := BatchMonitorProfile{}
	machine, _ := NewMachine(profile)
	if err := machine.Inject(Stimulus{Kind: "job.submit", Data: map[string]any{"id": "pending", "serviceTicks": "1"}}); err != nil {
		t.Fatal(err)
	}
	encoded, _ := machine.Serialize()
	restored, err := Restore(profile, encoded)
	if err != nil {
		t.Fatal(err)
	}
	_, _ = machine.Run(2)
	_, _ = restored.Run(2)
	left, _ := machine.Serialize()
	right, _ := restored.Serialize()
	if !bytes.Equal(left, right) {
		t.Fatalf("pending stimuli diverged: %s != %s", left, right)
	}
	if _, err := Restore(ProtectedMessageSchedulerProfile{}, encoded); err == nil {
		t.Fatal("profile mismatch accepted")
	}
}

func TestInvalidJSONInputsAreRejectedWithoutQueueMutation(t *testing.T) {
	machine, _ := NewMachine(BatchMonitorProfile{})
	before, _ := machine.Serialize()
	for _, value := range []any{math.NaN(), func() {}, map[string]any{"bad": math.Inf(1)}} {
		if err := machine.Inject(Stimulus{Kind: "invalid", Data: map[string]any{"payload": value}}); err == nil {
			t.Fatalf("accepted unsupported value %#v", value)
		}
	}
	after, _ := machine.Serialize()
	if !bytes.Equal(before, after) {
		t.Fatal("invalid inputs mutated machine")
	}
}

func TestProfileIntegerInputsMatchPortableNumberRange(t *testing.T) {
	for _, test := range []struct {
		profile Profile
		input   Stimulus
	}{
		{BatchMonitorProfile{}, Stimulus{Kind: "job.submit", Data: map[string]any{"id": "too-large", "serviceTicks": float64(1e100)}}},
		{ProtectedMessageSchedulerProfile{}, Stimulus{Kind: "task.create", Data: map[string]any{"id": "too-large", "quantum": float64(9_007_199_254_740_992)}}},
	} {
		machine, err := NewMachine(test.profile)
		if err != nil {
			t.Fatal(err)
		}
		if err := machine.Inject(test.input); err != nil {
			t.Fatal(err)
		}
		if _, err := machine.Step(); err != nil {
			t.Fatal(err)
		}
		events := machine.Events()
		if len(events) != 1 || events[0].Kind != "job.rejected" && events[0].Kind != "task.rejected" {
			t.Fatalf("unsafe integer input was not rejected: %#v", events)
		}
	}
}

func assertJSONEqual(t *testing.T, actual, expected any) {
	t.Helper()
	left, err := json.Marshal(actual)
	if err != nil {
		t.Fatal(err)
	}
	right, err := json.Marshal(expected)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(left, right) {
		t.Fatalf("JSON values differ:\nactual:   %s\nexpected: %s", left, right)
	}
}
