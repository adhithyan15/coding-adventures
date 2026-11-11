#!/usr/bin/env python3
"""Read one exact book-generation owner tree through pinned directory handles.

This is an I/O transport for the TypeScript validator, not a second book
schema. The core, owner root, and each section stay pinned while at most
sixteen files in that section are read relative to its handle. A pathname
swap after an initial check therefore cannot redirect a later owner read.
"""

import json
import os
import stat
import sys

import sharded_ledger as boundary

SECTIONS = (
    "script-sets.d",
    "reference-appendices.d",
    "glossaries.d",
    "answer-keys.d",
    "indexes.d",
    "targets.d",
    "handwritten.d",
)
TOP = {"_meta.json", *SECTIONS}


def _exact_top(names):
    if len(names) != len(TOP) or set(names) != TOP:
        raise ValueError("book-generation owner root has missing or unexpected entries")


def _owner_names(names):
    # `.gitkeep` is the tracked witness for an otherwise empty section. It is
    # not an owner; every other member must be an exact JSON candidate for
    # TypeScript's schema and identity checks.
    owners = [name for name in names if name != ".gitkeep"]
    if any(not name.endswith(".json") for name in owners):
        raise ValueError("book-generation section has a malformed owner name")
    if len(owners) != len(set(owners)) or len(owners) != len(
        {name.lower() for name in owners}
    ):
        raise ValueError("book-generation section has duplicate owner names")
    return owners


def _windows(root):
    with boundary._windows_directory(root, "core", list_entries=True) as core:
        core_names = {
            name.lower() for name, _ in boundary._windows_query_directory(core)
        }
        if "book-generation.json" in core_names:
            raise ValueError("resurrected book-generation monolith")
        book = boundary._windows_open_relative(
            core,
            "book-generation.d",
            directory=True,
            access=boundary._READ_DATA
            | boundary._READ_ATTRIBUTES
            | boundary._SYNCHRONIZE,
        )
        try:
            top = boundary._windows_query_directory(book)
            _exact_top([name for name, _ in top])
            for name, attributes in top:
                is_dir = bool(attributes & boundary._DIRECTORY_ATTRIBUTE)
                if attributes & boundary.REPARSE_POINT or is_dir != (
                    name != "_meta.json"
                ):
                    raise ValueError(
                        "book-generation owner root has a linked or wrong-kind entry"
                    )
            meta = boundary._windows_read_regular(book, "_meta.json")
            sections = {}
            for directory in SECTIONS:
                child = boundary._windows_open_relative(
                    book,
                    directory,
                    directory=True,
                    access=boundary._READ_DATA
                    | boundary._READ_ATTRIBUTES
                    | boundary._SYNCHRONIZE,
                )
                try:
                    entries = boundary._windows_query_directory(child)
                    names = _owner_names([name for name, _ in entries])
                    if any(
                        attributes
                        & (boundary.REPARSE_POINT | boundary._DIRECTORY_ATTRIBUTE)
                        for name, attributes in entries
                        if name != ".gitkeep"
                    ):
                        raise ValueError(
                            "book-generation owner is linked or not a regular file"
                        )
                    sections[directory] = {
                        "names": names,
                        "files": boundary._read_pinned_regular_batch(
                            child,
                            names,
                            boundary._windows_read_regular,
                        ),
                    }
                finally:
                    boundary._close_handle(child)
            return {"meta": meta, "sections": sections}
        finally:
            boundary._close_handle(book)


def _unix(root):
    with boundary._unix_directory(root, "core") as core:
        if "book-generation.json" in os.listdir(core):
            raise ValueError("resurrected book-generation monolith")
        book = os.open(
            "book-generation.d",
            os.O_RDONLY | boundary.DIRECTORY | boundary.NOFOLLOW,
            dir_fd=core,
        )
        try:
            _exact_top(os.listdir(book))

            def read_one(directory_fd, name):
                # A hostile `.json` FIFO must not wait for a writer. The flag
                # does not affect regular files, which fstat still requires.
                descriptor = os.open(
                    name,
                    os.O_RDONLY | os.O_NONBLOCK | boundary.NOFOLLOW,
                    dir_fd=directory_fd,
                )
                with os.fdopen(descriptor, encoding="utf-8") as stream:
                    if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                        raise ValueError("book-generation owner is not a regular file")
                    return stream.read()

            meta = read_one(book, "_meta.json")
            sections = {}
            for directory in SECTIONS:
                child = os.open(
                    directory,
                    os.O_RDONLY | boundary.DIRECTORY | boundary.NOFOLLOW,
                    dir_fd=book,
                )
                try:
                    names = _owner_names(os.listdir(child))
                    sections[directory] = {
                        "names": names,
                        "files": boundary._read_pinned_regular_batch(
                            child, names, read_one
                        ),
                    }
                finally:
                    os.close(child)
            return {"meta": meta, "sections": sections}
        finally:
            os.close(book)


def main():
    request = json.load(sys.stdin)
    if (
        not isinstance(request, dict)
        or set(request) != {"root"}
        or not isinstance(request["root"], str)
    ):
        raise ValueError("invalid book-generation owner request")
    result = (_windows if os.name == "nt" else _unix)(request["root"])
    json.dump(result, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
