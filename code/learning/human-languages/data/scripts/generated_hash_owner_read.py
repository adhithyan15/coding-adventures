#!/usr/bin/env python3
"""Read generated-hash owner sets without path-based descendant enumeration.

Each task pins the hash-kind directory, refuses a flat monolith from that
handle, opens the language owner directory relative to the same handle, then
lists and reads its exact files. Node remains the authoritative JSON validator.
"""

import json
import os
import re
import stat
import sys
from concurrent.futures import ThreadPoolExecutor

import sharded_ledger as boundary

LANGUAGE = re.compile(r"[a-z][a-z0-9-]*")
OWNER = re.compile(r"(?:_meta|[0-9]{4})\.json")


def _validate_names(names):
    if not names or "_meta.json" not in names:
        raise ValueError("generated hash owners lack metadata")
    if len(names) != len(set(names)) or len(names) != len({name.lower() for name in names}):
        raise ValueError("duplicate generated hash owner names")
    if any(OWNER.fullmatch(name) is None for name in names):
        raise ValueError("malformed generated hash owner name")
    if len(names) == 1:
        raise ValueError("generated hash owners lack chapters")


def _read_windows(root, kind, language):
    with boundary._windows_directory(root, "core", kind, list_entries=True) as parent:
        parent_names = [name for name, _attributes in boundary._windows_query_directory(parent)]
        if (language + ".json").lower() in {name.lower() for name in parent_names}:
            raise ValueError("resurrected generated hash monolith")
        child = boundary._windows_open_relative(
            parent, language + ".d", directory=True,
            access=boundary._READ_DATA | boundary._READ_ATTRIBUTES | boundary._SYNCHRONIZE,
        )
        try:
            entries = boundary._windows_query_directory(child)
            names = [name for name, _attributes in entries]
            _validate_names(names)
            if any(attributes & (boundary.REPARSE_POINT | boundary._DIRECTORY_ATTRIBUTE)
                   for _, attributes in entries):
                raise ValueError("generated hash owner is not a regular file")
            files = boundary._read_pinned_regular_batch(
                child, names, boundary._windows_read_regular,
            )
            return {"names": names, "files": files}
        finally:
            boundary._close_handle(child)


def _read_unix(root, kind, language):
    with boundary._unix_directory(root, "core", kind) as parent:
        if language + ".json" in os.listdir(parent):
            raise ValueError("resurrected generated hash monolith")
        child = os.open(
            language + ".d", os.O_RDONLY | boundary.DIRECTORY | boundary.NOFOLLOW,
            dir_fd=parent,
        )
        try:
            names = os.listdir(child)
            _validate_names(names)

            def read_one(directory_fd, name):
                descriptor = os.open(name, os.O_RDONLY | boundary.NOFOLLOW, dir_fd=directory_fd)
                with os.fdopen(descriptor, encoding="utf-8") as handle:
                    if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
                        raise ValueError("generated hash owner is not a regular file")
                    return handle.read()

            files = boundary._read_pinned_regular_batch(child, names, read_one)
            return {"names": names, "files": files}
        finally:
            os.close(child)


def main():
    request = json.load(sys.stdin)
    root = request["root"]
    tasks = request["tasks"]
    if not isinstance(tasks, list) or not 0 < len(tasks) <= 64:
        raise ValueError("invalid generated-hash batch size")

    def read_task(task):
        if not isinstance(task, dict):
            raise TypeError("invalid generated-hash task")
        kind = task["kind"]
        language = task["language"]
        if kind not in ("generated-book-hashes", "generated-narration-hashes"):
            raise ValueError("invalid generated-hash kind")
        if not isinstance(language, str) or LANGUAGE.fullmatch(language) is None:
            raise ValueError("invalid generated-hash language")
        return (_read_windows if os.name == "nt" else _read_unix)(root, kind, language)

    # Two pinned directories at once; each drains at most sixteen owner reads.
    with ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(read_task, tasks))
    json.dump({"results": results}, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
