package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

var testTriggerFixtureRoster = []string{
	"test-trigger-selection-dev-cycle-outside-graph.json",
	"test-trigger-selection-empty.json",
	"test-trigger-selection-graph-cycle.json",
	"test-trigger-selection-one-hop.json",
	"test-trigger-selection-prerequisites-after-hop.json",
	"test-trigger-selection-unknown-relation.json",
}

type testTriggerFixture struct {
	ID    string `json:"id"`
	Input struct {
		Options struct {
			Packages         []string    `json:"packages"`
			Edges            [][2]string `json:"edges"`
			TriggerRelations [][2]string `json:"trigger_relations"`
			ChangedPackages  []string    `json:"changed_packages"`
		} `json:"options"`
	} `json:"input"`
	Expected struct {
		Outcome string `json:"outcome"`
		Result  struct {
			ChangedPackages      []string `json:"changed_packages"`
			AffectedPackages     []string `json:"affected_packages"`
			PrerequisitePackages []string `json:"prerequisite_packages"`
		} `json:"result"`
		Diagnostics []struct {
			Code string `json:"code"`
		} `json:"diagnostics"`
	} `json:"expected"`
}

func loadTestTriggerFixture(t *testing.T, name string) testTriggerFixture {
	t.Helper()
	path := filepath.Join(graphDiffFixtureRoot(t), "cases", name)
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var fixture testTriggerFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode %s: %v", name, err)
	}
	return fixture
}

func testTriggerSortedKeys(values map[string]bool) []string {
	result := make([]string, 0, len(values))
	for value := range values {
		result = append(result, value)
	}
	sort.Strings(result)
	return result
}

func TestNeutralTestTriggerSelectionContractMatchesEstablishedGoHelpers(t *testing.T) {
	if names := enumerateGraphDiffFixtures(t, "test-trigger-selection-"); !reflect.DeepEqual(names, testTriggerFixtureRoster) {
		t.Fatalf("test-trigger fixture roster = %v, want %v", names, testTriggerFixtureRoster)
	}

	for _, name := range testTriggerFixtureRoster {
		fixture := loadTestTriggerFixture(t, name)
		t.Run(fixture.ID, func(t *testing.T) {
			known := make(map[string]bool, len(fixture.Input.Options.Packages))
			graph := directedgraph.New()
			for _, packageName := range fixture.Input.Options.Packages {
				known[packageName] = true
				graph.AddNode(packageName)
			}
			for _, edge := range fixture.Input.Options.Edges {
				graph.AddEdge(edge[0], edge[1])
			}

			if _, err := graph.IndependentGroups(); err != nil {
				if fixture.Expected.Outcome != "error" || len(fixture.Expected.Diagnostics) != 1 || fixture.Expected.Diagnostics[0].Code != "TEST_TRIGGER_GRAPH_CYCLE" {
					t.Fatalf("unexpected graph-cycle contract: outcome=%s diagnostics=%v", fixture.Expected.Outcome, fixture.Expected.Diagnostics)
				}
				return
			}

			triggers := make(resolver.TestTriggers)
			for _, relation := range fixture.Input.Options.TriggerRelations {
				if !known[relation[0]] || !known[relation[1]] {
					if fixture.Expected.Outcome != "error" || len(fixture.Expected.Diagnostics) != 1 || fixture.Expected.Diagnostics[0].Code != "TEST_TRIGGER_RELATION_UNKNOWN" {
						t.Fatalf("unexpected unknown-relation contract: outcome=%s diagnostics=%v", fixture.Expected.Outcome, fixture.Expected.Diagnostics)
					}
					return
				}
				triggers[relation[0]] = append(triggers[relation[0]], relation[1])
			}

			if fixture.Expected.Outcome != "ok" {
				t.Fatalf("fixture expected %s but its inert inputs passed the shared preconditions", fixture.Expected.Outcome)
			}
			changed := make(map[string]bool, len(fixture.Input.Options.ChangedPackages))
			for _, packageName := range fixture.Input.Options.ChangedPackages {
				changed[packageName] = true
			}
			buildClosure := graph.AffectedNodes(changed)
			affected := withDevDependents(buildClosure, triggers)
			total := expandAffectedSetWithPrereqs(graph, affected)
			prerequisites := make(map[string]bool)
			for packageName := range total {
				if !affected[packageName] {
					prerequisites[packageName] = true
				}
			}

			if got := testTriggerSortedKeys(changed); !reflect.DeepEqual(got, fixture.Expected.Result.ChangedPackages) {
				t.Fatalf("changed packages = %v, want %v", got, fixture.Expected.Result.ChangedPackages)
			}
			if got := testTriggerSortedKeys(affected); !reflect.DeepEqual(got, fixture.Expected.Result.AffectedPackages) {
				t.Fatalf("affected packages = %v, want %v", got, fixture.Expected.Result.AffectedPackages)
			}
			if got := testTriggerSortedKeys(prerequisites); !reflect.DeepEqual(got, fixture.Expected.Result.PrerequisitePackages) {
				t.Fatalf("prerequisite packages = %v, want %v", got, fixture.Expected.Result.PrerequisitePackages)
			}
		})
	}
}
