// Package globmatch provides pure string-based glob pattern matching
// that correctly handles the ** (double-star / globstar) wildcard.
//
// # Why not filepath.Glob?
//
// Go's filepath.Glob expands patterns against the filesystem and does
// NOT support ** for recursive directory matching. The pattern
// "src/**/*.py" would look for a literal directory named "**" — which
// never matches anything useful.
//
// The build tool needs to match file paths against declared source
// patterns (e.g., "src/**/*.py") without touching the filesystem.
// This is used in two places:
//
//  1. Git diff filtering: does a changed file match a package's
//     declared srcs? (No filesystem access — just path strings.)
//  2. Hasher: which files in a package directory match the declared
//     srcs? (Filesystem walk + pattern matching.)
//
// # Pattern syntax
//
// This package supports the same glob syntax as most build systems:
//
//   - *    matches any sequence of non-separator characters within
//     a single path segment. "*.py" matches "foo.py" but not
//     "dir/foo.py".
//   - **   matches zero or more complete path segments. "src/**/*.py"
//     matches "src/foo.py", "src/a/b/c.py", etc.
//   - ?    matches exactly one non-separator character.
//   - [ab] character classes, as supported by filepath.Match.
//
// All matching uses forward slashes as the path separator. Callers
// should normalize paths with filepath.ToSlash before calling.
package globmatch

import (
	"path/filepath"
	"strings"
)

// MatchPath reports whether a relative file path matches the given glob
// pattern. Both pattern and path should use forward slashes as separators.
//
// Examples:
//
//	MatchPath("src/**/*.py", "src/foo/bar.py")     → true
//	MatchPath("src/**/*.py", "src/bar.py")          → true
//	MatchPath("*.toml", "pyproject.toml")           → true
//	MatchPath("src/**/*.py", "README.md")           → false
//	MatchPath("**/*.py", "a/b/c.py")               → true
//	MatchPath("**", "anything/at/all")              → true
func MatchPath(pattern, path string) bool {
	// Normalize: ensure consistent forward slashes, no trailing slashes.
	pattern = strings.TrimRight(pattern, "/")
	path = strings.TrimRight(path, "/")

	// Split into segments on forward slash.
	patternParts := splitPath(pattern)
	pathParts := splitPath(path)

	return matchSegments(patternParts, pathParts)
}

// splitPath splits a path on "/" and filters out empty strings.
// An empty input returns an empty slice.
func splitPath(p string) []string {
	if p == "" {
		return nil
	}
	parts := strings.Split(p, "/")
	// Filter empty parts (from leading/trailing/double slashes).
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		if part != "" {
			result = append(result, part)
		}
	}
	return result
}

// matchSegments is the recursive core of the glob matcher.
//
// It walks the pattern segments and path segments in lockstep:
//
//   - A "**" pattern segment can match zero or more path segments.
//     We try consuming 0, 1, 2, … path segments until we find a match.
//   - Any other pattern segment must match exactly one path segment
//     using filepath.Match (which handles *, ?, and character classes).
//
// The recursion terminates when one or both slices are exhausted:
//
//   - Both empty → match (pattern fully consumed, path fully consumed).
//   - Pattern empty, path non-empty → no match (leftover path segments).
//   - Path empty, pattern non-empty → match only if all remaining
//     pattern segments are "**" (which can match zero segments).
func matchSegments(pattern, path []string) bool {
	matched, _ := matchSegmentsWithVisitCount(pattern, path)
	return matched
}

func matchSegmentsWithVisitCount(pattern, path []string) (bool, int) {
	type state struct {
		patternIndex int
		pathIndex    int
	}
	memo := make(map[state]bool)
	seen := make(map[state]bool)
	visited := 0

	var match func(int, int) bool
	match = func(patternIndex, pathIndex int) bool {
		key := state{patternIndex: patternIndex, pathIndex: pathIndex}
		if seen[key] {
			return memo[key]
		}
		seen[key] = true
		visited++

		var result bool
		switch {
		case patternIndex == len(pattern):
			result = pathIndex == len(path)
		case pattern[patternIndex] == "**":
			result = match(patternIndex+1, pathIndex) ||
				(pathIndex < len(path) && match(patternIndex, pathIndex+1))
		case pathIndex < len(path):
			segmentMatches, err := filepath.Match(pattern[patternIndex], path[pathIndex])
			result = err == nil && segmentMatches && match(patternIndex+1, pathIndex+1)
		}

		memo[key] = result
		return result
	}

	return match(0, 0), visited
}
