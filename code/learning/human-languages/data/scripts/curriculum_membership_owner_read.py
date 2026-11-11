"""Transport exact curriculum-membership owner sets through pinned handles.

The caller derives expected names from independent lesson filenames. This
process bounds concurrent reads without trusting the owner directory listing.
"""

import json
import re
import sys
from concurrent.futures import ThreadPoolExecutor

from sharded_ledger import WINDOWS_RESERVED, read_track_regular_texts


LANGUAGE = re.compile(r"^[a-z][a-z0-9-]*$")


def main():
    try:
        request = json.loads(sys.stdin.buffer.read().decode("utf-8"))
        if not isinstance(request, dict) or set(request) != {"root", "tasks"}:
            raise ValueError("invalid curriculum owner request")
        root = request["root"]
        tasks = request["tasks"]
        if not isinstance(root, str) or not root:
            raise ValueError("invalid curriculum owner root")
        if not isinstance(tasks, list) or not 0 < len(tasks) <= 64:
            raise ValueError("invalid curriculum owner task count")

        def read_task(task):
            if not isinstance(task, dict) or set(task) != {"language", "names"}:
                raise ValueError("invalid curriculum owner task")
            language = task["language"]
            if (
                not isinstance(language, str) or not LANGUAGE.fullmatch(language)
                or WINDOWS_RESERVED.fullmatch(language)
            ):
                raise ValueError("invalid curriculum owner language")
            return read_track_regular_texts(
                root, language, ("curriculum-membership.d",), task["names"],
                owner_kind="curriculum-membership",
            )

        # Eight pinned track directories at once; each drains at most 16 reads.
        with ThreadPoolExecutor(max_workers=8) as executor:
            results = list(executor.map(read_task, tasks))
        response = {"results": results}
        status = 0
    except Exception as cause:
        response = {"error": {"kind": type(cause).__name__}}
        status = 1
    sys.stdout.buffer.write(json.dumps(response, ensure_ascii=True).encode("ascii"))
    return status


if __name__ == "__main__":
    sys.exit(main())
