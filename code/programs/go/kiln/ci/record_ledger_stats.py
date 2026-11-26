#!/usr/bin/env python3
"""Count what a Kiln ledger directory holds, for the record workflow's job summary.

Usage: record_ledger_stats.py <ledger-dir>

Prints exactly one line, `records=<N> bytes=<B>`, and exits 0.

WHY THIS EXISTS.  The record workflow (.github/workflows/kiln-record.yml) wants
two numbers in its job summary -- how many ledger entries the restored cache
held before the merge and how many the ledger holds after it -- and one number
for a safety valve (the ledger directory's size, so an ever-growing cache can
be dropped instead of read).  Those are COUNTS.  The summary must never carry
a path, a package name or a reason string, so this script does not print any:
it reads the segment files, counts lines and adds up sizes, and says nothing
else.

WHAT A LEDGER DIRECTORY LOOKS LIKE (internal/ledger/store.go, manifest_store.go):

    <dir>/0000000007-0123456789abcdef.jsonl        pass segments (one record per line)
    <dir>/manifests/0000000007-0123456789abcdef.jsonl   manifest segments

A segment name is a ten-digit sequence, a dash, sixteen lowercase hex digits
and `.jsonl`.  A record is one non-empty line.  This script does NOT decode a
record: whether a segment is well formed is the Go reader's decision, made with
its strict decoder.  Counting lines here is only for the summary, so a bad
segment is still counted and the merge, which reads through the strict decoder,
is what refuses it.

FAIL CLOSED ON ODD FILES.  Anything in the directory that is not a regular,
non-symlink segment file (or the `manifests` directory, itself a real
directory) makes the script exit 1 with a fixed message that names no path.
A restored cache is data from an earlier run; it is never trusted to be
shaped like a ledger.  The workflow treats exit 1 as "drop the restored ledger
and start empty", which only costs hits.
"""

import os
import re
import stat
import sys

SEGMENT = re.compile(r"^[0-9]{10}-[0-9a-f]{16}\.jsonl$")
MANIFESTS = "manifests"


def count_segments(directory):
    """Return (records, bytes) of the segment files directly inside directory.

    A missing directory is an empty ledger.  Subdirectories other than the
    manifest store, symlinks, FIFOs and misnamed files are refused (ValueError).
    """
    records = 0
    size = 0
    try:
        names = sorted(os.listdir(directory))
    except FileNotFoundError:
        return 0, 0
    for name in names:
        path = os.path.join(directory, name)
        mode = os.lstat(path).st_mode
        if name == MANIFESTS and directory_is_ledger_root(directory):
            continue  # counted by the caller, as its own directory
        if not stat.S_ISREG(mode) or not SEGMENT.match(name):
            raise ValueError("not a ledger segment")
        size += os.lstat(path).st_size
        with open(path, "rb") as fh:
            for line in fh:
                if line.strip():
                    records += 1
    return records, size


def directory_is_ledger_root(directory):
    """True for the directory the caller named, false for the manifests/ below it.

    Only the root may contain a `manifests` entry; a `manifests` inside
    `manifests` is not a ledger shape.
    """
    return os.path.basename(os.path.normpath(directory)) != MANIFESTS


def ledger_stats(root):
    """Return (records, bytes) over the pass segments and the manifest segments."""
    records, size = count_segments(root)
    sub = os.path.join(root, MANIFESTS)
    try:
        mode = os.lstat(sub).st_mode
    except FileNotFoundError:
        return records, size
    if not stat.S_ISDIR(mode):
        raise ValueError("manifests is not a directory")
    more, more_size = count_segments(sub)
    return records + more, size + more_size


def main(argv):
    if len(argv) != 2:
        print("usage: record_ledger_stats.py <ledger-dir>", file=sys.stderr)
        return 2
    try:
        records, size = ledger_stats(argv[1])
    except (ValueError, OSError):
        print("the ledger directory is not shaped like a ledger", file=sys.stderr)
        return 1
    print(f"records={records} bytes={size}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
