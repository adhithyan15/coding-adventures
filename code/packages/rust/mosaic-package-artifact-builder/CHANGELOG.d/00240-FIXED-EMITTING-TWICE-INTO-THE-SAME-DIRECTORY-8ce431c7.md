### Fixed — emitting twice into the same directory

`generated_files_on_disk` walked the output directory and treated **every file
it found** as something this build generated. That is true only into a clean
directory. Emit twice into the same output — the ordinary local workflow, and
what `create_dir_all` is deliberately tolerant of ("not an error if the
directory already exists, which is the behaviour we want for incremental
rebuilds") — and the previous run's installed host files are still sitting
there, counted as this build's.

One cause, two bugs, and the second is the worse one.

**`install_host_effects` refused the re-run outright.**

```
mosaic-compile pkg … --backend qt --output DIR --emit-project   # ok
mosaic-compile pkg … --backend qt --output DIR --emit-project   # FAILED
io error: host effect target engram_effects.h would replace a generated file
```

Backend-agnostic: reproduced on Qt and SwiftUI. Any package declaring
`[host_effects]` could be emitted exactly once per directory.

**`install_host_assets` reported a replacement that never happened.** Measured
on Engram/compose, which declares host assets and no host effects, so it
survived a re-run to show the symptom:

| | `replacedGeneratedFiles` |
| --- | --- |
| run 1 | `[]` |
| run 2 | `["src/main/kotlin/MosaicHost.kt"]` |

The file it "replaced" was its own output from run 1. This is the more serious
half: `replacedGeneratedFiles` is the field a reviewer reads to see whether a
package has taken over the application boundary, and UI47 §5.5.5 pins its
acceptance assertion to it. A disclosure field that invents a disclosure
degrades the one signal meant to be trustworthy, in the direction that teaches
readers to ignore it.

#### Why no test and no CI lane caught either

Every existing test builds a fresh scratch directory per assertion, and every CI
lane emits into a new `$RUNNER_TEMP`. The bug needs a directory that already has
output, which is precisely the case nothing exercised — and precisely what the
generated README tells people to do.

#### The fix, and what it deliberately does not do

A file counts as this build's if **either** the build wrote it, or it changed
under the build. Two mechanisms, because each covers the other's blind spot.

The write record is kept in `write_file` itself, the single primitive every byte
reaching the output goes through. The directory is also stamped before emission
begins, and a file whose `(mtime, length)` is unchanged since then is excluded;
each file is compared against **its own** earlier stamp rather than a wall-clock
fence, which keeps that half free of clock skew.

**The second mechanism is not redundant, and the first version of this shipped
without it.** Security review pointed out that the `length` half of the stamp is
inert: the emitters are deterministic, so a re-run writes byte-identical content
and lengths match by construction. Mtime was doing the work alone — and on a
1-second-granularity mount (gRPC-FUSE, some NFS and overlay mounts) two
back-to-back builds land in the same tick, at which point *every* generated file
reads as pre-existing and a genuine takeover would be disclosed as `[]`. Content
hashing cannot help here for the same reason the length cannot: the bytes are
supposed to be identical.

Recording in `write_file` rather than having each emitter append to a returned
list is the point, not an implementation detail. That list is where the earlier
false negative came from, and an emitter cannot forget to do something it does
not do.

It stays a directory walk. Replacing the walk with a list the emitters push to
was the obvious fix and is the wrong one: that list is what an earlier bug came
from — `emit_index_file` writes two files and returns one path, so overwriting
the HTML app shell reported nothing — and the walk's whole virtue is catching a
file written by any means. The subtraction removes only files this build did not
touch.

An unreadable stamp counts as **changed**, not unchanged. Over-reporting a
replacement is noise; missing one is the silent false negative the field exists
to prevent.

#### Mutation-tested in both directions

The fix widens what counts as "not generated", so it is pinned against
loosening as well as against reverting:

| mutation | fails |
| --- | --- |
| presence means generated (the old behaviour) | the 2 re-run tests |
| nothing counts as generated | 7 tests, including every replacement-refusal and disclosure test |
| drop the write-record half of the union | `a_file_this_build_wrote_counts_even_if_its_stamp_looks_unchanged` |

Four tests added, covering what no existing test could: a second emit into the
same directory succeeds; a file this build actually wrote is still refused; an
untouched leftover is not generated while a rewritten one is; and a file this
build wrote counts even when its stamp claims otherwise.

One of them was vacuous first time round and is worth recording as such: the
leftover test rewrote its fixture from 23 bytes to 27, so it passed on the inert
`length` half and never exercised the mtime comparison at all. It now rewrites
to the same length with different content, which is the shape a real re-run
produces.

