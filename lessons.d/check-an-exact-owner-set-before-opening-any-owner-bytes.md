# Check an exact owner set before opening any owner bytes

Sharding the sound-tag registry by language removed the cross-language edit
surface, but completeness could still disappear if the reader discovered its
language set from the surviving owner files. Shape-checking every file is not a
deletion gate: a missing file contributes no malformed bytes to reject.

The independent language registry supplies the expected filenames. Enumerate
and type-check the directory first, compare that exact name set, and only then
open `_meta.json` and the language owners. This also makes diagnostics stable:
a missing Tamil owner is reported before unrelated malformed Spanish bytes.

**Rule:** when another ledger can state the complete identity set, establish
filename equality before reading owner contents. Owner validation proves that
present data is sound; independent identity closure proves that absent data did
not vanish silently.
