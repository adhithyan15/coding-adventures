package starlark

import (
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
			name:    "unknown rule",
			target:  Target{Rule: "unknown_rule"},
			wantLen: 1,
			wantHas: "Unknown rule",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmds := GenerateCommands(tt.target)
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
