"""
glob_match.py -- Pure String-Based Glob Pattern Matching
=========================================================

This module provides a single function, ``match_path()``, that tests whether
a file path matches a glob pattern. It performs **no filesystem access** --
everything is pure string manipulation. This makes it safe to use in contexts
where the files don't exist yet (e.g., validating declared source patterns
against changed file lists from ``git diff``).

Supported glob syntax
---------------------

+---------+-------------------------------------------------------+
| Pattern | Meaning                                               |
+---------+-------------------------------------------------------+
| ``*``   | Matches any sequence of characters within a single    |
|         | path segment (never matches ``/``).                   |
+---------+-------------------------------------------------------+
| ``?``   | Matches exactly one character within a segment        |
|         | (never matches ``/``).                                |
+---------+-------------------------------------------------------+
| ``[…]`` | Matches one character from a set (e.g., ``[abc]``,    |
|         | ``[!abc]`` for negation, ``[0-9]`` for ranges).      |
+---------+-------------------------------------------------------+
| ``**``  | Matches zero or more path segments. Must appear as    |
|         | an entire segment on its own (e.g., ``src/**/*.py``). |
+---------+-------------------------------------------------------+

Algorithm
---------

The algorithm splits both pattern and path on ``/`` into lists of segments.
Then it walks the two lists with two pointers (``pi`` for pattern, ``si`` for
path segments), using the following rules:

1. If the pattern segment is ``**``, it can consume zero or more path segments.
   We try matching the rest of the pattern starting from the current path
   position (zero segments consumed), then from the next position (one segment
   consumed), and so on.

2. For non-``**`` segments, we use ``fnmatch.fnmatchcase()`` to compare the
   pattern segment against the path segment. ``fnmatchcase`` handles ``*``,
   ``?``, and ``[…]`` within a single segment.

3. The match succeeds only when both pointers reach the end of their respective
   lists simultaneously. Trailing ``**`` segments are handled correctly because
   ``**`` can match zero segments.

Why not use ``pathlib.PurePath.match()``?
-----------------------------------------

Python's built-in ``PurePath.match()`` does not support ``**`` for recursive
matching in a cross-platform way (its behavior varies between Python versions
and has bugs with leading ``**``). By splitting on ``/`` and recursing
ourselves, we get deterministic, platform-independent behavior.

Why not use ``fnmatch.fnmatch()`` directly?
--------------------------------------------

``fnmatch.fnmatch()`` treats ``*`` as matching everything including ``/``.
That means ``*.py`` would match ``src/foo.py``, which is wrong for file path
matching where ``*`` should only match within a single directory. By splitting
into segments first, we confine ``*`` to single-segment matching.

We use ``fnmatchcase`` (case-sensitive) rather than ``fnmatch`` (which may be
case-insensitive on some platforms) for deterministic cross-platform behavior.
BUILD systems should be case-sensitive even on macOS/Windows.
"""

from __future__ import annotations

import fnmatch


def match_path(pattern: str, path: str) -> bool:
    """Test whether a file path matches a glob pattern.

    Both ``pattern`` and ``path`` use forward slashes (``/``) as separators.
    No filesystem access is performed -- this is pure string matching.

    Parameters
    ----------
    pattern : str
        The glob pattern. Supports ``*``, ``?``, ``[…]``, and ``**``.
    path : str
        The file path to test against the pattern.

    Returns
    -------
    bool
        True if the path matches the pattern, False otherwise.

    Examples
    --------
    >>> match_path("src/**/*.py", "src/foo/bar.py")
    True
    >>> match_path("src/*.py", "src/foo/bar.py")
    False
    >>> match_path("**/*.py", "deep/nested/file.py")
    True
    >>> match_path("src/**", "src/anything/at/all")
    True
    """
    # Split both pattern and path into segments.
    #
    # For example:
    #   pattern "src/**/*.py" -> ["src", "**", "*.py"]
    #   path    "src/foo/bar.py" -> ["src", "foo", "bar.py"]
    #
    # We filter out empty strings to handle edge cases like leading or
    # trailing slashes and double slashes ("src//foo").
    pat_segments = [s for s in pattern.split("/") if s]
    path_segments = [s for s in path.split("/") if s]

    # Delegate to the recursive matching engine.
    return _match_segments(pat_segments, 0, path_segments, 0)


def _match_segments(
    pat: list[str],
    pi: int,
    path: list[str],
    si: int,
) -> bool:
    """Recursive segment-by-segment matching engine.

    This is the core of the glob matching algorithm. It walks two lists
    of segments (pattern and path) using indices ``pi`` and ``si``.

    Parameters
    ----------
    pat : list[str]
        The pattern segments (e.g., ``["src", "**", "*.py"]``).
    pi : int
        Current index into the pattern segments.
    path : list[str]
        The path segments (e.g., ``["src", "foo", "bar.py"]``).
    si : int
        Current index into the path segments.

    Returns
    -------
    bool
        True if the remaining pattern matches the remaining path.

    The recursion has three cases:

    Case 1: Both lists exhausted (pi == len(pat) and si == len(path)).
        Match succeeds -- we consumed everything.

    Case 2: Pattern exhausted but path has remaining segments.
        Match fails -- there are unmatched path segments.

    Case 3: Current pattern segment is "**".
        Try matching the rest of the pattern against path[si:], path[si+1:],
        path[si+2:], etc. This implements "zero or more segments". We also
        handle consecutive "**" segments by collapsing them (advancing pi
        past all of them).

    Case 4: Current pattern segment is a normal glob (may contain * or ?).
        Use fnmatch.fnmatchcase to match against the current path segment.
        If it matches, advance both pointers.
    """
    # Skip consecutive "**" segments in the pattern. Multiple "**" in a row
    # are equivalent to a single "**", so we collapse them.
    while pi < len(pat) and pat[pi] == "**":
        # Try matching the rest of the pattern (after this **) against
        # every possible suffix of the remaining path segments.
        #
        # When si == len(path), we're trying to match ** against zero
        # remaining segments, which is valid.
        #
        # We advance pi past this "**" and try matching the rest of the
        # pattern starting from the next pattern segment.
        next_pi = pi + 1

        # Collapse consecutive ** segments.
        while next_pi < len(pat) and pat[next_pi] == "**":
            next_pi += 1

        # If ** is the last pattern segment, it matches everything remaining.
        # This is an optimization that avoids looping.
        if next_pi == len(pat):
            return True

        # Try matching the rest of the pattern against path[si:], path[si+1:],
        # etc. The ** consumes 0, 1, 2, ... segments from the path.
        for try_si in range(si, len(path) + 1):
            if _match_segments(pat, next_pi, path, try_si):
                return True

        # No suffix of the path matched the rest of the pattern.
        return False

    # Base case: pattern is exhausted.
    if pi == len(pat):
        # Success only if the path is also exhausted.
        return si == len(path)

    # Path is exhausted but pattern still has non-** segments.
    if si == len(path):
        return False

    # Normal segment matching: use fnmatchcase for *, ?, [abc] support.
    #
    # fnmatchcase("bar.py", "*.py") -> True
    # fnmatchcase("foo", "f??") -> True
    # fnmatchcase("a", "[abc]") -> True
    #
    # fnmatchcase is case-sensitive, which is correct for BUILD systems.
    if fnmatch.fnmatchcase(path[si], pat[pi]):
        return _match_segments(pat, pi + 1, path, si + 1)

    return False
