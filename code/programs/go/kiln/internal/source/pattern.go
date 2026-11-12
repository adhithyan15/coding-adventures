// Package source answers "which files are we building, and what changed?"
// (KILN00 section 13).  Two implementations exist: Git, which asks git, and
// Folder, which just walks a directory, so a build from an unpacked tarball
// with no history produces the same file list and therefore the same keys.
package source

import (
	"fmt"
	"strings"
)

// Match reports whether the "/"-separated path matches the glob pattern.
//
// The glob language is deliberately tiny and is defined exactly once, here:
//
//   - any run of characters inside ONE path segment (never crosses "/")
//     ?    exactly one character inside a segment
//     **   as a WHOLE segment: zero or more whole segments, at any depth
//     else a literal character
//
// Examples:
//
//	"src/*.go"     matches "src/a.go"       not "src/x/a.go"
//	"**/*.go"      matches "a.go", "src/x/a.go"
//	"src/**"       matches "src/a", "src/x/y"   (but not "src" itself)
//	"a**b"         is two ordinary stars inside one segment (not special)
//
// Matching is case-sensitive and works on UTF-8 characters, not bytes.
func Match(pattern, path string) bool {
	return matchSegs(strings.Split(pattern, "/"), strings.Split(path, "/"))
}

func matchSegs(pat, p []string) bool {
	for len(pat) > 0 {
		if pat[0] == "**" {
			// Collapse runs of "**" and try every possible split point.
			for len(pat) > 1 && pat[1] == "**" {
				pat = pat[1:]
			}
			if len(pat) == 1 {
				return len(p) > 0 // "x/**" needs at least one more segment
			}
			for i := 0; i <= len(p); i++ {
				if matchSegs(pat[1:], p[i:]) {
					return true
				}
			}
			return false
		}
		if len(p) == 0 || !matchSeg(pat[0], p[0]) {
			return false
		}
		pat, p = pat[1:], p[1:]
	}
	return len(p) == 0
}

// matchSeg matches one segment with "*" and "?" using the classic
// two-pointer backtracking algorithm (linear space, no recursion).
func matchSeg(pat, s string) bool {
	pr, sr := []rune(pat), []rune(s)
	pi, si := 0, 0
	star, mark := -1, 0
	for si < len(sr) {
		switch {
		case pi < len(pr) && pr[pi] == '*':
			star, mark = pi, si
			pi++
		case pi < len(pr) && (pr[pi] == '?' || pr[pi] == sr[si]):
			pi++
			si++
		case star >= 0:
			pi = star + 1
			mark++
			si = mark
		default:
			return false
		}
	}
	for pi < len(pr) && pr[pi] == '*' {
		pi++
	}
	return pi == len(pr)
}

// ValidatePattern rejects patterns that could never be meaningful: empty,
// absolute, or containing empty / ".." segments.
func ValidatePattern(pattern string) error {
	if pattern == "" {
		return fmt.Errorf("empty pattern")
	}
	if pattern[0] == '/' {
		return fmt.Errorf("pattern %q: leading slash", pattern)
	}
	for _, seg := range strings.Split(pattern, "/") {
		if seg == "" || seg == ".." {
			return fmt.Errorf("pattern %q: empty or %q segment", pattern, "..")
		}
	}
	return nil
}

// matchAny reports whether the path matches at least one pattern.  An empty
// pattern list means "everything".
func matchAny(patterns []string, path string) bool {
	if len(patterns) == 0 {
		return true
	}
	for _, p := range patterns {
		if Match(p, path) {
			return true
		}
	}
	return false
}
