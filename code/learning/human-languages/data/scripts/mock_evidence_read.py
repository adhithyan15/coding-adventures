"""Batch pinned full-mock evidence reads for the TypeScript artifact gate.

Request and response are JSON on stdin/stdout. The trusted root is supplied by
the caller; all descendant names are treated as untrusted and opened through
sharded_ledger's cross-platform handle-relative boundary.
"""

import json
import sys

from sharded_ledger import read_track_regular_text


def main():
    request = json.load(sys.stdin)
    if not isinstance(request, dict):
        raise ValueError("mock evidence request must be an object")
    root = request.get("root")
    track = request.get("track")
    paths = request.get("paths")
    if not isinstance(root, str) or not isinstance(paths, list):
        raise ValueError("mock evidence request needs root and paths")
    if any(not isinstance(path, str) for path in paths) or len(paths) != len(set(paths)):
        raise ValueError("mock evidence paths must be distinct strings")
    files = {}
    for path in paths:
        try:
            files[path] = read_track_regular_text(root, track, path)
        except Exception as cause:
            # Report only the declared name and exception class. An exception
            # message (notably UnicodeDecodeError) can quote evidence bytes.
            json.dump({"error": {"path": path, "kind": type(cause).__name__}}, sys.stdout)
            return 1
    # ASCII transport is portable across Windows console code pages; JSON
    # escapes round-trip every script's original Unicode text in Node.
    json.dump({"files": files}, sys.stdout, ensure_ascii=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
