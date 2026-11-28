# DB05e3b3b — Restore a quiescent NAND relocation device

Rebuild the block-enabled raw Go relocation device from the portable
page map and the ordered physical NAND observation. This slice provides
a raw constructor for the later protected recovery composition.
[Settled page map](DB05e3b3a-ftl-restore-page-map.md) · [Raw capture](DB05e3b2a-ftl-physical-media-capture.md)

```go
func RestoreQuiescent(media MediaObservation, snapshot ftlpagemap.Snapshot) (Device, error)
```

The caller supplies both views from one quiescent recovery boundary.
The two values stay stable during the call. The constructor opens
with an empty evacuation lifecycle and regenerated process-local
owner identities. A caller restarting an active evacuation must
first reconcile its durable records at the recovery boundary.

Check in this order and return the zero `Device` on a refusal.

1. Validate the block count and pages per block. Derive the physical
   page count with a checked product bounded at 128. Use the existing
   block geometry refusal for invalid block dimensions.
2. Restore the paired page map through `ftl-page-map.Restore` and
   propagate its typed errors. Validate the media page width and
   configured completed-erase ceiling using the relocation
   constructor's existing checks.
3. Visit blocks in ascending physical order. Require the exact page
   count, a completed-erase count at or below the supplied ceiling,
   and an unset incomplete-erase flag. Report a typed block refusal
   before checking the block's pages. Incomplete erase recovery is
   the next media slice and requires its own eligibility checks.
4. Within each block visit pages in ascending ordinal order. Require
   exact page width. Every clear marker requires bytes of `0xff`.
   A free page has a clear marker. Live and stale pages have a set
   marker. A set-marker page may contain any byte pattern, including
   all `0xff`. Report a typed role or media consistency refusal at
   the first offending page.

Copy the physical bytes and block progress into the device only after the
full paired observation passes. Create a fresh identity for each current
logical owner. An observation from the previous process, or another restore
from the same evidence, cannot authorize an owner mutation on this instance.
A successful snapshot and raw media observation preserve both supplied views.

Verify the physical and role matrix with an independent oracle, including
bad dimensions, invalid counts, reference mismatches, marker parity,
all-ones programs and multiple simultaneous faults. Round-trip real
quiescent histories involving programming, invalidating, copying, and
completed block erases. Check write, read, and erase continuations and
prove fresh owner identities with actual observation/invalidation calls.
Check detached input and output bytes at the maximum supported geometry.
Use both native lists, coverage, race, vet, guided fuzz, architecture and
emitted recipe checks. Run clean-controlled variants for missing geometry,
role and marker checks and identity reuse. Use the existing fixtures,
dependencies and build fronts.

The later protected recovery composition binds the current layout and
read-fault state, processes durable losses, and gates allocation on
quarantine and incomplete-erase evidence.
[Recovery order](DB05e3b-ftl-replay-ownership-boundary.md)

## Go verification

The reviewed runtime SHA-256 is
`3b49955420cb8c1c97da845d8afb85478d48c7995ccd58d23482a041a80db53c`.
An independent public caller matched 210,386 restore decisions and
2,674 admitted round trips, including 128 pages of 65,536 bytes each.
It verified detached input, real erase and write continuations,
and refusal of owner observations retained from before restart or
another restoration.

Both native command lists passed on Linux with race, vet and
96.6% package statement coverage. Guided fuzz completed 191,668
inputs. The six selected clean-controlled mutation witnesses were
detected. Restore-focused Linux/386 tests ran under QEMU and the
Windows/amd64 test binary compiled.

All 5,989 graph rows and 5,988 Linux recipes matched the prior
export byte for byte. The emitted recipe passed from a path with
spaces. [Interrupted erase recovery](DB05e3b3c-ftl-restore-interrupted-erases.md)
