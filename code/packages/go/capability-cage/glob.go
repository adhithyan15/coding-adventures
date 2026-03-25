// Target matching for capability declarations.
//
// Capability targets use exact matching — no wildcards, no globs.
// This is intentional: wildcards become attack vectors because an attacker who
// can influence what path gets passed to a wrapper function can use any pattern
// broad enough to cover a sensitive file.
//
// The only exception is the bare "*" token, which is reserved for capabilities
// that have no meaningful target (e.g. stdin:read, stdout:write, time:read).
// It must never be used for filesystem paths.
//
// Path normalization is applied before comparison to prevent traversal attacks:
// both the declared target and the actual path are slash-normalized (backslash →
// forward slash) and then cleaned via path.Clean before the equality check.
// This means a caller cannot bypass "code/grammars/verilog.tokens" by passing
// "code/grammars/../grammars/verilog.tokens" or a Windows backslash equivalent.
//
// # Stable logical paths vs. absolute OS paths
//
// Absolute OS paths vary per machine ("/home/runner/..." vs "C:\Users\..."),
// so capability targets must use stable, repo-relative logical paths like
// "code/grammars/verilog.tokens". When the actual OS path is absolute, use
// ReadFileAt (or the *At variants of other wrappers), which takes a separate
// declaredPath for the capability check and an osPath for the actual I/O.
package capabilitycage

import (
	"path"
	"strings"
)

// matchTarget reports whether the declared target covers the requested access
// target using exact matching.
//
// Rules:
//  1. Bare "*" matches anything. Reserved for non-path capabilities (stdin,
//     stdout, time). Must not be used for filesystem paths.
//  2. The actual target is slash-normalized (backslashes → forward slashes)
//     so that Windows runtime paths match forward-slash declarations.
//  3. Both strings are cleaned via path.Clean (handles .., .//, etc.).
//  4. Exact equality is required after normalization.
func matchTarget(pattern, target string) bool {
	// Rule 1: bare wildcard matches anything.
	if pattern == "*" {
		return true
	}

	// Rule 2: normalize Windows backslashes in the runtime path.
	normalizedTarget := strings.ReplaceAll(target, "\\", "/")

	// Rule 3: normalize both paths (removes .., ./, // etc.).
	normalizedPattern := path.Clean(pattern)
	normalizedTarget = path.Clean(normalizedTarget)

	// Rule 4: exact match only — no wildcards.
	return normalizedPattern == normalizedTarget
}
