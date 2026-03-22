defmodule BuildTool.GlobMatch do
  @moduledoc """
  Pure string-based glob pattern matching that correctly handles the `**`
  (double-star / globstar) wildcard.

  ## Why not Path.wildcard?

  Elixir's `Path.wildcard/2` expands patterns against the filesystem and
  returns actual file paths. We need the opposite: match a *given* path
  string against a pattern without touching the filesystem. This arises
  in two situations:

    1. **Git diff filtering** — does a changed file (a path string from
       `git diff --name-only`) match a package's declared srcs? No
       filesystem access, just string matching.
    2. **Hasher filtering** — after walking a package directory, which of
       the discovered files match the declared srcs? We already have the
       file paths; we just need to test each one against the patterns.

  ## Pattern syntax

  This module supports the same glob syntax used by Bazel, Buck, and most
  build systems:

    | Pattern  | Meaning                                              |
    |----------|------------------------------------------------------|
    | `*`      | Matches any sequence of non-`/` characters within a  |
    |          | single path segment. `*.py` matches `foo.py` but     |
    |          | NOT `dir/foo.py`.                                    |
    | `**`     | Matches zero or more complete path segments.          |
    |          | `src/**/*.py` matches `src/foo.py` and also          |
    |          | `src/a/b/c.py`.                                      |
    | `?`      | Matches exactly one non-`/` character.                |

  All matching uses forward slashes (`/`) as the path separator. Callers
  should normalize paths before calling.

  ## Algorithm

  The matcher splits both pattern and path into segments on `/`, then
  walks them in lockstep with a recursive `match_segments/2` function:

    - A `**` segment tries consuming 0, 1, 2, ... path segments.
    - Any other segment must match exactly one path segment using
      `match_segment/2` (which handles `*`, `?`, and literals).
    - Both lists empty → match. Pattern empty but path remains → no match.
    - Path empty but pattern remains → match only if all remaining
      pattern segments are `**`.

  This is a direct port of the Go implementation at
  `code/programs/go/build-tool/internal/globmatch/globmatch.go`.

  ## Examples

      iex> BuildTool.GlobMatch.match_path?("src/**/*.py", "src/foo/bar.py")
      true

      iex> BuildTool.GlobMatch.match_path?("*.py", "dir/foo.py")
      false

      iex> BuildTool.GlobMatch.match_path?("**", "anything/at/all")
      true
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Reports whether a relative file path matches the given glob pattern.

  Both `pattern` and `path` should use forward slashes as separators.
  Trailing slashes are stripped before matching.

  ## Parameters

    - `pattern` — the glob pattern (e.g., `"src/**/*.py"`)
    - `path` — the file path to test (e.g., `"src/foo/bar.py"`)

  ## Returns

    `true` if the path matches the pattern, `false` otherwise.

  ## Examples

      iex> BuildTool.GlobMatch.match_path?("src/**/*.py", "src/foo.py")
      true

      iex> BuildTool.GlobMatch.match_path?("src/**/*.py", "tests/foo.py")
      false

      iex> BuildTool.GlobMatch.match_path?("**", "")
      true
  """
  def match_path?(pattern, path) do
    # Normalize: strip trailing slashes for consistent matching.
    pattern = String.trim_trailing(pattern, "/")
    path = String.trim_trailing(path, "/")

    # Split into segments. Empty strings produce empty lists.
    pattern_parts = split_path(pattern)
    path_parts = split_path(path)

    match_segments(pattern_parts, path_parts)
  end

  # ---------------------------------------------------------------------------
  # Path splitting
  # ---------------------------------------------------------------------------
  #
  # Splits a path string on "/" and filters out empty strings (from leading,
  # trailing, or double slashes). An empty input returns an empty list.
  #
  # Examples:
  #   split_path("")        → []
  #   split_path("a/b/c")   → ["a", "b", "c"]
  #   split_path("/a/b/")   → ["a", "b"]
  #   split_path("a//b")    → ["a", "b"]

  @doc false
  def split_path(""), do: []

  def split_path(p) do
    p
    |> String.split("/")
    |> Enum.filter(&(&1 != ""))
  end

  # ---------------------------------------------------------------------------
  # Recursive segment matching
  # ---------------------------------------------------------------------------
  #
  # This is the heart of the glob matcher. It processes pattern segments
  # and path segments in lockstep, with special handling for "**".
  #
  # The recursion has three base cases:
  #
  #   1. Both lists empty → match (we consumed everything perfectly).
  #   2. Pattern empty, path non-empty → no match (leftover path).
  #   3. Path empty, pattern non-empty → match ONLY if all remaining
  #      pattern segments are "**" (which can match zero segments).
  #
  # The recursive cases:
  #
  #   - Pattern starts with "**": try consuming 0, 1, 2, ... path
  #     segments from the front. Consecutive "**" segments collapse
  #     to a single one (optimization to avoid exponential blowup).
  #
  #   - Pattern starts with anything else: the current pattern segment
  #     must match exactly one path segment via match_segment/2. If it
  #     matches, recurse on the tails. If not, fail.

  defp match_segments([], []), do: true
  defp match_segments([], _path), do: false

  defp match_segments(pattern, []) do
    # Path is empty. Match only if all remaining pattern segments are "**".
    Enum.all?(pattern, &(&1 == "**"))
  end

  defp match_segments(["**" | rest_pattern], path) do
    # Skip consecutive "**" segments — they're equivalent to a single one.
    # This prevents exponential blowup on patterns like "**/**/**/**".
    rest_pattern = Enum.drop_while(rest_pattern, &(&1 == "**"))

    # Try matching the rest of the pattern against path[i..] for every
    # possible i from 0 to length(path). i=0 means ** matches zero
    # segments; i=length(path) means ** matches all remaining segments.
    Enum.any?(0..length(path), fn i ->
      match_segments(rest_pattern, Enum.drop(path, i))
    end)
  end

  defp match_segments([pat_seg | rest_pattern], [path_seg | rest_path]) do
    # Normal segment: must match exactly one path segment.
    if match_segment(pat_seg, path_seg) do
      match_segments(rest_pattern, rest_path)
    else
      false
    end
  end

  # ---------------------------------------------------------------------------
  # Single-segment matching
  # ---------------------------------------------------------------------------
  #
  # Matches a single pattern segment against a single path segment.
  # Supports:
  #   - `*`  — matches any sequence of characters (including empty)
  #   - `?`  — matches exactly one character
  #   - Literal characters — must match exactly
  #
  # This is implemented as a recursive character-by-character walk.
  #
  # Truth table for match_segment:
  #
  #   | Pattern | Path     | Result | Reason                            |
  #   |---------|----------|--------|-----------------------------------|
  #   | ""      | ""       | true   | Both empty                        |
  #   | ""      | "a"      | false  | Pattern exhausted, path remains   |
  #   | "*"     | "foo"    | true   | * matches any sequence            |
  #   | "*"     | ""       | true   | * matches empty too               |
  #   | "?"     | "a"      | true   | ? matches one character           |
  #   | "?"     | ""       | false  | ? needs exactly one character     |
  #   | "?.py"  | "a.py"   | true   | ? + literal                       |
  #   | "*.py"  | "foo.py" | true   | * + literal                       |
  #   | "*.py"  | "foo.rb" | false  | * matches but .rb != .py          |

  defp match_segment(pattern, path) do
    do_match_segment(String.graphemes(pattern), String.graphemes(path))
  end

  # Base cases.
  defp do_match_segment([], []), do: true
  defp do_match_segment([], _), do: false

  # Star: try matching zero or more characters from path.
  defp do_match_segment(["*" | rest_pat], path_chars) do
    # Try consuming 0, 1, 2, ... characters from the path.
    Enum.any?(0..length(path_chars), fn i ->
      do_match_segment(rest_pat, Enum.drop(path_chars, i))
    end)
  end

  # Question mark: match exactly one character.
  defp do_match_segment(["?" | rest_pat], [_char | rest_path]) do
    do_match_segment(rest_pat, rest_path)
  end

  defp do_match_segment(["?" | _rest_pat], []), do: false

  # Literal character: must match exactly.
  defp do_match_segment([c | rest_pat], [c | rest_path]) do
    do_match_segment(rest_pat, rest_path)
  end

  # Mismatch.
  defp do_match_segment([_c | _rest_pat], _path), do: false
end
