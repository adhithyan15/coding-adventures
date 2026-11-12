// Package source answers "which files are we building, and what changed?"
// (KILN00 section 13).  Two implementations exist: Git, which asks git, and
// Folder, which just walks a directory, so a build from an unpacked tarball
// with no history produces the same file list and therefore the same keys.
package source

import (
	"fmt"
	"strings"
	"unicode"
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

// matchSegs is a dynamic program over (pattern segment i, path segment j):
//
//	ok[i][j] = "pat[i:] matches p[j:]"
//
// filled from the end, so each cell is computed once.  The earlier recursive
// version retried every split point for every "**", which is exponential for
// a pattern such as "**/**/**/.../x" against a deep path; this one is
// O(len(pat) * len(p)) segment comparisons.
//
//	pat[i] is "**" and only "**" remain:  matches iff at least one segment is left
//	pat[i] is "**" otherwise:             zero segments (ok[i+1][j]) or
//	                                      swallow one and stay on "**" (ok[i][j+1])
//	pat[i] is an ordinary segment:        matchSeg, then ok[i+1][j+1]
func matchSegs(pat, p []string) bool {
	P, S := len(pat), len(p)
	prunes := make([][]rune, S)
	for j, seg := range p {
		prunes[j] = []rune(seg)
	}
	ok := make([][]bool, P+1)
	for i := range ok {
		ok[i] = make([]bool, S+1)
	}
	ok[P][S] = true
	onlyStars := true // true while every pattern segment from i on is "**"
	for i := P - 1; i >= 0; i-- {
		if pat[i] != "**" {
			onlyStars = false
		}
		var pr []rune
		if pat[i] != "**" {
			pr = []rune(pat[i])
		}
		for j := S; j >= 0; j-- {
			switch {
			case pat[i] == "**" && onlyStars:
				ok[i][j] = j < S
			case pat[i] == "**":
				ok[i][j] = ok[i+1][j] || (j < S && ok[i][j+1])
			default:
				ok[i][j] = j < S && matchSeg(pr, prunes[j]) && ok[i+1][j+1]
			}
		}
	}
	return ok[0][0]
}

// matchSeg matches one segment with "*" and "?" using the classic
// two-pointer backtracking algorithm (linear space, no recursion).
func matchSeg(pr, sr []rune) bool {
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

// MaxPatternSegments caps the "/"-separated segments of one pattern.
const MaxPatternSegments = 64

// short returns at most 40 runes of s with control characters removed, for
// error messages: a hostile file must not be able to inject terminal escapes
// or flood a log through an error.
func short(s string) string {
	r := []rune(strings.Map(func(r rune) rune {
		if unicode.IsControl(r) {
			return -1
		}
		return r
	}, s))
	if len(r) > 40 {
		return string(r[:40]) + "..."
	}
	return string(r)
}

// ValidatePattern rejects patterns that could never be meaningful: empty,
// absolute, with empty / ".." segments, or with more than MaxPatternSegments
// segments.
func ValidatePattern(pattern string) error {
	if pattern == "" {
		return fmt.Errorf("empty pattern")
	}
	if pattern[0] == '/' {
		return fmt.Errorf("pattern %q: leading slash", short(pattern))
	}
	segs := strings.Split(pattern, "/")
	if len(segs) > MaxPatternSegments {
		return fmt.Errorf("pattern %q: more than %d segments", short(pattern), MaxPatternSegments)
	}
	for _, seg := range segs {
		if seg == "" || seg == ".." {
			return fmt.Errorf("pattern %q: empty or %q segment", short(pattern), "..")
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
