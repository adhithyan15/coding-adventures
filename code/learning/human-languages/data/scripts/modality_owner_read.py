"""Transport one exact modality-owner track through a pinned directory handle.

The request is JSON on stdin. Error responses intentionally expose only the
exception class, never a path or source byte that could land in CI logs.
"""

import json
import re
import sys

from sharded_ledger import WINDOWS_RESERVED, read_track_regular_texts


LANGUAGE = re.compile(r"^[a-z][a-z0-9-]*$")


def main():
    try:
        request = json.loads(sys.stdin.buffer.read().decode("utf-8"))
        if not isinstance(request, dict) or set(request) != {"root", "language", "names"}:
            raise ValueError("invalid modality owner request")
        root = request["root"]
        language = request["language"]
        names = request["names"]
        if not isinstance(root, str) or not root:
            raise ValueError("invalid modality owner root")
        if (
            not isinstance(language, str) or not LANGUAGE.fullmatch(language)
            or WINDOWS_RESERVED.fullmatch(language)
        ):
            raise ValueError("invalid modality owner language")
        files = read_track_regular_texts(
            root, "core", ("lesson-modality", f"{language}.d"), names,
        )
        response = {"files": files}
        status = 0
    except Exception as cause:
        response = {"error": {"kind": type(cause).__name__}}
        status = 1
    sys.stdout.buffer.write(json.dumps(response, ensure_ascii=True).encode("ascii"))
    return status


if __name__ == "__main__":
    sys.exit(main())
