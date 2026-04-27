package starlark

import (
	"os"
	"testing"
)

// TestIsStarlarkBuild verifies detection of Starlark vs shell BUILD files.
func TestIsStarlarkBuild(t *testing.T) {
	tests := []struct {
		name    string
		content string
		want    bool
	}{
		{
			name:    "load statement",
			content: `load("code/packages/starlark/library-rules/python_library.star", "py_library")`,
			want:    true,
		},
		{
			name: "load with leading comments",
			content: `# This is a comment
# Another comment

load("code/packages/starlark/library-rules/go_library.star", "go_library")`,
			want: true,
		},
		{
			name:    "py_library call",
			content: `py_library(name = "foo", srcs = ["src/**/*.py"])`,
			want:    true,
		},
		{
			name:    "go_library call",
			content: `go_library(name = "bar")`,
			want:    true,
		},
		{
			name:    "def statement",
			content: `def my_rule(name): pass`,
			want:    true,
		},
		{
			name:    "shell command",
			content: `go build ./...`,
			want:    false,
		},
		{
			name: "shell with comments",
			content: `# Build and test
go build ./...
go test ./... -v`,
			want: false,
		},
		{
			name:    "empty file",
			content: "",
			want:    false,
		},
		{
			name:    "only comments",
			content: "# just a comment\n# another",
			want:    false,
		},
		{
			name:    "echo command",
			content: `echo "hello"`,
			want:    false,
		},
		{
			name:    "ts_library call",
			content: `ts_library(name = "foo")`,
			want:    true,
		},
		{
			name:    "rust_binary call",
			content: `rust_binary(name = "foo")`,
			want:    true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsStarlarkBuild(tt.content)
			if got != tt.want {
				t.Errorf("IsStarlarkBuild(%q) = %v, want %v", tt.content, got, tt.want)
			}
		})
	}
}

// TestExtractTargets verifies target extraction from Starlark variables.
func TestExtractTargets(t *testing.T) {
	t.Run("no targets variable", func(t *testing.T) {
		vars := map[string]interface{}{}
		targets, err := extractTargets(vars)
		if err != nil {
			t.Errorf("unexpected error: %v", err)
		}
		if targets != nil {
			t.Errorf("expected nil targets, got %v", targets)
		}
	})

	t.Run("single target", func(t *testing.T) {
		vars := map[string]interface{}{
			"_targets": []interface{}{
				map[string]interface{}{
					"rule": "py_library",
					"name": "logic-gates",
					"srcs": []interface{}{"src/**/*.py"},
					"deps": []interface{}{"python/transistors"},
					"test_runner": "pytest",
				},
			},
		}
		targets, err := extractTargets(vars)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if len(targets) != 1 {
			t.Fatalf("expected 1 target, got %d", len(targets))
		}
		if targets[0].Rule != "py_library" {
			t.Errorf("rule = %q, want py_library", targets[0].Rule)
		}
		if targets[0].Name != "logic-gates" {
			t.Errorf("name = %q, want logic-gates", targets[0].Name)
		}
		if len(targets[0].Srcs) != 1 || targets[0].Srcs[0] != "src/**/*.py" {
			t.Errorf("srcs = %v, want [src/**/*.py]", targets[0].Srcs)
		}
		if len(targets[0].Deps) != 1 || targets[0].Deps[0] != "python/transistors" {
			t.Errorf("deps = %v, want [python/transistors]", targets[0].Deps)
		}
		if targets[0].TestRunner != "pytest" {
			t.Errorf("test_runner = %q, want pytest", targets[0].TestRunner)
		}
	})

	t.Run("multiple targets", func(t *testing.T) {
		vars := map[string]interface{}{
			"_targets": []interface{}{
				map[string]interface{}{
					"rule": "go_library",
					"name": "directed-graph",
					"srcs": []interface{}{"*.go"},
					"deps": []interface{}{},
				},
				map[string]interface{}{
					"rule": "go_binary",
					"name": "build-tool",
					"srcs": []interface{}{"*.go", "internal/**/*.go"},
					"deps": []interface{}{"go/directed-graph", "go/progress-bar"},
				},
			},
		}
		targets, err := extractTargets(vars)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if len(targets) != 2 {
			t.Fatalf("expected 2 targets, got %d", len(targets))
		}
	})

	t.Run("targets not a list", func(t *testing.T) {
		vars := map[string]interface{}{
			"_targets": "not a list",
		}
		_, err := extractTargets(vars)
		if err == nil {
			t.Error("expected error for non-list _targets")
		}
	})
}

// TestGenerateCommands verifies command generation for each rule type.
func TestGenerateCommands(t *testing.T) {
	tests := []struct {
		name     string
		target   Target
		wantLen  int
		wantHas  string
	}{
		{
			name:    "py_library with pytest",
			target:  Target{Rule: "py_library", TestRunner: "pytest"},
			wantLen: 2,
			wantHas: "pytest",
		},
		{
			name:    "py_library with unittest",
			target:  Target{Rule: "py_library", TestRunner: "unittest"},
			wantLen: 2,
			wantHas: "unittest",
		},
		{
			name:    "go_library",
			target:  Target{Rule: "go_library"},
			wantLen: 3,
			wantHas: "go test",
		},
		{
			name:    "ruby_library",
			target:  Target{Rule: "ruby_library"},
			wantLen: 2,
			wantHas: "bundle",
		},
		{
			name:    "ts_library",
			target:  Target{Rule: "ts_library"},
			wantLen: 2,
			wantHas: "vitest",
		},
		{
			name:    "rust_library",
			target:  Target{Rule: "rust_library"},
			wantLen: 2,
			wantHas: "cargo",
		},
		{
			name:    "elixir_library",
			target:  Target{Rule: "elixir_library"},
			wantLen: 2,
			wantHas: "mix",
		},
		{
			name:    "perl_library",
			target:  Target{Rule: "perl_library"},
			wantLen: 2,
			wantHas: "prove",
		},
		{
			name:    "unknown rule",
			target:  Target{Rule: "unknown_rule"},
			wantLen: 1,
			wantHas: "Unknown rule",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmds := GenerateCommands(tt.target, "")
			if len(cmds) != tt.wantLen {
				t.Errorf("got %d commands, want %d: %v", len(cmds), tt.wantLen, cmds)
			}
			found := false
			for _, cmd := range cmds {
				if containsStr(cmd, tt.wantHas) {
					found = true
					break
				}
			}
			if !found {
				t.Errorf("commands %v should contain %q", cmds, tt.wantHas)
			}
		})
	}
}

func containsStr(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && findSubstring(s, substr))
}

func findSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// TestGetString verifies safe string extraction from dicts.
func TestGetString(t *testing.T) {
	dict := map[string]interface{}{
		"name": "foo",
		"count": 42,
	}

	if got := getString(dict, "name"); got != "foo" {
		t.Errorf("getString(name) = %q, want foo", got)
	}
	if got := getString(dict, "missing"); got != "" {
		t.Errorf("getString(missing) = %q, want empty", got)
	}
	if got := getString(dict, "count"); got != "" {
		t.Errorf("getString(count) = %q, want empty (not a string)", got)
	}
}

// TestGetStringList verifies safe string list extraction from dicts.
func TestGetStringList(t *testing.T) {
	dict := map[string]interface{}{
		"srcs": []interface{}{"a.py", "b.py"},
		"bad":  "not a list",
		"mixed": []interface{}{"a", 42, "b"},
	}

	srcs := getStringList(dict, "srcs")
	if len(srcs) != 2 || srcs[0] != "a.py" || srcs[1] != "b.py" {
		t.Errorf("getStringList(srcs) = %v, want [a.py b.py]", srcs)
	}

	if got := getStringList(dict, "missing"); got != nil {
		t.Errorf("getStringList(missing) = %v, want nil", got)
	}

	if got := getStringList(dict, "bad"); got != nil {
		t.Errorf("getStringList(bad) = %v, want nil", got)
	}

	// Mixed list should only include strings.
	mixed := getStringList(dict, "mixed")
	if len(mixed) != 2 || mixed[0] != "a" || mixed[1] != "b" {
		t.Errorf("getStringList(mixed) = %v, want [a b]", mixed)
	}
}

// TestExtractPyMonorepoDeps verifies parsing of monorepo deps from pyproject.toml.
func TestExtractPyMonorepoDeps(t *testing.T) {
	tests := []struct {
		name string
		text string
		want []string
	}{
		{
			name: "single dep",
			text: `[project]
name = "coding-adventures-logic-gates"
dependencies = ["coding-adventures-transistors>=0.1.0"]`,
			want: []string{"transistors"},
		},
		{
			name: "multiple deps",
			text: `[project]
dependencies = [
    "coding-adventures-lexer>=0.1.0",
    "coding-adventures-grammar-tools>=0.1.0",
    "pytest>=7.0",
]`,
			want: []string{"lexer", "grammar-tools"},
		},
		{
			name: "no monorepo deps",
			text: `[project]
dependencies = ["pytest>=7.0", "numpy>=1.0"]`,
			want: nil,
		},
		{
			name: "no dependencies section",
			text: `[project]
name = "foo"`,
			want: nil,
		},
		{
			name: "single line array",
			text: `dependencies = ["coding-adventures-vm>=0.1.0", "coding-adventures-compiler"]`,
			want: []string{"vm", "compiler"},
		},
		{
			name: "dep with version specifiers",
			text: `dependencies = ["coding-adventures-parser~=1.2.0"]`,
			want: []string{"parser"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := extractPyMonorepoDeps(tt.text)
			if len(got) != len(tt.want) {
				t.Fatalf("extractPyMonorepoDeps() = %v, want %v", got, tt.want)
			}
			for i := range got {
				if got[i] != tt.want[i] {
					t.Errorf("dep[%d] = %q, want %q", i, got[i], tt.want[i])
				}
			}
		})
	}
}

// TestPyInstallCmd verifies the install command includes monorepo deps.
func TestPyInstallCmd(t *testing.T) {
	// With empty path, should return default command.
	cmd := pyInstallCmd("")
	if cmd != `uv pip install --system -e ".[dev]"` {
		t.Errorf("pyInstallCmd(\"\") = %q, want default", cmd)
	}

	// With nonexistent path, should return default command.
	cmd = pyInstallCmd("/nonexistent/path")
	if cmd != `uv pip install --system -e ".[dev]"` {
		t.Errorf("pyInstallCmd(nonexistent) = %q, want default", cmd)
	}

	// Create a temp dir structure to test transitive dep discovery.
	// parent/
	//   pkg-a/pyproject.toml  (depends on pkg-b)
	//   pkg-b/pyproject.toml  (depends on pkg-c)
	//   pkg-c/pyproject.toml  (no monorepo deps)
	parent := t.TempDir()

	// pkg-c: leaf, no monorepo deps
	os.MkdirAll(parent+"/pkg-c", 0755)
	os.WriteFile(parent+"/pkg-c/pyproject.toml", []byte(`[project]
name = "coding-adventures-pkg-c"
dependencies = ["pytest>=7.0"]`), 0644)

	// pkg-b: depends on pkg-c
	os.MkdirAll(parent+"/pkg-b", 0755)
	os.WriteFile(parent+"/pkg-b/pyproject.toml", []byte(`[project]
name = "coding-adventures-pkg-b"
dependencies = ["coding-adventures-pkg-c"]`), 0644)

	// pkg-a: depends on pkg-b
	os.MkdirAll(parent+"/pkg-a", 0755)
	os.WriteFile(parent+"/pkg-a/pyproject.toml", []byte(`[project]
name = "coding-adventures-pkg-a"
dependencies = ["coding-adventures-pkg-b"]`), 0644)

	cmd = pyInstallCmd(parent + "/pkg-a")
	// Should install pkg-c first (leaf), then pkg-b, then pkg-a's own deps
	expected := `uv pip install --system -e ../pkg-c -e ../pkg-b -e ".[dev]"`
	if cmd != expected {
		t.Errorf("pyInstallCmd(transitive) = %q, want %q", cmd, expected)
	}

	// pkg-c has no monorepo deps, should get default command.
	cmd = pyInstallCmd(parent + "/pkg-c")
	if cmd != `uv pip install --system -e ".[dev]"` {
		t.Errorf("pyInstallCmd(leaf) = %q, want default", cmd)
	}
}

// TestTsInstallCmd verifies the npm install command chains transitive deps.
func TestTsInstallCmd(t *testing.T) {
	// With empty path, should return default command.
	cmd := tsInstallCmd("")
	if cmd != "npm install --silent" {
		t.Errorf("tsInstallCmd(\"\") = %q, want default", cmd)
	}

	// Create a temp dir structure with transitive file: deps.
	// parent/
	//   pkg-a/package.json  (depends on pkg-b)
	//   pkg-b/package.json  (depends on pkg-c)
	//   pkg-c/package.json  (no file: deps)
	parent := t.TempDir()

	os.MkdirAll(parent+"/pkg-c", 0755)
	os.WriteFile(parent+"/pkg-c/package.json", []byte(`{
  "name": "@ca/pkg-c",
  "dependencies": {}
}`), 0644)

	os.MkdirAll(parent+"/pkg-b", 0755)
	os.WriteFile(parent+"/pkg-b/package.json", []byte(`{
  "name": "@ca/pkg-b",
  "dependencies": {
    "@ca/pkg-c": "file:../pkg-c"
  }
}`), 0644)

	os.MkdirAll(parent+"/pkg-a", 0755)
	os.WriteFile(parent+"/pkg-a/package.json", []byte(`{
  "name": "@ca/pkg-a",
  "dependencies": {
    "@ca/pkg-b": "file:../pkg-b"
  }
}`), 0644)

	cmd = tsInstallCmd(parent + "/pkg-a")
	// Should install pkg-c first (leaf), then pkg-b, then pkg-a
	if !containsStr(cmd, "cd ../pkg-c") {
		t.Errorf("tsInstallCmd() missing transitive dep pkg-c: %q", cmd)
	}
	if !containsStr(cmd, "cd ../pkg-b") {
		t.Errorf("tsInstallCmd() missing direct dep pkg-b: %q", cmd)
	}

	// pkg-c has no file: deps, should get default command.
	cmd = tsInstallCmd(parent + "/pkg-c")
	if cmd != "npm install --silent" {
		t.Errorf("tsInstallCmd(leaf) = %q, want default", cmd)
	}
}

// TestEnhanceInstallCommands verifies that rendered Starlark commands
// get their install commands replaced with auto-discovered versions.
func TestEnhanceInstallCommands(t *testing.T) {
	// Create a temp package with monorepo deps.
	parent := t.TempDir()
	os.MkdirAll(parent+"/my-pkg", 0755)
	os.MkdirAll(parent+"/dep-a", 0755)
	os.WriteFile(parent+"/my-pkg/pyproject.toml", []byte(`[project]
name = "coding-adventures-my-pkg"
dependencies = ["coding-adventures-dep-a"]`), 0644)
	os.WriteFile(parent+"/dep-a/pyproject.toml", []byte(`[project]
name = "coding-adventures-dep-a"
dependencies = []`), 0644)

	cmds := []string{
		`uv pip install --system -e .[dev]`,
		`python -m pytest --cov --cov-report=term-missing`,
	}

	enhanced := EnhanceInstallCommands(cmds, parent+"/my-pkg")
	if enhanced[0] == cmds[0] {
		t.Errorf("install command not enhanced: %q", enhanced[0])
	}
	if !containsStr(enhanced[0], "-e ../dep-a") {
		t.Errorf("enhanced command missing dep: %q", enhanced[0])
	}
	// Test command should be unchanged.
	if enhanced[1] != cmds[1] {
		t.Errorf("test command changed: %q", enhanced[1])
	}

	// Empty path should return commands unchanged.
	unchanged := EnhanceInstallCommands(cmds, "")
	if unchanged[0] != cmds[0] {
		t.Errorf("empty path should not enhance: %q", unchanged[0])
	}
}
