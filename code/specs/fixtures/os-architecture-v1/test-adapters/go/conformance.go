// Package main verifies the public Go profile machine against its shared fixture.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	ossimulator "github.com/adhithyan15/coding-adventures/code/packages/go/os-architecture-simulator"
)

type portableFixture struct {
	SchemaVersion int           `json:"schemaVersion"`
	Cases         []fixtureCase `json:"cases"`
}

type fixtureCase struct {
	Profile         string                   `json:"profile"`
	Stimuli         []ossimulator.Stimulus   `json:"stimuli"`
	Budgets         []int                    `json:"budgets"`
	ExpectedTick    uint64                   `json:"expectedTick"`
	ExpectedState   any                      `json:"expectedState"`
	ExpectedEffects []ossimulator.TraceEvent `json:"expectedEffects"`
}

func main() {
	if err := verifyFixture(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func verifyFixture() error {
	_, sourcePath, _, ok := runtime.Caller(0)
	if !ok {
		return fmt.Errorf("locate Go fixture verifier source")
	}
	fixturePath := filepath.Join(filepath.Dir(sourcePath), "..", "..", "profiles.json")
	encoded, err := os.ReadFile(fixturePath)
	if err != nil {
		return fmt.Errorf("read shared profile fixture: %w", err)
	}
	var fixture portableFixture
	if err := json.Unmarshal(encoded, &fixture); err != nil {
		return fmt.Errorf("decode shared profile fixture: %w", err)
	}
	if fixture.SchemaVersion != ossimulator.SchemaVersion {
		return fmt.Errorf("fixture schema = %d, want %d", fixture.SchemaVersion, ossimulator.SchemaVersion)
	}
	profiles := map[string]ossimulator.Profile{
		ossimulator.BatchMonitorID:       ossimulator.BatchMonitorProfile{},
		ossimulator.ProtectedSchedulerID: ossimulator.ProtectedMessageSchedulerProfile{},
	}
	if len(fixture.Cases) != len(profiles) {
		return fmt.Errorf("fixture has %d cases, want %d profile cases", len(fixture.Cases), len(profiles))
	}
	seen := make(map[string]bool, len(fixture.Cases))
	for _, scenario := range fixture.Cases {
		profile, exists := profiles[scenario.Profile]
		if !exists || seen[scenario.Profile] {
			return fmt.Errorf("fixture profile %q is unknown or duplicated", scenario.Profile)
		}
		seen[scenario.Profile] = true
		if err := verifyScenario(profile, scenario); err != nil {
			return fmt.Errorf("profile %s: %w", scenario.Profile, err)
		}
	}
	for id := range profiles {
		if !seen[id] {
			return fmt.Errorf("fixture is missing profile %q", id)
		}
	}
	return nil
}

func verifyScenario(profile ossimulator.Profile, scenario fixtureCase) error {
	machine, err := ossimulator.NewMachine(profile)
	if err != nil {
		return err
	}
	for _, stimulus := range scenario.Stimuli {
		if err := machine.Inject(stimulus); err != nil {
			return fmt.Errorf("inject %q: %w", stimulus.Kind, err)
		}
	}
	for _, budget := range scenario.Budgets {
		if _, err := machine.Run(budget); err != nil {
			return fmt.Errorf("run budget %d: %w", budget, err)
		}
	}
	if machine.Tick() != scenario.ExpectedTick {
		return fmt.Errorf("tick = %d, want %d", machine.Tick(), scenario.ExpectedTick)
	}
	if err := assertJSONEqual(machine.State(), scenario.ExpectedState); err != nil {
		return fmt.Errorf("final state: %w", err)
	}
	if err := assertJSONEqual(machine.Events(), scenario.ExpectedEffects); err != nil {
		return fmt.Errorf("ordered effects: %w", err)
	}

	encoded, err := machine.Serialize()
	if err != nil {
		return fmt.Errorf("serialize snapshot: %w", err)
	}
	restored, err := ossimulator.Restore(profile, encoded)
	if err != nil {
		return fmt.Errorf("restore snapshot: %w", err)
	}
	replayed, err := ossimulator.Replay(profile, machine.Snapshot().History)
	if err != nil {
		return fmt.Errorf("replay history: %w", err)
	}
	for _, continuation := range []*ossimulator.Machine{restored, replayed} {
		if _, err := continuation.Run(3); err != nil {
			return fmt.Errorf("run continuation: %w", err)
		}
	}
	restoredJSON, err := restored.Serialize()
	if err != nil {
		return fmt.Errorf("serialize restored continuation: %w", err)
	}
	replayedJSON, err := replayed.Serialize()
	if err != nil {
		return fmt.Errorf("serialize replayed continuation: %w", err)
	}
	if !bytes.Equal(restoredJSON, replayedJSON) {
		return fmt.Errorf("restored/replayed continuation differs:\n%s\n%s", restoredJSON, replayedJSON)
	}
	return nil
}

func assertJSONEqual(actual, expected any) error {
	left, err := json.Marshal(actual)
	if err != nil {
		return fmt.Errorf("marshal actual: %w", err)
	}
	right, err := json.Marshal(expected)
	if err != nil {
		return fmt.Errorf("marshal expected: %w", err)
	}
	if !bytes.Equal(left, right) {
		return fmt.Errorf("JSON values differ:\nactual:   %s\nexpected: %s", left, right)
	}
	return nil
}
