package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/graphdiff"
)

var graphFixtureRoster = []string{
	"graph-canonical-edge-order.json", "graph-chain.json", "graph-cycle.json",
	"graph-diamond.json", "graph-empty.json", "graph-isolated.json",
	"graph-multiple-components.json", "graph-partial-cycle-no-output.json",
}

var diffFixtureRoster = []string{
	"diff-selection-exact-build-fronts.json", "diff-selection-forced-package.json",
	"diff-selection-known-unmatched-near-build.json", "diff-selection-match-work-at-limit.json",
	"diff-selection-match-work-over-limit.json", "diff-selection-package-prefix.json",
	"diff-selection-repository-boundary.json", "diff-selection-shared-input-multiconsumer.json",
	"diff-selection-strict-glob-character-classes.json",
	"diff-selection-transitive.json", "diff-selection-unknown-all.json",
	"diff-selection-unknown-error.json",
}

var graphFixtureIDs = []string{
	"graph/canonical-edge-order", "graph/chain", "graph/cycle", "graph/diamond",
	"graph/empty", "graph/isolated", "graph/multiple-components",
	"graph/partial-cycle-no-output",
}

var diffFixtureIDs = []string{
	"diff-selection/exact-build-fronts", "diff-selection/forced-package",
	"diff-selection/known-unmatched-near-build", "diff-selection/match-work-at-limit",
	"diff-selection/match-work-over-limit", "diff-selection/package-prefix",
	"diff-selection/repository-boundary-reverse-index",
	"diff-selection/shared-input-multiconsumer",
	"diff-selection/strict-glob-character-classes",
	"diff-selection/transitive-package-change", "diff-selection/unknown-path-all",
	"diff-selection/unknown-path-error",
}

type graphDiffFixture struct {
	ID    string `json:"id"`
	Input struct {
		Operation string `json:"operation"`
		Options   struct {
			Packages          []json.RawMessage `json:"packages"`
			Edges             [][2]string       `json:"edges"`
			ForcedPackages    []string          `json:"forced_packages"`
			UnknownPathPolicy string            `json:"unknown_path_policy"`
			BoundarySHA256    string            `json:"boundary_sha256"`
		} `json:"options"`
		ChangedPaths []string `json:"changed_paths"`
	} `json:"input"`
	Expected struct {
		Outcome string `json:"outcome"`
		Result  struct {
			Edges                [][2]string `json:"edges"`
			Levels               [][]string  `json:"levels"`
			ChangedPackages      []string    `json:"changed_packages"`
			AffectedPackages     []string    `json:"affected_packages"`
			PrerequisitePackages []string    `json:"prerequisite_packages"`
		} `json:"result"`
		Diagnostics []struct {
			Code string `json:"code"`
		} `json:"diagnostics"`
	} `json:"expected"`
}

func graphDiffFixtureRoot(t *testing.T) string {
	t.Helper()
	return filepath.Join(toolchainFixtureRepoRoot(t), "code", "specs", "fixtures", "build-tool-v1")
}

func enumerateGraphDiffFixtures(t *testing.T, prefix string) []string {
	t.Helper()
	entries, err := os.ReadDir(filepath.Join(graphDiffFixtureRoot(t), "cases"))
	if err != nil {
		t.Fatal(err)
	}
	names := make([]string, 0)
	for _, entry := range entries {
		if strings.HasPrefix(entry.Name(), prefix) && strings.HasSuffix(entry.Name(), ".json") {
			info, err := entry.Info()
			if err != nil {
				t.Fatal(err)
			}
			if !info.Mode().IsRegular() || info.Size() > 1<<20 {
				t.Fatalf("fixture %s must be a regular file no larger than 1 MiB", entry.Name())
			}
			names = append(names, entry.Name())
		}
	}
	sort.Strings(names)
	return names
}

func loadGraphDiffFixture(t *testing.T, name string) graphDiffFixture {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(graphDiffFixtureRoot(t), "cases", name))
	if err != nil {
		t.Fatal(err)
	}
	var fixture graphDiffFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode %s: %v", name, err)
	}
	return fixture
}

func graphDiffEdges(values [][2]string) []graphdiff.Edge {
	result := make([]graphdiff.Edge, len(values))
	for index, value := range values {
		result[index] = graphdiff.Edge{Prerequisite: value[0], Dependent: value[1]}
	}
	return result
}

func TestNeutralGraphDiffContractCoverage(t *testing.T) {
	if names := enumerateGraphDiffFixtures(t, "graph-"); !reflect.DeepEqual(names, graphFixtureRoster) {
		t.Fatalf("graph fixture roster = %v, want %v", names, graphFixtureRoster)
	}
	if names := enumerateGraphDiffFixtures(t, "diff-selection-"); !reflect.DeepEqual(names, diffFixtureRoster) {
		t.Fatalf("diff fixture roster = %v, want %v", names, diffFixtureRoster)
	}

	graphIDs := make([]string, 0, len(graphFixtureRoster))
	for _, name := range graphFixtureRoster {
		fixture := loadGraphDiffFixture(t, name)
		graphIDs = append(graphIDs, fixture.ID)
		packages := make([]string, len(fixture.Input.Options.Packages))
		for index, raw := range fixture.Input.Options.Packages {
			if err := json.Unmarshal(raw, &packages[index]); err != nil {
				t.Fatal(err)
			}
		}
		actual, err := graphdiff.EvaluateGraph(graphdiff.GraphInput{
			Packages: packages, Edges: graphDiffEdges(fixture.Input.Options.Edges),
		})
		if err != nil {
			t.Fatalf("%s: %v", fixture.ID, err)
		}
		if fixture.Expected.Outcome == "error" {
			if actual.ErrorCode != fixture.Expected.Diagnostics[0].Code || len(actual.Edges) != 0 || len(actual.Levels) != 0 {
				t.Fatalf("%s: error result = %#v", fixture.ID, actual)
			}
			continue
		}
		if actual.ErrorCode != "" || !reflect.DeepEqual(actual.Edges, graphDiffEdges(fixture.Expected.Result.Edges)) || !reflect.DeepEqual(actual.Levels, fixture.Expected.Result.Levels) {
			t.Fatalf("%s: result = %#v, want %#v", fixture.ID, actual, fixture.Expected.Result)
		}
	}
	if !reflect.DeepEqual(graphIDs, graphFixtureIDs) {
		t.Fatalf("graph fixture ids = %v, want %v", graphIDs, graphFixtureIDs)
	}

	diffIDs := make([]string, 0, len(diffFixtureRoster))
	for _, name := range diffFixtureRoster {
		fixture := loadGraphDiffFixture(t, name)
		diffIDs = append(diffIDs, fixture.ID)
		packages := make([]graphdiff.PackageSpec, len(fixture.Input.Options.Packages))
		for index, raw := range fixture.Input.Options.Packages {
			if err := json.Unmarshal(raw, &packages[index]); err != nil {
				t.Fatal(err)
			}
		}
		input := graphdiff.DiffSelectionInput{
			Packages: packages, Edges: graphDiffEdges(fixture.Input.Options.Edges),
			ForcedPackages:    fixture.Input.Options.ForcedPackages,
			UnknownPathPolicy: fixture.Input.Options.UnknownPathPolicy,
			ChangedPaths:      fixture.Input.ChangedPaths,
			BoundarySHA256:    fixture.Input.Options.BoundarySHA256,
		}
		if input.BoundarySHA256 != "" {
			data, err := os.ReadFile(filepath.Join(graphDiffFixtureRoot(t), "repository-source-input-boundary.json"))
			if err != nil {
				t.Fatal(err)
			}
			var boundary graphdiff.RepositoryBoundary
			if err := json.Unmarshal(data, &boundary); err != nil {
				t.Fatal(err)
			}
			input.Boundary = &boundary
		}
		actual, err := graphdiff.EvaluateDiffSelection(input)
		if err != nil {
			t.Fatalf("%s: %v", fixture.ID, err)
		}
		if fixture.Expected.Outcome == "error" {
			if actual.ErrorCode != fixture.Expected.Diagnostics[0].Code || len(actual.ChangedPackages) != 0 || len(actual.AffectedPackages) != 0 || len(actual.PrerequisitePackages) != 0 {
				t.Fatalf("%s: error result = %#v", fixture.ID, actual)
			}
			continue
		}
		if actual.ErrorCode != "" || !reflect.DeepEqual(actual.ChangedPackages, fixture.Expected.Result.ChangedPackages) || !reflect.DeepEqual(actual.AffectedPackages, fixture.Expected.Result.AffectedPackages) || !reflect.DeepEqual(actual.PrerequisitePackages, fixture.Expected.Result.PrerequisitePackages) {
			t.Fatalf("%s: result = %#v, want %#v", fixture.ID, actual, fixture.Expected.Result)
		}
	}
	if !reflect.DeepEqual(diffIDs, diffFixtureIDs) {
		t.Fatalf("diff fixture ids = %v, want %v", diffIDs, diffFixtureIDs)
	}
}
