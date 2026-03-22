"""grep -- print lines that match patterns.

=== What This Program Does ===

This is a reimplementation of the GNU ``grep`` utility. It searches
input files for lines matching a regular expression pattern, and
prints the matching lines.

=== Pattern Matching Modes ===

grep supports several pattern interpretation modes:

+-------+----------------------------------------------+
| Flag  | Mode                                         |
+=======+==============================================+
| ``-G``| Basic regular expressions (default)           |
| ``-E``| Extended regular expressions                  |
| ``-F``| Fixed strings (no regex, literal match)       |
| ``-P``| Perl-compatible regular expressions           |
+-------+----------------------------------------------+

In our implementation, Python's ``re`` module handles all regex modes.
The difference between basic and extended regex in GNU grep involves
escaping rules for ``()``, ``{}``, etc. — we simplify by treating both
the same way (Python regex is already "extended").

=== Match Modifiers ===

+-------+----------------------------------------------+
| Flag  | Effect                                       |
+=======+==============================================+
| ``-i``| Case-insensitive matching                    |
| ``-v``| Invert match — select non-matching lines     |
| ``-w``| Match whole words only                       |
| ``-x``| Match whole lines only                       |
+-------+----------------------------------------------+

=== Output Modes ===

+-------+----------------------------------------------+
| Flag  | Output                                       |
+=======+==============================================+
| (none)| Print matching lines                         |
| ``-c``| Print count of matching lines per file       |
| ``-l``| Print names of files with matches            |
| ``-L``| Print names of files without matches         |
| ``-o``| Print only the matched parts                 |
| ``-q``| No output — exit status indicates match      |
+-------+----------------------------------------------+

=== Context Lines ===

grep can show surrounding context for each match:

- ``-A N``: Show N lines *after* each match.
- ``-B N``: Show N lines *before* each match.
- ``-C N``: Show N lines before *and* after (same as ``-A N -B N``).

Context groups are separated by ``--`` separator lines.

=== CLI Builder Integration ===

The entire CLI is defined in ``grep.json``.
"""

from __future__ import annotations

import os
import re
import sys
from fnmatch import fnmatch
from pathlib import Path
from typing import TextIO

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "grep.json")


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------


class GrepOptions:
    """Container for grep option flags.

    Bundling all options into a class keeps function signatures clean
    and makes it easy to pass options through multiple layers.
    """

    def __init__(self, **kwargs: int | bool | str | None) -> None:
        self.ignore_case: bool = bool(kwargs.get("ignore_case", False))
        self.invert_match: bool = bool(kwargs.get("invert_match", False))
        self.word_regexp: bool = bool(kwargs.get("word_regexp", False))
        self.line_regexp: bool = bool(kwargs.get("line_regexp", False))
        self.fixed_strings: bool = bool(kwargs.get("fixed_strings", False))
        self.count: bool = bool(kwargs.get("count", False))
        self.files_with_matches: bool = bool(kwargs.get("files_with_matches", False))
        self.files_without_match: bool = bool(kwargs.get("files_without_match", False))
        self.only_matching: bool = bool(kwargs.get("only_matching", False))
        self.quiet: bool = bool(kwargs.get("quiet", False))
        self.line_number: bool = bool(kwargs.get("line_number", False))
        self.with_filename: bool = bool(kwargs.get("with_filename", False))
        self.no_filename: bool = bool(kwargs.get("no_filename", False))
        self.max_count: int | None = kwargs.get("max_count", None)  # type: ignore[assignment]
        self.after_context: int = int(kwargs.get("after_context", 0) or 0)
        self.before_context: int = int(kwargs.get("before_context", 0) or 0)
        self.context: int = int(kwargs.get("context", 0) or 0)


# ---------------------------------------------------------------------------
# Business logic — compile pattern
# ---------------------------------------------------------------------------


def compile_pattern(
    pattern: str,
    *,
    ignore_case: bool = False,
    fixed_strings: bool = False,
    word_regexp: bool = False,
    line_regexp: bool = False,
) -> re.Pattern[str]:
    """Compile a search pattern into a regex object.

    This function handles all the pattern transformation modes:

    1. Fixed strings: escape the pattern so it's treated literally.
    2. Word regexp: wrap the pattern with ``\\b`` word boundaries.
    3. Line regexp: anchor the pattern with ``^`` and ``$``.

    Args:
        pattern: The raw pattern string.
        ignore_case: If True, compile with re.IGNORECASE.
        fixed_strings: If True, treat pattern as literal text.
        word_regexp: If True, match whole words only.
        line_regexp: If True, match whole lines only.

    Returns:
        A compiled regex pattern.
    """
    if fixed_strings:
        pattern = re.escape(pattern)

    if word_regexp:
        pattern = r"\b" + pattern + r"\b"
    elif line_regexp:
        pattern = "^" + pattern + "$"

    flags = 0
    if ignore_case:
        flags |= re.IGNORECASE

    return re.compile(pattern, flags)


# ---------------------------------------------------------------------------
# Business logic — line matching
# ---------------------------------------------------------------------------


def grep_line(line: str, compiled: re.Pattern[str], opts: GrepOptions) -> bool:
    """Test whether a single line matches the grep pattern.

    This is the core matching function. It applies the compiled regex
    to the line and respects the invert-match flag.

    Args:
        line: The line to test (without trailing newline).
        compiled: The compiled regex pattern.
        opts: Grep options (only ``invert_match`` is used here).

    Returns:
        True if the line should be selected, False otherwise.

    Truth table for invert_match
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    +-------+---------------+---------+
    | Match | invert_match  | Result  |
    +=======+===============+=========+
    | Yes   | False         | True    |
    | Yes   | True          | False   |
    | No    | False         | False   |
    | No    | True          | True    |
    +-------+---------------+---------+
    """
    match = compiled.search(line)
    if opts.invert_match:
        return match is None
    return match is not None


# ---------------------------------------------------------------------------
# Business logic — file searching
# ---------------------------------------------------------------------------


def grep_file(
    lines: list[str],
    compiled: re.Pattern[str],
    opts: GrepOptions,
    *,
    filename: str | None = None,
) -> list[str]:
    """Search through lines and return formatted output.

    This function handles all output modes: normal, count, only-matching,
    and context lines. It does NOT handle files-with-matches or
    files-without-match — those are handled at the caller level.

    Args:
        lines: The lines to search (without trailing newlines).
        compiled: The compiled regex pattern.
        opts: Grep options controlling output format.
        filename: If set, prefix output lines with this filename.

    Returns:
        A list of formatted output lines.

    How context lines work
    ~~~~~~~~~~~~~~~~~~~~~~~
    Before-context and after-context require us to track which lines
    have been printed. We use a set to avoid duplicating lines that
    appear in overlapping context windows.

    ::

        Line 1
        Line 2        <- before context (B=1)
        Line 3 MATCH  <- matching line
        Line 4        <- after context (A=1)
        Line 5
        --            <- separator between context groups
        Line 6
        Line 7 MATCH
        Line 8
    """
    output: list[str] = []
    match_count = 0

    # Determine context window sizes.
    before = max(opts.before_context, opts.context)
    after = max(opts.after_context, opts.context)
    use_context = before > 0 or after > 0

    # For context mode, track which lines have been output and the
    # last line index that was printed.
    printed: set[int] = set()
    last_printed_idx = -2  # Sentinel: no line printed yet.

    # Determine filename prefix.
    show_filename = opts.with_filename and not opts.no_filename
    prefix = f"{filename}:" if show_filename and filename else ""
    sep_prefix = f"{filename}-" if show_filename and filename else ""

    # --- Count mode ---
    if opts.count:
        for line in lines:
            if grep_line(line, compiled, opts):
                match_count += 1
                if opts.max_count is not None and match_count >= opts.max_count:
                    break
        return [f"{prefix}{match_count}"]

    # --- Normal / context / only-matching mode ---
    for i, line in enumerate(lines):
        if grep_line(line, compiled, opts):
            match_count += 1

            if opts.max_count is not None and match_count > opts.max_count:
                break

            if use_context:
                # Print separator between non-contiguous context groups.
                context_start = max(0, i - before)
                if last_printed_idx >= 0 and context_start > last_printed_idx + 1:
                    output.append("--")

                # Print before-context lines.
                for j in range(context_start, i):
                    if j not in printed:
                        line_num = f"{j + 1}-" if opts.line_number else ""
                        output.append(f"{sep_prefix}{line_num}{lines[j]}")
                        printed.add(j)
                        last_printed_idx = j

            # Print the matching line itself.
            line_num_str = f"{i + 1}:" if opts.line_number else ""

            if opts.only_matching and not opts.invert_match:
                # Print only the matched portions.
                for m in compiled.finditer(line):
                    output.append(f"{prefix}{line_num_str}{m.group()}")
            else:
                output.append(f"{prefix}{line_num_str}{line}")

            printed.add(i)
            last_printed_idx = i

            if use_context:
                # Print after-context lines.
                for j in range(i + 1, min(len(lines), i + 1 + after)):
                    if j not in printed:
                        line_num_a = f"{j + 1}-" if opts.line_number else ""
                        output.append(f"{sep_prefix}{line_num_a}{lines[j]}")
                        printed.add(j)
                        last_printed_idx = j

    return output


def grep_stream(
    stream: TextIO,
    compiled: re.Pattern[str],
    opts: GrepOptions,
    *,
    filename: str | None = None,
) -> list[str]:
    """Read lines from a stream and grep them.

    This is a convenience wrapper that reads all lines, strips trailing
    newlines, and delegates to ``grep_file``.

    Args:
        stream: An open text stream to read from.
        compiled: The compiled regex pattern.
        opts: Grep options.
        filename: Optional filename for output prefixing.

    Returns:
        Formatted output lines.
    """
    lines = [line.rstrip("\n").rstrip("\r") for line in stream.readlines()]
    return grep_file(lines, compiled, opts, filename=filename)


# ---------------------------------------------------------------------------
# Business logic — recursive directory walking
# ---------------------------------------------------------------------------


def walk_files(
    paths: list[str],
    *,
    recursive: bool = False,
    follow_symlinks: bool = False,
    include_globs: list[str] | None = None,
    exclude_globs: list[str] | None = None,
    exclude_dir_globs: list[str] | None = None,
) -> list[str]:
    """Expand paths, handling recursive directory traversal.

    Args:
        paths: List of file/directory paths.
        recursive: If True, recurse into directories.
        follow_symlinks: If True, follow symlinks during recursion.
        include_globs: Only include files matching these globs.
        exclude_globs: Exclude files matching these globs.
        exclude_dir_globs: Exclude directories matching these globs.

    Returns:
        A flat list of file paths to search.
    """
    result: list[str] = []

    for path in paths:
        if os.path.isdir(path):
            if not recursive:
                print(f"grep: {path}: Is a directory", file=sys.stderr)
                continue
            for root, dirs, files in os.walk(path, followlinks=follow_symlinks):
                # Filter directories.
                if exclude_dir_globs:
                    dirs[:] = [
                        d for d in dirs
                        if not any(fnmatch(d, g) for g in exclude_dir_globs)
                    ]

                for f in sorted(files):
                    filepath = os.path.join(root, f)
                    if include_globs and not any(fnmatch(f, g) for g in include_globs):
                        continue
                    if exclude_globs and any(fnmatch(f, g) for g in exclude_globs):
                        continue
                    result.append(filepath)
        else:
            result.append(path)

    return result


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then grep files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"grep: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Extract flags and pattern ---------------------------------
    assert isinstance(result, ParseResult)

    # Build options.
    opts = GrepOptions(
        ignore_case=result.flags.get("ignore_case", False),
        invert_match=result.flags.get("invert_match", False),
        word_regexp=result.flags.get("word_regexp", False),
        line_regexp=result.flags.get("line_regexp", False),
        fixed_strings=result.flags.get("fixed_strings", False),
        count=result.flags.get("count", False),
        files_with_matches=result.flags.get("files_with_matches", False),
        files_without_match=result.flags.get("files_without_match", False),
        only_matching=result.flags.get("only_matching", False),
        quiet=result.flags.get("quiet", False),
        line_number=result.flags.get("line_number", False),
        with_filename=result.flags.get("with_filename", False),
        no_filename=result.flags.get("no_filename", False),
        max_count=result.flags.get("max_count", None),
        after_context=result.flags.get("after_context", 0),
        before_context=result.flags.get("before_context", 0),
        context=result.flags.get("context", 0),
    )

    # Get patterns — from -e flags, -f files, or positional argument.
    patterns: list[str] = []
    regexp_flags = result.flags.get("regexp", [])
    if isinstance(regexp_flags, list):
        patterns.extend(regexp_flags)
    elif regexp_flags:
        patterns.append(regexp_flags)

    file_flags = result.flags.get("file", [])
    if isinstance(file_flags, list):
        for fp in file_flags:
            with open(fp) as f:
                patterns.extend(line.rstrip("\n") for line in f)
    elif file_flags:
        with open(file_flags) as f:
            patterns.extend(line.rstrip("\n") for line in f)

    if not patterns:
        positional_pattern = result.arguments.get("pattern", "")
        if positional_pattern:
            patterns.append(positional_pattern)

    if not patterns:
        print("grep: no pattern specified", file=sys.stderr)
        raise SystemExit(2)

    # Combine multiple patterns with alternation.
    combined_pattern = "|".join(patterns)

    # Compile the pattern.
    compiled = compile_pattern(
        combined_pattern,
        ignore_case=opts.ignore_case,
        fixed_strings=opts.fixed_strings,
        word_regexp=opts.word_regexp,
        line_regexp=opts.line_regexp,
    )

    # --- Step 4: Determine input files -------------------------------------
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    recursive = result.flags.get("recursive", False) or result.flags.get(
        "dereference_recursive", False
    )
    follow_symlinks = result.flags.get("dereference_recursive", False)

    if files:
        file_list = walk_files(
            files,
            recursive=recursive,
            follow_symlinks=follow_symlinks,
            include_globs=result.flags.get("include", None),
            exclude_globs=result.flags.get("exclude", None),
            exclude_dir_globs=result.flags.get("exclude_dir", None),
        )
    else:
        file_list = []  # Read from stdin.

    # Auto-enable filename display for multiple files.
    if len(file_list) > 1 and not opts.no_filename:
        opts.with_filename = True

    # --- Step 5: Search each file ------------------------------------------
    found_match = False

    if not file_list:
        # Read from stdin.
        output = grep_stream(sys.stdin, compiled, opts, filename="(standard input)")
        if output:
            found_match = True
            if not opts.quiet:
                for line in output:
                    print(line)
    else:
        for filepath in file_list:
            try:
                with open(filepath, errors="replace") as f:
                    lines = [line.rstrip("\n").rstrip("\r") for line in f.readlines()]
            except OSError as e:
                print(f"grep: {filepath}: {e.strerror}", file=sys.stderr)
                continue

            # Handle files-with-matches / files-without-match modes.
            has_match = any(grep_line(line, compiled, opts) for line in lines)

            if opts.files_with_matches:
                if has_match:
                    found_match = True
                    if not opts.quiet:
                        print(filepath)
                continue

            if opts.files_without_match:
                if not has_match:
                    if not opts.quiet:
                        print(filepath)
                continue

            output = grep_file(lines, compiled, opts, filename=filepath)
            if output:
                found_match = True
                if not opts.quiet:
                    for line in output:
                        print(line)

    # Exit code: 0 = match found, 1 = no match, 2 = error.
    raise SystemExit(0 if found_match else 1)


if __name__ == "__main__":
    main()
