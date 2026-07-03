// Glob matching for capability target patterns.
//
// Capability targets use a restricted glob syntax designed for security:
//
//  1. Bare "*" — matches any target string. Used for broad access declarations
//     like "read any file" (use sparingly; prefer specific paths).
//  2. Pattern with "*" — the star matches any sequence of characters within
//     a single path segment (does not cross "/" boundaries). This prevents
//     a pattern like "grammars/*.tokens" from accidentally matching
//     "grammars/evil/../../etc/passwd".
//  3. Literal — an exact string match with no wildcards.
//
// Path normalization is applied before matching to prevent traversal attacks:
// both the declared target and the actual path are cleaned via path.Clean
// (slash-based, not OS-specific) before comparison. This means a caller
// cannot bypass a restriction by passing "../../../grammars/verilog.tokens"
// when the manifest declares "grammars/verilog.tokens".
package capabilitycage

import (
	"path"
	"strings"
)

// matchTarget reports whether the declared target pattern covers the
// requested access target.
//
// Rules (applied in order):
//  1. Bare "*" matches everything.
//  2. Both strings are path-normalized (path.Clean) before further checks.
//  3. If the pattern contains no "*", require exact equality.
//  4. If the pattern contains "*", use glob matching where "*" matches any
//     sequence of non-"/" characters.
func matchTarget(pattern, target string) bool {
	// Rule 1: bare wildcard matches anything.
	if pattern == "*" {
		return true
	}

	// Rule 2: normalize paths to prevent traversal tricks.
	// path.Clean handles "//", ".", "..", etc. in a platform-neutral way.
	normalizedPattern := path.Clean(pattern)
	normalizedTarget := path.Clean(target)

	// Rule 3: no wildcards — require exact equality.
	if !strings.Contains(normalizedPattern, "*") {
		return normalizedPattern == normalizedTarget
	}

	// Rule 4: glob matching.
	return matchGlob(normalizedPattern, normalizedTarget)
}

// matchGlob performs recursive glob matching where "*" matches any sequence
// of characters that does not include "/".
//
// This is a character-level recursive implementation that handles any number
// of "*" wildcards in the pattern, with the invariant that no star can span
// a "/" directory separator.
//
// Examples:
//   - "*.tokens" matches "verilog.tokens" but not "sub/verilog.tokens"
//   - "grammars/*" matches "grammars/verilog.tokens" but not "grammars/sub/x"
//   - "pre*mid*suf" matches "preXmidYsuf" but not "preX/Ymidsuf"
func matchGlob(pattern, target string) bool {
	// Base case: empty pattern matches only empty target.
	if pattern == "" {
		return target == ""
	}

	// If pattern is just "*", match any non-empty or empty target that
	// contains no "/".
	if pattern == "*" {
		return !strings.Contains(target, "/")
	}

	// Pattern starts with a literal character — must match exactly.
	if pattern[0] != '*' {
		if target == "" {
			return false
		}
		if pattern[0] != target[0] {
			return false
		}
		return matchGlob(pattern[1:], target[1:])
	}

	// Pattern starts with "*". The star matches zero or more non-"/" chars.
	rest := pattern[1:] // Pattern after the star.

	// Try matching the star against zero characters (skip the star).
	if matchGlob(rest, target) {
		return true
	}

	// Try matching the star against one more non-"/" character.
	if target == "" || target[0] == '/' {
		return false
	}
	return matchGlob(pattern, target[1:])
}
