# A staging copy silently disabled a security guard three commits later

`_zip_tree` refuses hard links, with a stated reason: a hard link reaches
outside the payload with none of a symlink's tells, and `.git/config` on a
runner holds the checkout token. That guard was mutation-tested and worked.

Then I added a staging step — copy the bundle into a temp directory, archive
that — to stop the SwiftUI payload including the whole SwiftPM project. The
guard has not fired since, because `shutil.copytree` **dereferences** a hard
link: it reads the bytes and writes a fresh file with `st_nlink == 1`. By the
time the archiver walks the staged tree there is nothing left to detect.
Verified on a fixture:

```
_zip_tree on the source tree : refused: payload contains a hard link
after copytree staging       : nlink 1 -> ARCHIVED: b'AUTHORIZATION: basic ...'
```

Nothing about the guard changed. What changed was *what it was pointed at*, and
the two commits were far enough apart that neither review looked at both.

Three things generalise:

- **A check is defined by the tree it walks, not by the function it lives in.**
  Inserting a transformation upstream of a validator can void it without
  touching a line of it, and no test of the validator will notice — the fixture
  goes straight into the validator, not through the new step.
- **When you add a copy, ask what the copy normalises away.** `copytree`
  flattens hard links, and with `symlinks=False` would flatten symlinks too. A
  copy is not identity; it is a filter with opinions.
- **Verify the tree you are about to ship, not the one you inspected.** The
  engine was checked on the source and never re-checked on the staged copy, so
  the property asserted was not a property of the published bytes. Now checked
  on both.

Also from the same review, and the same species as the byte scan: the engine
check matched on FILENAME. It accepted `engram_capi.pdb`, a one-byte file, and
a text file — while rejecting a real versioned soname
(`libengram_capi.so.0.4.0`), because `Path.stem` strips only one suffix. A Rust
cdylib on Windows emits `engram_capi.dll`, `engram_capi.dll.lib` and
`engram_capi.pdb` side by side, so a copy step with a sloppy glob picks up the
debug symbols and passes. It now checks the platform's container magic and
matches on `name`. The release note text was corrected too: it claimed the lane
"carries the engine", where the code proves "a shared library of the right kind
is in the right directory". **Prose outruns code — say what the check proves.**
