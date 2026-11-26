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


# BOUNDS.  A restored cache is data from an earlier run and is read before it is
# trusted, so every read is bounded.  Over any bound the script exits 1 with the
# fixed message, and the workflow starts from an empty ledger (it only loses hits).
#
#   MAX_ENTRIES      directory entries per directory.  A real ledger holds one
#                    segment per merged run; 100000 is far beyond any plausible one.
#   MAX_FILE_BYTES   one segment, from the descriptor's fstat.  The Go reader's own
#                    cap is 64 MiB per segment; this one is looser on purpose.
#   MAX_TOTAL_BYTES  every segment together; the Go reader's MaxDirBytes (256 MiB).
#   CHUNK            the read size, so memory stays flat whatever the file holds.
MAX_ENTRIES = 100_000
MAX_FILE_BYTES = 256 << 20
MAX_TOTAL_BYTES = 256 << 20
CHUNK = 1 << 20


def count_records(fd, size):
    """Count the non-blank lines of the open descriptor in fixed-size chunks.

    Same meaning as the earlier line loop: a line counts when it holds a byte that
    is not whitespace, and a final line with no newline counts too.  `pending` says
    the current (unterminated) line already has such a byte, so a line split across
    chunks is counted once.
    """
    records = 0
    pending = False
    remaining = size
    while remaining > 0:
        chunk = os.read(fd, min(CHUNK, remaining))
        if not chunk:
            break
        remaining -= len(chunk)
        parts = chunk.split(b"\n")
        for part in parts[:-1]:
            if pending or part.strip():
                records += 1
            pending = False
        pending = pending or bool(parts[-1].strip())
    if pending:
        records += 1
    return records


def open_segment(path):
    """Open a segment without following a symlink and return (fd, size).

    There is no lstat-then-open gap: the descriptor is opened with O_NOFOLLOW (a
    symlink fails) and O_NONBLOCK (a FIFO cannot hang the open), and the type and
    the size come from fstat on that descriptor.  Anything but a regular file is
    refused.
    """
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(path, flags)
    try:
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode):
            raise ValueError("not a ledger segment")
        return fd, st.st_size
    except BaseException:
        os.close(fd)
        raise


def count_segments(directory, budget):
    """Return (records, bytes) of the segment files directly inside directory.

    budget is the bytes still allowed in total.  A missing directory is an empty
    ledger.  Subdirectories other than the manifest store, symlinks, FIFOs,
    misnamed files and anything over a bound are refused (ValueError).
    """
    records = 0
    size = 0
    try:
        names = os.listdir(directory)
    except FileNotFoundError:
        return 0, 0
    if len(names) > MAX_ENTRIES:
        raise ValueError("too many entries")
    for name in sorted(names):
        path = os.path.join(directory, name)
        if name == MANIFESTS and directory_is_ledger_root(directory):
            continue  # counted by the caller, as its own directory
        if not SEGMENT.match(name):
            raise ValueError("not a ledger segment")
        fd, file_size = open_segment(path)
        try:
            if file_size > MAX_FILE_BYTES or size + file_size > budget:
                raise ValueError("over the size bound")
            size += file_size
            records += count_records(fd, file_size)
        finally:
            os.close(fd)
    return records, size


def directory_is_ledger_root(directory):
    """True for the directory the caller named, false for the manifests/ below it.

    Only the root may contain a `manifests` entry; a `manifests` inside
    `manifests` is not a ledger shape.
    """
    return os.path.basename(os.path.normpath(directory)) != MANIFESTS


def ledger_stats(root):
    """Return (records, bytes) over the pass segments and the manifest segments."""
    records, size = count_segments(root, MAX_TOTAL_BYTES)
    sub = os.path.join(root, MANIFESTS)
    try:
        mode = os.lstat(sub).st_mode
    except FileNotFoundError:
        return records, size
    if not stat.S_ISDIR(mode):
        raise ValueError("manifests is not a directory")
    more, more_size = count_segments(sub, MAX_TOTAL_BYTES - size)
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
