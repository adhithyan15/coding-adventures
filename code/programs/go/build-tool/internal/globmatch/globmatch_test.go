package globmatch

import "testing"

func TestMatchPath(t *testing.T) {
	tests := []struct {
		pattern string
		path    string
		want    bool
	}{
		// ── Double-star (**) patterns ─────────────────────────────
		// ** matches zero or more complete path segments.

		{"src/**/*.py", "src/foo.py", true},
		{"src/**/*.py", "src/a/b/c.py", true},
		{"src/**/*.py", "src/a/b.py", true},
		{"src/**/*.py", "tests/foo.py", false},
		{"src/**/*.py", "src/foo.txt", false},

		{"**/*.py", "foo.py", true},
		{"**/*.py", "a/b/c.py", true},
		{"**/*.py", "foo.txt", false},

		{"**", "anything", true},
		{"**", "a/b/c", true},
		{"**", "", true}, // ** matches zero segments

		{"src/**/test_*.py", "src/test_foo.py", true},
		{"src/**/test_*.py", "src/a/b/test_bar.py", true},
		{"src/**/test_*.py", "src/a/b/foo.py", false},

		// ** at the end matches everything below.
		{"src/**", "src/foo.py", true},
		{"src/**", "src/a/b/c.py", true},
		{"src/**", "src", true}, // ** can match zero segments, so src/ + nothing = src

		// Consecutive ** segments collapse to one.
		{"**/**/*.py", "a/b.py", true},
		{"**/**/**", "x/y/z", true},

		// ── Single-star (*) patterns ─────────────────────────────
		// * matches within a single path segment.

		{"*.py", "foo.py", true},
		{"*.py", "bar.py", true},
		{"*.py", "dir/foo.py", false},
		{"*.toml", "pyproject.toml", true},
		{"src/*.py", "src/foo.py", true},
		{"src/*.py", "src/a/foo.py", false},

		// ── Question mark (?) patterns ───────────────────────────
		{"?.py", "a.py", true},
		{"?.py", "ab.py", false},

		// ── Literal (no wildcards) ───────────────────────────────
		{"pyproject.toml", "pyproject.toml", true},
		{"pyproject.toml", "other.toml", false},
		{"src/main.py", "src/main.py", true},
		{"src/main.py", "src/other.py", false},

		// ── Character classes ────────────────────────────────────
		{"*.[ch]", "foo.c", true},
		{"*.[ch]", "foo.h", true},
		{"*.[ch]", "foo.py", false},

		// ── Edge cases ───────────────────────────────────────────
		{"", "", true},       // empty matches empty
		{"", "a", false},     // empty pattern doesn't match non-empty
		{"a", "", false},     // non-empty pattern doesn't match empty
		{"*", "", false},     // * needs at least one character in a segment
		{"**", "", true},     // ** can match zero segments
		{"a/b/c", "a/b/c", true},
		{"a/b/c", "a/b/d", false},

		// Trailing slashes are normalized.
		{"src/", "src", true},
		{"src/**/*.py", "src/foo.py", true},

		// Patterns from real Starlark BUILD files.
		{"src/**/*.py", "src/build_tool/cli.py", true},
		{"tests/**/*.py", "tests/test_hasher.py", true},
		{"src/**/*.ex", "src/build_tool/glob_match.ex", true},
		{"lib/**/*.rb", "lib/build_tool/plan.rb", true},
	}

	for _, tt := range tests {
		got := MatchPath(tt.pattern, tt.path)
		if got != tt.want {
			t.Errorf("MatchPath(%q, %q) = %v, want %v",
				tt.pattern, tt.path, got, tt.want)
		}
	}
}

func TestSplitPath(t *testing.T) {
	tests := []struct {
		input string
		want  int // expected number of segments
	}{
		{"", 0},
		{"a", 1},
		{"a/b/c", 3},
		{"/a/b/", 2},    // leading/trailing slashes ignored
		{"a//b", 2},      // double slashes collapsed
	}

	for _, tt := range tests {
		got := splitPath(tt.input)
		if len(got) != tt.want {
			t.Errorf("splitPath(%q) returned %d segments, want %d: %v",
				tt.input, len(got), tt.want, got)
		}
	}
}
