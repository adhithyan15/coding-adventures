# Changelog — engram-capi

Entries start at the change below. This crate shipped without a changelog, and
reconstructing one from git after the fact would be inventing a record rather
than keeping one; what is here is what has been tracked since.

## Unreleased

**The `rusqlite` and `tempfile` dev-dependencies are gone.**

The tests built their V11 Anki fixture by opening a real bundled-C SQLite
database through `rusqlite`, writing five tables into a temporary file, and
reading the bytes back. It worked, but it compiled the SQLite C amalgamation on
every test build of the one crate in the Engram stack whose entire purpose is to
not need a C toolchain — and while it stood, "the Engram crates no longer
reference `rusqlite`" was not a claim anyone could settle with a grep.

The fixture is now built by this repository's own `sqlite-file` writer, via
`write_multi_table_db_with`, with `user_version: 11` — which is what real Anki
stamps and what our own V11 exporter writes, and which the old fixture never set
at all.

This is not circular. Nothing in the fixture is an oracle: it is *input*, and
what is under test is the C ABI's import path. The writer's own correctness is
measured against real C SQLite over in `sqlite-file`, which deliberately keeps
its `rusqlite` dev-dependency for exactly that purpose.

After the swap, `engram-capi`'s full dev graph — normal, build, and dev edges —
contains no `rusqlite`, no `libsqlite3-sys`, no `cc`, and no C source of any
kind. Every remaining non-registry crate in it is one this repository owns; the
only third-party crates left are `serde`/`serde_json` and their macro
transitives, which #14414 tracks separately.

No production change: the dependency graph the `cdylib` is built from is
untouched, it still exports all 47 `eg_*` symbols, and it still links nothing
beyond `libSystem`.
