# DB05e3b3c — Restore NAND with interrupted block erases

Extend raw restoration to a paired page map and physical observation with
one or more interrupted block erases. Keep the completed-media entry point
and reuse its geometry, map ownership and byte checks.
[Quiescent restore](DB05e3b3b-ftl-restore-quiescent-media.md) · [Raw block erase](DB05c2b-ftl-block-erase-reclamation.md) · [Erase cuts](DB04d-nand-block-erase-cuts.md)

```go
func RestoreWithInterruptedErases(media MediaObservation, snapshot ftlpagemap.Snapshot) (Device, error)
```

The input may also contain only completed-media blocks. Return the
zero device on refusal. Both inputs stay stable during the call. The
new device owns detached page bytes and freshly created process-local owner
identities. Its active evacuation lifecycle is empty at this recovery
boundary.

Use the quiescent constructor's global validation order. Within each
block, validate shape and the configured erase ceiling before visiting
pages in ordinal order. An interrupted block requires a completed-erase
count strictly below that ceiling. Validate the page width and require
all `0xff` bytes under any clear marker. In a completed block, apply the
quiescent role and marker contract. In an interrupted block, free
pages require clear markers, live pages are refused, and stale pages
may carry either marker.

Check the interrupted block's progress after its page scan. It must
have at least one stale page. There must exist a cut prefix
`p` in `0..pagesPerBlock-1` such that every stale page below `p`
has a clear marker and every stale page at or above it has a set marker.
Free pages already have clear markers, so they carry no prefix
information. Refuse a clear stale page after a marked stale page and
a clear marker on a stale final page because they cannot come from
an interrupted prefix. A prior retry with a shorter prefix preserves
the earlier physical progress. Detach bytes and preserve the exact
incomplete flag, erase count and page roles on a successful restore.

Exercise the existing write, copy and exact unowned program entry points
against restored interrupted blocks. Allocation must skip the whole
interrupted block, including any free page in it. A completed erase
advances its generation once, clears the flag and releases the extent
even when its acknowledgement is lost. Before-accept and prefix retries
keep existing progress and allocation restrictions.

Check small geometries against an independent feasible-prefix model.
Compare true raw histories across incomplete erase, restore, repeated
shorter and longer cuts, complete media with lost acknowledgement, and
subsequent allocation. Include prefix zero, the maximum prefix,
all-erased bytes with set markers, no stale pages, live owners, exhausted
erase counts, out-of-order marker evidence, and multiple interrupted
blocks. Check refused zero values, detached snapshots, old owner identity
refusal, both build lists, race, vet, guided fuzz, architecture and
emitted commands. Disposable mutants must expose missing block fences,
impossible prefix admission and premature release. Protected layout and
read availability policy belong to the higher recovery composition.

## Local verification

Commit `d9cf7bd7cb0` passed 21,840 decisions from an independent
prefix oracle and 756 durable round trips. The same witness checked
the whole-block allocation, program and copy fence, shorter and longer
erase retries, and fence release after media completion with a lost
acknowledgement. It also checked owner identity after reopening.

The emitted Go recipe passed from a path containing spaces with
96.8% statement coverage. Race and vet passed, as did 100,000 guided
inputs and three clean-controlled mutations. The full Linux 386 suite
passed under QEMU, and the Windows amd64 test binary cross-compiled.
