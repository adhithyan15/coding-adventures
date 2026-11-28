# DB12c2c2 — Recover a committed WAL prefix on one device

Recover fixed data pages from a disjoint log reservation on a
restarted logical block device. Take the data page actions only after
the layout, captured log, tail policy and complete redo plan pass.
[Reservation pair](DB12c2c1-go-wal-data-layout.md) · [Tail policy](DB12c2b2-go-wal-tail-admission.md) · [Redo replay](DB12c2a-go-wal-redo-apply.md)

Add the Go package `storage-wal-recovery`.

```go
func Recover(
    device blockdevicecontract.Device,
    log, data storagepagelayout.Layout,
) (storagewalredoapply.Receipt, error)
```

The caller provides exclusive access to a restarted device with no
pending writes. The log and data layouts name intervals on that same
device. Keep both intervals hidden from foreground clients until
recovery has completed.

Validate the reservation pair first and preserve the first error.
Capture the log through the shared bounded reader and preserve its
geometry, read or hard scan refusal. Apply the initial tail policy.
Return the typed damaged-tail error with its page and original cause.
A valid prefix includes any complete commit record that reached the
device despite a lost acknowledgement. Empty or incomplete transaction
prefixes are admitted. They can produce an empty redo plan.

Return a zero replay receipt on these refusals. After admission,
call the full-image replay boundary with the captured records and
the data layout. Preserve its exact receipt and error. Data replay
writes only declared data pages. The original log bytes remain
available for another restart, including after a failed replay barrier.

Compare exact block calls and durable bytes with scripted devices and
real restarted memory and HDD profiles. Include nonoverlap and
adjacency, every reserved-log read fault, hard scan errors, damage
before and after a valid prefix, full and vacant scans, a complete
unacknowledged commit, incomplete transactions and repeated recovery
with a second crash. Verify untouched log blocks and intervening
non-data neighbors. Run both build lists, race, vet, guided inputs and
controls for using a damaged tail or writing before full admission.

## Local verification

The implementation is in `8827977720e`. An independent caller checked
66 actual commit barrier cuts through one restarted HDD. It classified
12 intact commits, including 11 with absent acknowledgements, two
vacant tails and 52 damaged tails. It crossed each intact commit
with all 12 data-page barrier cuts for 144 interrupted or complete
replay cases, then checked final replay convergence. It compared the
whole device after every restart and preserved the log and guards.

Both command lists passed on Linux at 100% statement coverage with
race and vet. Module tidy, 100,000 guided inputs and both mutation
controls passed. Native tests also checked every log-block read
position and repeated recovery on memory and HDD. The build graph
gained only the recovery package and its 17 expected edges per platform.
