package graphdiff

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

var expectedGraphFixtures = []string{
	"graph-canonical-edge-order.json",
	"graph-chain.json",
	"graph-cycle.json",
	"graph-diamond.json",
	"graph-empty.json",
	"graph-isolated.json",
	"graph-multiple-components.json",
	"graph-partial-cycle-no-output.json",
}

var expectedDiffFixtures = []string{
	"diff-selection-exact-build-fronts.json",
	"diff-selection-forced-package.json",
	"diff-selection-known-unmatched-near-build.json",
	"diff-selection-match-work-at-limit.json",
	"diff-selection-match-work-over-limit.json",
	"diff-selection-package-prefix.json",
	"diff-selection-repository-boundary.json",
	"diff-selection-strict-glob-character-classes.json",
	"diff-selection-transitive.json",
	"diff-selection-unknown-all.json",
	"diff-selection-unknown-error.json",
}

type neutralFixture struct {
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

func repoRoot(t *testing.T) string {
	t.Helper()
	directory, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		candidate := filepath.Join(directory, "code", "specs", "fixtures", "build-tool-v1")
		if info, statErr := os.Stat(candidate); statErr == nil && info.IsDir() {
			return directory
		}
		parent := filepath.Dir(directory)
		if parent == directory {
			t.Fatal("repository root not found")
		}
		directory = parent
	}
}

func fixtureRoot(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "code", "specs", "fixtures", "build-tool-v1")
}

func loadFixture(t *testing.T, name string) neutralFixture {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(fixtureRoot(t), "cases", name))
	if err != nil {
		t.Fatal(err)
	}
	var fixture neutralFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode %s: %v", name, err)
	}
	return fixture
}

func loadBoundary(t *testing.T) RepositoryBoundary {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(fixtureRoot(t), "repository-source-input-boundary.json"))
	if err != nil {
		t.Fatal(err)
	}
	var boundary RepositoryBoundary
	if err := json.Unmarshal(data, &boundary); err != nil {
		t.Fatal(err)
	}
	return boundary
}

func edgeValues(values [][2]string) []Edge {
	result := make([]Edge, len(values))
	for index, value := range values {
		result[index] = Edge{Prerequisite: value[0], Dependent: value[1]}
	}
	return result
}

func graphInput(t *testing.T, fixture neutralFixture) GraphInput {
	t.Helper()
	packages := make([]string, len(fixture.Input.Options.Packages))
	for index, raw := range fixture.Input.Options.Packages {
		if err := json.Unmarshal(raw, &packages[index]); err != nil {
			t.Fatal(err)
		}
	}
	return GraphInput{Packages: packages, Edges: edgeValues(fixture.Input.Options.Edges)}
}

func diffInput(t *testing.T, fixture neutralFixture) DiffSelectionInput {
	t.Helper()
	packages := make([]PackageSpec, len(fixture.Input.Options.Packages))
	for index, raw := range fixture.Input.Options.Packages {
		if err := json.Unmarshal(raw, &packages[index]); err != nil {
			t.Fatal(err)
		}
	}
	input := DiffSelectionInput{
		Packages:          packages,
		Edges:             edgeValues(fixture.Input.Options.Edges),
		ForcedPackages:    fixture.Input.Options.ForcedPackages,
		UnknownPathPolicy: fixture.Input.Options.UnknownPathPolicy,
		ChangedPaths:      fixture.Input.ChangedPaths,
		BoundarySHA256:    fixture.Input.Options.BoundarySHA256,
	}
	if input.BoundarySHA256 != "" {
		boundary := loadBoundary(t)
		input.Boundary = &boundary
	}
	return input
}

func fixtureNames(t *testing.T, pattern string) []string {
	t.Helper()
	paths, err := filepath.Glob(filepath.Join(fixtureRoot(t), "cases", pattern))
	if err != nil {
		t.Fatal(err)
	}
	names := make([]string, len(paths))
	for index, path := range paths {
		names[index] = filepath.Base(path)
	}
	sort.Strings(names)
	return names
}

func TestConsumesEveryNeutralGraphFixture(t *testing.T) {
	if got := fixtureNames(t, "graph-*.json"); !reflect.DeepEqual(got, expectedGraphFixtures) {
		t.Fatalf("graph fixture roster = %v, want %v", got, expectedGraphFixtures)
	}
	for _, name := range expectedGraphFixtures {
		fixture := loadFixture(t, name)
		t.Run(fixture.ID, func(t *testing.T) {
			actual, err := EvaluateGraph(graphInput(t, fixture))
			if err != nil {
				t.Fatalf("EvaluateGraph: %v", err)
			}
			if fixture.Expected.Outcome == "error" {
				if actual.ErrorCode != fixture.Expected.Diagnostics[0].Code || len(actual.Edges) != 0 || len(actual.Levels) != 0 {
					t.Fatalf("error result = %#v", actual)
				}
				return
			}
			if actual.ErrorCode != "" || !reflect.DeepEqual(actual.Edges, edgeValues(fixture.Expected.Result.Edges)) || !reflect.DeepEqual(actual.Levels, fixture.Expected.Result.Levels) {
				t.Fatalf("result = %#v, want %#v", actual, fixture.Expected.Result)
			}
		})
	}
}

func TestConsumesEveryNeutralDiffFixture(t *testing.T) {
	if got := fixtureNames(t, "diff-selection-*.json"); !reflect.DeepEqual(got, expectedDiffFixtures) {
		t.Fatalf("diff fixture roster = %v, want %v", got, expectedDiffFixtures)
	}
	for _, name := range expectedDiffFixtures {
		fixture := loadFixture(t, name)
		t.Run(fixture.ID, func(t *testing.T) {
			actual, err := EvaluateDiffSelection(diffInput(t, fixture))
			if err != nil {
				t.Fatalf("EvaluateDiffSelection: %v", err)
			}
			if fixture.Expected.Outcome == "error" {
				if actual.ErrorCode != fixture.Expected.Diagnostics[0].Code || len(actual.ChangedPackages) != 0 || len(actual.AffectedPackages) != 0 || len(actual.PrerequisitePackages) != 0 {
					t.Fatalf("error result = %#v", actual)
				}
				return
			}
			if actual.ErrorCode != "" || !reflect.DeepEqual(actual.ChangedPackages, fixture.Expected.Result.ChangedPackages) || !reflect.DeepEqual(actual.AffectedPackages, fixture.Expected.Result.AffectedPackages) || !reflect.DeepEqual(actual.PrerequisitePackages, fixture.Expected.Result.PrerequisitePackages) {
				t.Fatalf("result = %#v, want %#v", actual, fixture.Expected.Result)
			}
		})
	}
}

func TestMatchWorkPreflightSkipsMatcherAboveLimit(t *testing.T) {
	original := matchPath
	t.Cleanup(func() { matchPath = original })
	calls := 0
	matchPath = func(pattern, path string) bool {
		calls++
		return globmatch.MatchPath(pattern, path)
	}
	exact, err := EvaluateDiffSelection(diffInput(t, loadFixture(t, "diff-selection-match-work-at-limit.json")))
	if err != nil || exact.ErrorCode != "" || calls == 0 {
		t.Fatalf("at-limit result=%#v err=%v calls=%d", exact, err, calls)
	}
	matchPath = func(_, _ string) bool { t.Fatal("matcher called above limit"); return false }
	over, err := EvaluateDiffSelection(diffInput(t, loadFixture(t, "diff-selection-match-work-over-limit.json")))
	if err != nil || over.ErrorCode != "DIFF_MATCH_LIMIT_EXCEEDED" {
		t.Fatalf("over-limit result=%#v err=%v", over, err)
	}
}

func TestStructuralValidationAndPrecedence(t *testing.T) {
	tests := []struct {
		name  string
		input GraphInput
		code  string
	}{
		{"duplicate package", GraphInput{Packages: []string{"fixture/a", "fixture/a"}}, "GRAPH_PACKAGE_DUPLICATE"},
		{"self edge", GraphInput{Packages: []string{"fixture/a"}, Edges: []Edge{{"fixture/a", "fixture/a"}}}, "GRAPH_EDGE_SELF"},
		{"unknown edge", GraphInput{Packages: []string{"fixture/a"}, Edges: []Edge{{"fixture/a", "fixture/b"}}}, "GRAPH_EDGE_UNKNOWN"},
		{"duplicate edge", GraphInput{Packages: []string{"fixture/a", "fixture/b"}, Edges: []Edge{{"fixture/a", "fixture/b"}, {"fixture/a", "fixture/b"}}}, "GRAPH_EDGE_DUPLICATE"},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			_, err := EvaluateGraph(test.input)
			if err == nil || err.Error() != test.code {
				t.Fatalf("error = %v, want %s", err, test.code)
			}
		})
	}

	cycle := DiffSelectionInput{
		Packages: []PackageSpec{{Name: "fixture/a", RelPath: "a", SourceMode: "package_prefix"}, {Name: "fixture/b", RelPath: "b", SourceMode: "package_prefix"}},
		Edges:    []Edge{{"fixture/a", "fixture/b"}, {"fixture/b", "fixture/a"}}, UnknownPathPolicy: "error",
	}
	if _, err := EvaluateDiffSelection(cycle); err == nil || err.Error() != "DIFF_EDGE_CYCLE" {
		t.Fatalf("cycle error = %v", err)
	}

	alias := DiffSelectionInput{
		Packages: []PackageSpec{{Name: "fixture/a", RelPath: "code/P", SourceMode: "package_prefix"}, {Name: "fixture/b", RelPath: "code/p/child", SourceMode: "package_prefix"}}, UnknownPathPolicy: "error",
	}
	if _, err := EvaluateDiffSelection(alias); err == nil || err.Error() != "DIFF_PATH_INVALID" {
		t.Fatalf("alias error = %v", err)
	}

	badGlob := DiffSelectionInput{
		Packages:          []PackageSpec{{Name: "fixture/a", RelPath: "p", SourceMode: "strict_globs", SourceGlobs: []string{"src/**", "[z-a]"}}},
		UnknownPathPolicy: "error", ChangedPaths: []string{"p/src/value.py"},
	}
	original := matchPath
	t.Cleanup(func() { matchPath = original })
	matchPath = func(_, _ string) bool { t.Fatal("matcher called before whole-list validation"); return false }
	if _, err := EvaluateDiffSelection(badGlob); err == nil || err.Error() != "DIFF_GLOB_INVALID" {
		t.Fatalf("glob error = %v", err)
	}
	badGlob.Packages[0].SourceGlobs = []string{"src/?.py"}
	if _, err := EvaluateDiffSelection(badGlob); err == nil || err.Error() != "DIFF_GLOB_INVALID" {
		t.Fatalf("question-mark glob error = %v", err)
	}
	for _, pattern := range []string{"src/[a--b].py", "src/[a&&b].py", "src/[a~~b].py", "src/[a||b].py"} {
		badGlob.Packages[0].SourceGlobs = []string{pattern}
		if _, err := EvaluateDiffSelection(badGlob); err == nil || err.Error() != "DIFF_GLOB_INVALID" {
			t.Fatalf("ambiguous glob %q error = %v", pattern, err)
		}
	}

	boundaryFirst := diffInput(t, loadFixture(t, "diff-selection-match-work-over-limit.json"))
	boundary := loadBoundary(t)
	boundaryFirst.Boundary = &boundary
	boundaryFirst.BoundarySHA256 = strings.Repeat("0", 64)
	if _, err := EvaluateDiffSelection(boundaryFirst); err == nil || err.Error() != "DIFF_BOUNDARY_DIGEST_MISMATCH" {
		t.Fatalf("boundary precedence error = %v", err)
	}

	matchPath = func(_, _ string) bool { t.Fatal("matcher called above limit"); return false }
	limitFirst := diffInput(t, loadFixture(t, "diff-selection-match-work-over-limit.json"))
	limitFirst.ChangedPaths = append(limitFirst.ChangedPaths, "outside/unknown.txt")
	result, err := EvaluateDiffSelection(limitFirst)
	if err != nil || result.ErrorCode != "DIFF_MATCH_LIMIT_EXCEEDED" {
		t.Fatalf("match-limit precedence result=%#v error=%v", result, err)
	}
}

func TestBoundaryDigestAndCallerInputsAreStable(t *testing.T) {
	boundary := loadBoundary(t)
	digest, err := boundary.Digest()
	if err != nil {
		t.Fatal(err)
	}
	if digest != "7983f42a84dc9905f50729798a5d7d4000a4356016eb2b9cfd42217b15070b59" {
		t.Fatalf("boundary digest = %s", digest)
	}

	input := diffInput(t, loadFixture(t, "diff-selection-transitive.json"))
	before, err := json.Marshal(input)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := EvaluateDiffSelection(input); err != nil {
		t.Fatal(err)
	}
	after, err := json.Marshal(input)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(before, after) {
		t.Fatal("EvaluateDiffSelection mutated caller input")
	}
}

func TestCanonicalBoundaryJSONUsesRawUTF8AndEmptyArrays(t *testing.T) {
	document := map[string]any{
		"schema_version":                        1,
		"language_source_input_registry_sha256": "<tag>&\u2028\u2029",
		"boundaries": []any{
			map[string]any{
				"applies_to": map[string]any{
					"exact_roots":      canonicalStringList(nil),
					"descendant_roots": canonicalStringList(nil),
					"excluded_roots":   canonicalStringList(nil),
				},
			},
		},
	}
	actual, err := canonicalJSON(document)
	if err != nil {
		t.Fatal(err)
	}
	expected := `{"boundaries":[{"applies_to":{"descendant_roots":[],"exact_roots":[],"excluded_roots":[]}}],"language_source_input_registry_sha256":"<tag>&  ","schema_version":1}`
	if string(actual) != expected {
		t.Fatalf("canonical JSON = %q, want %q", actual, expected)
	}

	literalEscape, err := canonicalJSON(map[string]any{"value": `\u2028`})
	if err != nil {
		t.Fatal(err)
	}
	if string(literalEscape) != `{"value":"\\u2028"}` {
		t.Fatalf("literal escape changed: %q", literalEscape)
	}
}

func TestGraphResourceCeilings(t *testing.T) {
	packages := make([]string, MaxPackages)
	// Use a compact deterministic sequence while retaining unique valid names.
	for index := range packages {
		packages[index] = "fixture/p-" + base36(index)
	}
	result, err := EvaluateGraph(GraphInput{Packages: packages})
	if err != nil || len(result.Levels) != 1 {
		t.Fatalf("exact package limit result=%#v err=%v", result, err)
	}
	packages = append(packages, "fixture/overflow")
	if _, err := EvaluateGraph(GraphInput{Packages: packages}); err == nil || err.Error() != "GRAPH_PACKAGE_LIMIT_EXCEEDED" {
		t.Fatalf("package ceiling error = %v", err)
	}
	edges := make([]Edge, MaxEdges+1)
	for index := range edges {
		edges[index] = Edge{"fixture/a", "fixture/b"}
	}
	if _, err := EvaluateGraph(GraphInput{Packages: []string{"fixture/a", "fixture/b"}, Edges: edges}); err == nil || err.Error() != "GRAPH_EDGE_LIMIT_EXCEEDED" {
		t.Fatalf("edge ceiling error = %v", err)
	}
}

func base36(value int) string {
	const digits = "0123456789abcdefghijklmnopqrstuvwxyz"
	if value == 0 {
		return "0"
	}
	var result []byte
	for value > 0 {
		result = append(result, digits[value%36])
		value /= 36
	}
	for left, right := 0, len(result)-1; left < right; left, right = left+1, right-1 {
		result[left], result[right] = result[right], result[left]
	}
	return string(result)
}
