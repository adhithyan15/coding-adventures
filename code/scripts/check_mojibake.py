#!/usr/bin/env python3
"""Reject double-encoded UTF-8 ("mojibake") in tracked source.

The corruption happens when a non-ASCII character is read out of a terminal
and copied back into a patch: the terminal renders the character's UTF-8
bytes as if they were cp1252, and re-encoding those glyphs produces a longer
byte sequence that still *looks* like the original character in a rendered
diff.  That invisibility is the whole problem -- 2133 occurrences accumulated
across 11 files before anyone noticed (#14885), and they were not confined to
comments: generated README text, an emitted XAML comment, and two user-facing
error messages carried them into shipped artifacts.

Each pattern below is derived, not hand-typed, so the table cannot drift from
the characters it is meant to protect:

    '-'.encode('utf-8').decode('cp1252').encode('utf-8')

Run with no arguments to scan the repository; pass paths to scan a subset.
"""

from __future__ import annotations

import os
import sys

# The characters that actually appear in this repo's prose and diagrams.
# Extend this list rather than writing byte literals by hand.
PROTECTED = ["—", "→", "§", "…", "─", "’"]

SKIP_DIRS = {"target", "node_modules", ".git", "build", "dist", ".venv", "__pycache__"}
SCAN_SUFFIXES = (
    ".rs", ".kt", ".swift", ".dart", ".ts", ".tsx", ".js", ".py", ".rb",
    ".go", ".java", ".cs", ".c", ".h", ".cpp", ".md", ".toml", ".json",
    ".yml", ".yaml", ".sh",
)


def patterns() -> dict[bytes, str]:
    """Map each corrupt byte sequence to the character it should be."""
    table: dict[bytes, str] = {}
    for ch in PROTECTED:
        try:
            corrupt = ch.encode("utf-8").decode("cp1252").encode("utf-8")
        except UnicodeDecodeError:
            # Not every character's UTF-8 bytes are valid cp1252; those cannot
            # be corrupted this way, so there is nothing to guard.
            continue
        table[corrupt] = ch
    return table


def scan(roots: list[str]) -> list[tuple[str, int, bytes]]:
    table = patterns()
    findings: list[tuple[str, int, bytes]] = []
    for root in roots:
        if os.path.isfile(root):
            walk = [(os.path.dirname(root), [], [os.path.basename(root)])]
        else:
            walk = os.walk(root)
        for dirpath, dirnames, filenames in walk:
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
            for name in filenames:
                if not name.endswith(SCAN_SUFFIXES):
                    continue
                path = os.path.join(dirpath, name)
                try:
                    blob = open(path, "rb").read()
                except OSError:
                    continue
                for corrupt in table:
                    count = blob.count(corrupt)
                    if count:
                        findings.append((path, count, corrupt))
    return findings


def main() -> int:
    roots = sys.argv[1:] or ["code"]
    findings = scan(roots)
    if not findings:
        return 0
    table = patterns()
    total = sum(count for _, count, _ in findings)
    print(f"error: {total} double-encoded UTF-8 sequence(s) in {len({f[0] for f in findings})} file(s):\n")
    for path, count, corrupt in sorted(findings):
        want = table[corrupt]
        print(f"  {path}: {count} x {corrupt.hex(' ')} (should be {want!r} = {want.encode('utf-8').hex(' ')})")
    print(
        "\nThis happens when a non-ASCII character is copied out of terminal output.\n"
        "Type the character directly, or write it as a \\uXXXX escape in whatever\n"
        "generates the patch."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
