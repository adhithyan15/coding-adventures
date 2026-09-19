package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
)

type neutralGraphDiffFixture struct {
	Input struct {
		Options struct {
			Packages       []json.RawMessage `json:"packages"`
			Edges          [][2]string       `json:"edges"`
			ForcedPackages []string          `json:"forced_packages"`
		} `json:"options"`
		ChangedPaths []string `json:"changed_paths"`
	} `json:"input"`
	Expected struct {
		Result struct {
			Edges                [][2]string `json:"edges"`
			Levels               [][]string  `json:"levels"`
			ChangedPackages      []string    `json:"changed_packages"`
			AffectedPackages     []string    `json:"affected_packages"`
			PrerequisitePackages []string    `json:"prerequisite_packages"`
		} `json:"result"`
	} `json:"expected"`
}

func loadNeutralGraphDiffFixture(t *testing.T, name string) neutralGraphDiffFixture {
	t.Helper()
	path := filepath.Join(
		toolchainFixtureRepoRoot(t), "code", "specs", "fixtures", "build-tool-v1", "cases", name,
	)
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var fixture neutralGraphDiffFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode %s: %v", name, err)
	}
	return fixture
}

func graphFromNeutralFixture(fixture neutralGraphDiffFixture) *directedgraph.Graph {
	graph := directedgraph.New()
	for _, raw := range fixture.Input.Options.Packages {
		var name string
		if err := json.Unmarshal(raw, &name); err == nil {
			graph.AddNode(name)
		}
	}
	for _, edge := range fixture.Input.Options.Edges {
		graph.AddEdge(edge[0], edge[1])
	}
	return graph
}

func sortedSetValues(values map[string]bool) []string {
	result := make([]string, 0, len(values))
	for value := range values {
		result = append(result, value)
	}
	slices.Sort(result)
	return result
}

func packagesFromNeutralFixture(
	t *testing.T,
	fixture neutralGraphDiffFixture,
	repoRoot string,
) []discovery.Package {
	t.Helper()
	packages := make([]discovery.Package, 0, len(fixture.Input.Options.Packages))
	for _, raw := range fixture.Input.Options.Packages {
		var pkg struct {
			Name        string   `json:"name"`
			RelPath     string   `json:"rel_path"`
			SourceMode  string   `json:"source_mode"`
			SourceGlobs []string `json:"source_globs"`
		}
		if err := json.Unmarshal(raw, &pkg); err != nil {
			t.Fatal(err)
		}
		packages = append(packages, discovery.Package{
			Name:         pkg.Name,
			Path:         filepath.Join(repoRoot, filepath.FromSlash(pkg.RelPath)),
			IsStarlark:   pkg.SourceMode == "strict_globs",
			DeclaredSrcs: pkg.SourceGlobs,
		})
	}
	return packages
}

func TestNeutralGraphDiffContractCoverage(t *testing.T) {
	t.Run("empty graph", func(t *testing.T) {
		fixture := loadNeutralGraphDiffFixture(t, "graph-empty.json")
		graph := graphFromNeutralFixture(fixture)
		if edges := graph.Edges(); len(edges) != 0 {
			t.Fatalf("edges = %v, want empty", edges)
		}
		levels, err := graph.IndependentGroups()
		if err != nil {
			t.Fatal(err)
		}
		if len(levels) != 0 {
			t.Fatalf("levels = %v, want empty", levels)
		}
	})

	t.Run("partial cycle has no output", func(t *testing.T) {
		fixture := loadNeutralGraphDiffFixture(t, "graph-partial-cycle-no-output.json")
		graph := graphFromNeutralFixture(fixture)
		levels, err := graph.IndependentGroups()
		if err == nil {
			t.Fatal("expected cycle error")
		}
		if len(levels) != 0 {
			t.Fatalf("levels = %v, want empty", levels)
		}
	})

	t.Run("canonical graph edge order", func(t *testing.T) {
		fixture := loadNeutralGraphDiffFixture(t, "graph-canonical-edge-order.json")
		graph := graphFromNeutralFixture(fixture)
		if got := graph.Edges(); !reflect.DeepEqual(got, fixture.Expected.Result.Edges) {
			t.Fatalf("edges = %v, want %v", got, fixture.Expected.Result.Edges)
		}
		levels, err := graph.IndependentGroups()
		if err != nil {
			t.Fatal(err)
		}
		if !reflect.DeepEqual(levels, fixture.Expected.Result.Levels) {
			t.Fatalf("levels = %v, want %v", levels, fixture.Expected.Result.Levels)
		}
	})

	t.Run("package prefix", func(t *testing.T) {
		fixture := loadNeutralGraphDiffFixture(t, "diff-selection-package-prefix.json")
		repoRoot := filepath.Join(t.TempDir(), "repo")
		packages := make([]discovery.Package, 0, len(fixture.Input.Options.Packages))
		for _, raw := range fixture.Input.Options.Packages {
			var pkg struct {
				Name    string `json:"name"`
				RelPath string `json:"rel_path"`
			}
			if err := json.Unmarshal(raw, &pkg); err != nil {
				t.Fatal(err)
			}
			packages = append(packages, discovery.Package{
				Name: pkg.Name,
				Path: filepath.Join(repoRoot, filepath.FromSlash(pkg.RelPath)),
			})
		}
		changed := gitdiff.MapFilesToPackages(fixture.Input.ChangedPaths, packages, repoRoot)
		if got := sortedSetValues(changed); !slices.Equal(got, fixture.Expected.Result.ChangedPackages) {
			t.Fatalf("changed packages = %v, want %v", got, fixture.Expected.Result.ChangedPackages)
		}
	})

	for _, name := range []string{
		"diff-selection-exact-build-fronts.json",
		"diff-selection-known-unmatched-near-build.json",
		"diff-selection-strict-glob-character-classes.json",
	} {
		name := name
		t.Run(name, func(t *testing.T) {
			fixture := loadNeutralGraphDiffFixture(t, name)
			repoRoot := filepath.Join(t.TempDir(), "repo")
			packages := packagesFromNeutralFixture(t, fixture, repoRoot)
			changed := gitdiff.MapFilesToPackages(fixture.Input.ChangedPaths, packages, repoRoot)
			if got := sortedSetValues(changed); !slices.Equal(got, fixture.Expected.Result.ChangedPackages) {
				t.Fatalf("changed packages = %v, want %v", got, fixture.Expected.Result.ChangedPackages)
			}
		})
	}

	t.Run("forced package closure", func(t *testing.T) {
		fixture := loadNeutralGraphDiffFixture(t, "diff-selection-forced-package.json")
		graph := graphFromNeutralFixture(fixture)
		changed := make(map[string]bool)
		for _, name := range fixture.Input.Options.ForcedPackages {
			changed[name] = true
		}
		affected := graph.AffectedNodes(changed)
		closed := expandAffectedSetWithPrereqs(graph, affected)
		prerequisites := make(map[string]bool)
		for name := range closed {
			if !affected[name] {
				prerequisites[name] = true
			}
		}
		if got := sortedSetValues(changed); !slices.Equal(got, fixture.Expected.Result.ChangedPackages) {
			t.Fatalf("changed packages = %v, want %v", got, fixture.Expected.Result.ChangedPackages)
		}
		if got := sortedSetValues(affected); !slices.Equal(got, fixture.Expected.Result.AffectedPackages) {
			t.Fatalf("affected packages = %v, want %v", got, fixture.Expected.Result.AffectedPackages)
		}
		if got := sortedSetValues(prerequisites); !slices.Equal(got, fixture.Expected.Result.PrerequisitePackages) {
			t.Fatalf("prerequisite packages = %v, want %v", got, fixture.Expected.Result.PrerequisitePackages)
		}
	})
}
