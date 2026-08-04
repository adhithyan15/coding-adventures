package capabilitycage

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

type taxonomyFixture struct {
	Categories                    map[string][]string `json:"categories"`
	AllActions                    []string            `json:"all_actions"`
	ExpectedValidPairCount        int                 `json:"expected_valid_pair_count"`
	ExpectedInvalidCrossPairCount int                 `json:"expected_invalid_cross_pair_count"`
}

func loadTaxonomyFixture(t *testing.T) taxonomyFixture {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "capability-security-v1", "taxonomy.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read shared taxonomy fixture: %v", err)
	}
	var fixture taxonomyFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("parse shared taxonomy fixture: %v", err)
	}
	return fixture
}

func containsAction(actions []string, wanted string) bool {
	for _, action := range actions {
		if action == wanted {
			return true
		}
	}
	return false
}

func TestCapabilityTaxonomyMatchesSharedFixture(t *testing.T) {
	fixture := loadTaxonomyFixture(t)
	validCount := 0
	invalidCount := 0
	for category, allowed := range fixture.Categories {
		for _, action := range fixture.AllActions {
			want := containsAction(allowed, action)
			if got := validCapabilityPair(category, action); got != want {
				t.Errorf("validCapabilityPair(%q, %q) = %v, want %v", category, action, got, want)
			}
			if want {
				validCount++
			} else {
				invalidCount++
			}
		}
	}
	if validCount != fixture.ExpectedValidPairCount {
		t.Errorf("valid pair count = %d, want %d", validCount, fixture.ExpectedValidPairCount)
	}
	if invalidCount != fixture.ExpectedInvalidCrossPairCount {
		t.Errorf("invalid cross-pair count = %d, want %d", invalidCount, fixture.ExpectedInvalidCrossPairCount)
	}
	if validCapabilityPair("filesystem", "read") {
		t.Error("unknown category must fail closed")
	}
	if validCapabilityPair("fs", "destroy") {
		t.Error("unknown action must fail closed")
	}
}

func TestNewManifestRejectsInvalidPairBeforeObservation(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected invalid manifest construction to panic")
		}
	}()
	_ = NewManifest([]Capability{{
		Category:      CategoryFS,
		Action:        ActionConnect,
		Target:        "*",
		Justification: "Invalid cross-pair fixture.",
	}})
}
