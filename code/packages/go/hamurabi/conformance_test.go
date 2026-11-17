package hamurabi

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

type fixtureTurn struct {
	Turn           int           `json:"turn"`
	Order          TurnOrder     `json:"order"`
	ExpectedState  HamurabiState `json:"expected_state"`
	ExpectedReport string        `json:"expected_report"`
}

type fixtureCase struct {
	ID            string        `json:"id"`
	Description   string        `json:"description"`
	Seed          uint64        `json:"seed"`
	InitialState  HamurabiState `json:"initial_state"`
	Order         *TurnOrder    `json:"order"`
	ExpectedError string        `json:"expected_error"`
	Turns         []fixtureTurn `json:"turns"`
}

type fixtureDocument struct {
	Version int           `json:"version"`
	Suite   string        `json:"suite"`
	Cases   []fixtureCase `json:"cases"`
}

const fixturePath = "../../../specs/fixtures/hamurabi-v1/cases.json"

func TestCorpusConformance(t *testing.T) {
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		// Fallback to absolute path or clean path if run from different dir
		cleaned := filepath.Clean(fixturePath)
		data, err = os.ReadFile(cleaned)
		if err != nil {
			t.Fatalf("failed to read fixture file %s: %v", fixturePath, err)
		}
	}

	var doc fixtureDocument
	if err := json.Unmarshal(data, &doc); err != nil {
		t.Fatalf("failed to unmarshal fixture JSON: %v", err)
	}

	if len(doc.Cases) < 31 {
		t.Fatalf("fixture suite shrunk: got %d cases, want at least 31", len(doc.Cases))
	}

	for _, c := range doc.Cases {
		c := c
		t.Run(c.ID, func(t *testing.T) {
			if c.ExpectedError != "" {
				// Refusal case
				if c.Order == nil {
					t.Fatalf("refusal case %s has expected_error %q but nil order", c.ID, c.ExpectedError)
				}
				err := ValidateOrder(c.InitialState, *c.Order)
				if err == nil {
					t.Fatalf("ValidateOrder succeeded; want error %q", c.ExpectedError)
				}
				if err.Error() != c.ExpectedError {
					t.Fatalf("ValidateOrder error = %q; want %q", err.Error(), c.ExpectedError)
				}

				// StepYear must refuse with the exact same error without advancing state
				_, stepErr := StepYear(c.InitialState, *c.Order)
				if stepErr == nil {
					t.Fatalf("StepYear succeeded; want error %q", c.ExpectedError)
				}
				if stepErr.Error() != c.ExpectedError {
					t.Fatalf("StepYear error = %q; want %q", stepErr.Error(), c.ExpectedError)
				}
			} else {
				// Trajectory case
				state := c.InitialState
				for _, turn := range c.Turns {
					next, err := StepYear(state, turn.Order)
					if err != nil {
						t.Fatalf("turn %d StepYear error: %v", turn.Turn, err)
					}
					if next != turn.ExpectedState {
						t.Fatalf("turn %d state mismatch:\n  got:  %+v\n  want: %+v", turn.Turn, next, turn.ExpectedState)
					}
					if turn.ExpectedReport != "" {
						report := FormatReport(next)
						if report != turn.ExpectedReport {
							t.Fatalf("turn %d report mismatch:\n  got:  %q\n  want: %q", turn.Turn, report, turn.ExpectedReport)
						}
					}
					state = next
				}
			}
		})
	}
}
