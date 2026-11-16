package lunarlander

import (
	"encoding/json"
	"fmt"
	"os"
	"testing"
)

const corpusPath = "../../../specs/fixtures/lunar-lander-v1/cases.json"

type corpusFile struct {
	Version int          `json:"version"`
	Suite   string       `json:"suite"`
	Cases   []corpusCase `json:"cases"`
}

type corpusCase struct {
	ID            string       `json:"id"`
	Description   string       `json:"description"`
	InitialState  LanderState  `json:"initial_state"`
	Turns         []corpusTurn `json:"turns,omitempty"`
	Command       *BurnCommand `json:"command,omitempty"`
	ExpectedError string       `json:"expected_error,omitempty"`
}

type corpusTurn struct {
	Turn          uint32      `json:"turn"`
	Command       BurnCommand `json:"command"`
	ExpectedState LanderState `json:"expected_state"`
}

func parseExpectedError(s string) (LanderError, error) {
	switch s {
	case "FlightAlreadyEnded":
		return ErrFlightAlreadyEnded, nil
	case "ExcessiveBurnRate":
		return ErrExcessiveBurnRate, nil
	case "InvalidDuration":
		return ErrInvalidDuration, nil
	case "ArithmeticOverflow":
		return ErrArithmeticOverflow, nil
	default:
		return "", fmt.Errorf("unknown expected error in fixture: %s", s)
	}
}

func TestAll39CorpusCases(t *testing.T) {
	data, err := os.ReadFile(corpusPath)
	if err != nil {
		t.Fatalf("failed reading corpus at %s: %v", corpusPath, err)
	}

	var corpus corpusFile
	if err := json.Unmarshal(data, &corpus); err != nil {
		t.Fatalf("failed unmarshaling corpus: %v", err)
	}

	if corpus.Version != 1 {
		t.Errorf("expected corpus version 1, got %d", corpus.Version)
	}
	if corpus.Suite != "lunar_lander_deterministic_flight" {
		t.Errorf("expected suite 'lunar_lander_deterministic_flight', got %s", corpus.Suite)
	}
	if len(corpus.Cases) != 39 {
		t.Fatalf("expected exactly 39 cases in corpus, got %d", len(corpus.Cases))
	}

	for _, tc := range corpus.Cases {
		tc := tc
		t.Run(tc.ID, func(t *testing.T) {
			state := tc.InitialState

			if len(tc.Turns) > 0 {
				for _, turn := range tc.Turns {
					if err := ValidateBurn(state, turn.Command); err != nil {
						t.Fatalf("Case %s Turn %d ValidateBurn failed: %v", tc.ID, turn.Turn, err)
					}

					next, err := StepBurn(state, turn.Command)
					if err != nil {
						t.Fatalf("Case %s Turn %d StepBurn failed: %v", tc.ID, turn.Turn, err)
					}

					exp := turn.ExpectedState
					if next.TimeSeconds != exp.TimeSeconds {
						t.Errorf("Case %s Turn %d TimeSeconds mismatch: got %d, want %d",
							tc.ID, turn.Turn, next.TimeSeconds, exp.TimeSeconds)
					}
					if next.AltitudeMft != exp.AltitudeMft {
						t.Errorf("Case %s Turn %d AltitudeMft mismatch: got %d, want %d",
							tc.ID, turn.Turn, next.AltitudeMft, exp.AltitudeMft)
					}
					if next.VelocityMftS != exp.VelocityMftS {
						t.Errorf("Case %s Turn %d VelocityMftS mismatch: got %d, want %d",
							tc.ID, turn.Turn, next.VelocityMftS, exp.VelocityMftS)
					}
					if next.FuelLbs != exp.FuelLbs {
						t.Errorf("Case %s Turn %d FuelLbs mismatch: got %d, want %d",
							tc.ID, turn.Turn, next.FuelLbs, exp.FuelLbs)
					}
					if next.DryMassLbs != exp.DryMassLbs {
						t.Errorf("Case %s Turn %d DryMassLbs mismatch: got %d, want %d",
							tc.ID, turn.Turn, next.DryMassLbs, exp.DryMassLbs)
					}
					if next.Status != exp.Status {
						t.Errorf("Case %s Turn %d Status mismatch: got %v, want %v",
							tc.ID, turn.Turn, next.Status, exp.Status)
					}
					if next.HasImpactVelocity != exp.HasImpactVelocity {
						t.Errorf("Case %s Turn %d HasImpactVelocity mismatch: got %v, want %v",
							tc.ID, turn.Turn, next.HasImpactVelocity, exp.HasImpactVelocity)
					}
					if next.ImpactVelocityMftS != exp.ImpactVelocityMftS {
						t.Errorf("Case %s Turn %d ImpactVelocityMftS mismatch: got %d, want %d",
							tc.ID, turn.Turn, next.ImpactVelocityMftS, exp.ImpactVelocityMftS)
					}

					state = next
				}
			} else if tc.Command != nil {
				expectedErr, err := parseExpectedError(tc.ExpectedError)
				if err != nil {
					t.Fatalf("Case %s: %v", tc.ID, err)
				}

				valErr := ValidateBurn(state, *tc.Command)
				if valErr != expectedErr {
					t.Errorf("Case %s ValidateBurn error mismatch: got %v, want %v",
						tc.ID, valErr, expectedErr)
				}

				_, stepErr := StepBurn(state, *tc.Command)
				if stepErr != expectedErr {
					t.Errorf("Case %s StepBurn error mismatch: got %v, want %v",
						tc.ID, stepErr, expectedErr)
				}
			} else {
				t.Fatalf("Case %s has neither turns nor command", tc.ID)
			}
		})
	}
}
