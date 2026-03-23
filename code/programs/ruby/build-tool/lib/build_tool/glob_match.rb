# frozen_string_literal: true

# glob_match.rb -- Pure String-Based Glob Pattern Matching
# ========================================================
#
# This module provides glob pattern matching that correctly handles the
# ** (double-star / globstar) wildcard. It works on path strings without
# touching the filesystem.
#
# Why not File.fnmatch?
# ---------------------
#
# Ruby's File.fnmatch with FNM_PATHNAME does NOT correctly handle **
# as a standalone pattern or at the end of a path. For example:
#
#     File.fnmatch("**", "a/b/c", File::FNM_PATHNAME)  # => false (wrong!)
#     File.fnmatch("src/**", "src", File::FNM_PATHNAME) # => false (wrong!)
#
# The ** wildcard should match zero or more complete path segments, but
# FNM_PATHNAME treats each * as matching only within a single segment.
# The FNM_EXTGLOB flag helps in some cases (src/**/*.py) but fails for
# standalone ** patterns.
#
# To match the behavior of the Go build tool's globmatch package, we
# implement a custom recursive segment-based matcher. This gives us
# consistent behavior across all three build tool implementations
# (Go, Python, Ruby).
#
# Pattern syntax
# --------------
#
# This module supports the same glob syntax as most build systems:
#
#   - *    matches any sequence of non-separator characters within a
#          single path segment. "*.py" matches "foo.py" but not
#          "dir/foo.py".
#   - **   matches zero or more complete path segments. "src/**/*.py"
#          matches "src/foo.py", "src/a/b/c.py", etc.
#   - ?    matches exactly one non-separator character.
#   - [ab] character classes, as supported by File.fnmatch.
#
# All matching uses forward slashes as the path separator. Callers
# should normalize paths before calling.
#
# Algorithm
# ---------
#
# The matcher splits both pattern and path on "/" into segments, then
# recursively matches them:
#
#   1. Split pattern and path into arrays of segments.
#   2. Walk the two arrays in lockstep.
#   3. For a "**" pattern segment, try consuming 0, 1, 2, ... path
#      segments (backtracking search). Consecutive ** segments are
#      collapsed into one for efficiency.
#   4. For any other pattern segment, use File.fnmatch to match it
#      against exactly one path segment.
#   5. Both arrays exhausted simultaneously => match.
#      Path exhausted but pattern remains => match only if all
#      remaining pattern segments are "**".
#      Pattern exhausted but path remains => no match.
#
# This is a direct Ruby port of the Go build tool's
# internal/globmatch/globmatch.go. The recursive structure and edge
# case handling are identical.

module BuildTool
  module GlobMatch
    module_function

    # match_path? -- Check whether a relative file path matches a glob pattern.
    #
    # Both pattern and path should use forward slashes as separators.
    # Trailing slashes are stripped before matching.
    #
    # Examples:
    #
    #     match_path?("src/**/*.py", "src/foo/bar.py")   # => true
    #     match_path?("src/**/*.py", "src/bar.py")        # => true
    #     match_path?("*.toml", "pyproject.toml")         # => true
    #     match_path?("src/**/*.py", "README.md")         # => false
    #     match_path?("**/*.py", "a/b/c.py")             # => true
    #     match_path?("**", "anything/at/all")            # => true
    #
    # Truth table for common Starlark BUILD patterns:
    #
    #     Pattern              | Path                  | Result
    #     ---------------------|-----------------------|--------
    #     "src/**/*.py"        | "src/main.py"         | true
    #     "src/**/*.py"        | "src/a/b.py"          | true
    #     "src/**/*.py"        | "tests/test.py"       | false
    #     "**/*.py"            | "foo.py"              | true
    #     "**"                 | "a/b/c"               | true
    #     "**"                 | ""                    | true
    #     "*.py"               | "dir/foo.py"          | false
    #     "pyproject.toml"     | "pyproject.toml"      | true
    #     "src/**"             | "src/foo.py"          | true
    #     "src/**"             | "src"                 | true
    #
    # @param pattern [String] The glob pattern to match against.
    # @param path [String] The file path to test.
    # @return [Boolean] True if the path matches the pattern.
    def match_path?(pattern, path)
      # Normalize: strip trailing slashes for consistent matching.
      # "src/" and "src" should match the same paths.
      pattern = pattern.chomp("/")
      path = path.chomp("/")

      # Split into segments on forward slash.
      pattern_parts = split_path(pattern)
      path_parts = split_path(path)

      match_segments(pattern_parts, path_parts)
    end

    # split_path -- Split a path string on "/" into non-empty segments.
    #
    # Handles edge cases:
    #   - Empty string => empty array
    #   - Leading/trailing slashes => ignored (no empty segments)
    #   - Double slashes => collapsed (no empty segments)
    #
    # Examples:
    #   split_path("")        => []
    #   split_path("a/b/c")  => ["a", "b", "c"]
    #   split_path("/a/b/")  => ["a", "b"]
    #   split_path("a//b")   => ["a", "b"]
    #
    # @param str [String] The path to split.
    # @return [Array<String>] The path segments.
    def split_path(str)
      return [] if str.empty?

      str.split("/").reject(&:empty?)
    end

    # match_segments -- Recursive core of the glob matcher.
    #
    # Walks pattern segments and path segments in lockstep. This is a
    # direct port of the Go implementation's matchSegments function.
    #
    # The recursion has three base cases:
    #
    #   1. Both arrays empty => match (everything consumed successfully).
    #   2. Pattern empty, path non-empty => no match (leftover path).
    #   3. Path empty, pattern non-empty => match only if ALL remaining
    #      pattern segments are "**" (which can match zero segments).
    #
    # For the recursive step:
    #
    #   - "**" segment: try consuming 0, 1, 2, ... path segments.
    #     First, skip any consecutive "**" segments (they're redundant).
    #     Then try matching the rest of the pattern against each suffix
    #     of the path array: path[0:], path[1:], path[2:], etc.
    #
    #   - Normal segment: use File.fnmatch (without FNM_PATHNAME, since
    #     we're matching individual segments) to compare against exactly
    #     one path segment. If it matches, recurse with both arrays
    #     shifted by one.
    #
    # @param pattern [Array<String>] Remaining pattern segments.
    # @param path [Array<String>] Remaining path segments.
    # @return [Boolean]
    def match_segments(pattern, path)
      # Base case: pattern fully consumed.
      return path.empty? if pattern.empty?

      # Path empty: remaining pattern must be all "**".
      if path.empty?
        return pattern.all? { |seg| seg == "**" }
      end

      seg = pattern[0]

      if seg == "**"
        # ** matches zero or more complete path segments.
        #
        # Optimization: skip consecutive ** segments. Three consecutive
        # "**" segments are equivalent to one — they all match "zero or
        # more segments". Collapsing them reduces recursion depth.
        rest_pattern = pattern[1..]
        rest_pattern = rest_pattern[1..] while !rest_pattern.empty? && rest_pattern[0] == "**"

        # Try matching rest_pattern against path[i..] for every
        # possible i from 0 to path.length. When i=0, ** matches zero
        # segments; when i=path.length, ** consumed everything.
        (0..path.length).each do |i|
          return true if match_segments(rest_pattern, path[i..])
        end

        return false
      end

      # Normal segment: must match exactly one path segment.
      #
      # We use File.fnmatch WITHOUT FNM_PATHNAME because we're matching
      # a single segment (no "/" characters). This handles *, ?, and
      # character classes like [abc].
      return false unless File.fnmatch(seg, path[0])

      match_segments(pattern[1..], path[1..])
    end
  end
end
