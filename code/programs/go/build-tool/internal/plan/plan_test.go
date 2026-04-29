package plan

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestRoundTrip(t *testing.T) {
	bp := &BuildPlan{
		DiffBase:         "origin/main",
		Force:            false,
		AffectedPackages: []string{"python/foo", "go/bar"},
		Packages: []PackageEntry{
			{
				Name:          "python/foo",
				RelPath:       "code/packages/python/foo",
				Language:      "python",
				BuildCommands: []string{"python -m pip install .", "pytest"},
				IsStarlark:    true,
				DeclaredSrcs:  []string{"src/**/*.py"},
				DeclaredDeps:  []string{"python/bar"},
			},
			{
				Name:          "go/bar",
				RelPath:       "code/packages/go/bar",
				Language:      "go",
				BuildCommands: []string{"go test ./..."},
			},
		},
		DependencyEdges: [][2]string{{"python/bar", "python/foo"}},
		LanguagesNeeded: map[string]bool{"python": true, "go": true, "ruby": false},
	}

	dir := t.TempDir()
	path := filepath.Join(dir, "plan.json")

	// Write
	if err := Write(bp, path); err != nil {
		t.Fatalf("Write failed: %v", err)
	}

	// Read back
	got, err := Read(path)
	if err != nil {
		t.Fatalf("Read failed: %v", err)
	}

	// Verify schema version was stamped.
	if got.SchemaVersion != CurrentSchemaVersion {
		t.Errorf("SchemaVersion = %d, want %d", got.SchemaVersion, CurrentSchemaVersion)
	}

	if got.DiffBase != "origin/main" {
		t.Errorf("DiffBase = %q, want %q", got.DiffBase, "origin/main")
	}

	if got.Force {
		t.Error("Force should be false")
	}

	if len(got.AffectedPackages) != 2 {
		t.Errorf("AffectedPackages length = %d, want 2", len(got.AffectedPackages))
	}

	if len(got.Packages) != 2 {
		t.Errorf("Packages length = %d, want 2", len(got.Packages))
	}

	if got.Packages[0].Name != "python/foo" {
		t.Errorf("Packages[0].Name = %q, want %q", got.Packages[0].Name, "python/foo")
	}

	if !got.Packages[0].IsStarlark {
		t.Error("Packages[0].IsStarlark should be true")
	}

	if len(got.DependencyEdges) != 1 {
		t.Fatalf("DependencyEdges length = %d, want 1", len(got.DependencyEdges))
	}

	if got.DependencyEdges[0] != [2]string{"python/bar", "python/foo"} {
		t.Errorf("DependencyEdges[0] = %v, want [python/bar, python/foo]", got.DependencyEdges[0])
	}

	if !got.LanguagesNeeded["python"] {
		t.Error("LanguagesNeeded[python] should be true")
	}
	if got.LanguagesNeeded["ruby"] {
		t.Error("LanguagesNeeded[ruby] should be false")
	}
}

func TestNilVsEmptyAffectedPackages(t *testing.T) {
	dir := t.TempDir()

	// nil affected packages (force mode).
	bp := &BuildPlan{
		Force:            true,
		AffectedPackages: nil,
		Packages:         []PackageEntry{},
		DependencyEdges:  [][2]string{},
		LanguagesNeeded:  map[string]bool{},
	}

	path := filepath.Join(dir, "nil.json")
	if err := Write(bp, path); err != nil {
		t.Fatalf("Write nil: %v", err)
	}

	// Verify JSON contains null, not [].
	data, _ := os.ReadFile(path)
	var raw map[string]json.RawMessage
	json.Unmarshal(data, &raw)
	if string(raw["affected_packages"]) != "null" {
		t.Errorf("nil AffectedPackages serialized as %s, want null", raw["affected_packages"])
	}

	got, _ := Read(path)
	if got.AffectedPackages != nil {
		t.Error("nil should round-trip as nil")
	}

	// Empty affected packages (nothing changed).
	bp.AffectedPackages = []string{}
	path = filepath.Join(dir, "empty.json")
	if err := Write(bp, path); err != nil {
		t.Fatalf("Write empty: %v", err)
	}

	data, _ = os.ReadFile(path)
	json.Unmarshal(data, &raw)
	if string(raw["affected_packages"]) != "[]" {
		t.Errorf("empty AffectedPackages serialized as %s, want []", raw["affected_packages"])
	}

	got, _ = Read(path)
	if got.AffectedPackages == nil {
		t.Error("empty slice should round-trip as non-nil")
	}
	if len(got.AffectedPackages) != 0 {
		t.Error("empty slice should have length 0")
	}
}

func TestVersionRejection(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "future.json")

	// Write a plan with a future version manually.
	data := []byte(`{"schema_version": 99, "packages": [], "dependency_edges": [], "languages_needed": {}}`)
	os.WriteFile(path, data, 0644)

	_, err := Read(path)
	if err == nil {
		t.Fatal("expected error for unsupported version, got nil")
	}
	if !contains(err.Error(), "unsupported build plan version") {
		t.Errorf("error message = %q, expected 'unsupported build plan version'", err.Error())
	}
}

func TestComputeShardsAddsTransitivePrerequisites(t *testing.T) {
	bp := &BuildPlan{
		AffectedPackages: []string{"ruby/app", "python/tool"},
		Packages: []PackageEntry{
			{Name: "ruby/core", Language: "ruby", BuildCommands: []string{"ruby test.rb"}},
			{Name: "ruby/app", Language: "ruby", BuildCommands: []string{"ruby test.rb"}},
			{Name: "python/base", Language: "python", BuildCommands: []string{"pytest"}},
			{Name: "python/tool", Language: "python", BuildCommands: []string{"pytest"}},
		},
		DependencyEdges: [][2]string{
			{"ruby/core", "ruby/app"},
			{"python/base", "python/tool"},
		},
	}

	shards := ComputeShards(bp, 2)
	if len(shards) != 2 {
		t.Fatalf("len(shards) = %d, want 2", len(shards))
	}

	for _, shard := range shards {
		packages := asSet(shard.PackageNames)
		for _, assigned := range shard.AssignedPackages {
			switch assigned {
			case "ruby/app":
				if !packages["ruby/core"] {
					t.Fatalf("ruby/app shard missing ruby/core prerequisite: %#v", shard.PackageNames)
				}
			case "python/tool":
				if !packages["python/base"] {
					t.Fatalf("python/tool shard missing python/base prerequisite: %#v", shard.PackageNames)
				}
			}
		}
	}
}

func TestComputeShardsForceUsesAllPackages(t *testing.T) {
	bp := &BuildPlan{
		Force:            true,
		AffectedPackages: nil,
		Packages: []PackageEntry{
			{Name: "go/a", Language: "go"},
			{Name: "go/b", Language: "go"},
			{Name: "go/c", Language: "go"},
		},
	}

	shards := ComputeShards(bp, 2)
	seen := make(map[string]bool)
	for _, shard := range shards {
		for _, assigned := range shard.AssignedPackages {
			seen[assigned] = true
		}
	}

	for _, name := range []string{"go/a", "go/b", "go/c"} {
		if !seen[name] {
			t.Fatalf("force shard assignments missing %s", name)
		}
	}
}

func TestMatrixEntries(t *testing.T) {
	shards := []ShardEntry{
		{
			Index:           0,
			Name:            "shard-1-of-1",
			PackageNames:    []string{"ruby/app"},
			LanguagesNeeded: map[string]bool{"ruby": true},
		},
	}

	entries := MatrixEntries(shards)
	if len(entries) != 1 {
		t.Fatalf("len(entries) = %d, want 1", len(entries))
	}
	if entries[0].ShardIndex != 0 || entries[0].ShardCount != 1 {
		t.Fatalf("matrix entry = %#v, want shard 0 of 1", entries[0])
	}
	if len(entries[0].Languages) != 1 || entries[0].Languages[0] != "ruby" {
		t.Fatalf("matrix languages = %#v, want [ruby]", entries[0].Languages)
	}
}

func TestReadMissingFile(t *testing.T) {
	_, err := Read("/nonexistent/plan.json")
	if err == nil {
		t.Fatal("expected error for missing file, got nil")
	}
}

func TestReadMalformedJSON(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "bad.json")
	os.WriteFile(path, []byte("not json"), 0644)

	_, err := Read(path)
	if err == nil {
		t.Fatal("expected error for malformed JSON, got nil")
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && searchString(s, substr)
}

func asSet(values []string) map[string]bool {
	set := make(map[string]bool, len(values))
	for _, value := range values {
		set[value] = true
	}
	return set
}

func searchString(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
