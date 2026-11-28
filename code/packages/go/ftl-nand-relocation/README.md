# ftl-nand-relocation (Go)

This package composes a value-owned flash translation table with deterministic
NAND page-program cuts. Each write reserves the lowest free physical page and
returns a successor device. A prefix cut spends the destination and keeps the
original mapping. Full media completion publishes the destination even when
an all-ones payload leaves its bytes unchanged. Publication and host
acknowledgement remain separate receipt fields.

`WriteExcluding` accepts a caller-owned Boolean mask with one entry per
physical page. A true entry fences that page from allocation. The allocator
combines this mask with any incomplete-erase and active-evacuation fences and
reserves the lowest remaining free page. `Write` uses the same transition with
an empty mask. Invalid mask width is refused after logical-address and
page-map pending-reservation validation. Every refusal preserves the device
and returns a zero receipt. Reads of existing live mappings do not consult
allocation fences.

The optional block-enabled profile composes DB04d erase cuts with atomic
page-map release. Every slot in an incompletely erased block is quarantined.
An unacknowledged `AfterMedia` result advances the media completion count and
releases the extent.

`PlanRetirementDrain` provides a read-only view of a readable source block.
It delegates ordered copy selection to `ftl-evacuation-plan.PlanDrain` after
combining the caller's block mask with incomplete erase extents and the active
reclamation target. A caller may exclude the source itself. Planning does not
consume erase budget or change the device, and empty or all-live sources are
valid when their destinations suffice.

`BeginEvacuationExcluding` preflights a target using caller-owned block
destination fences combined with incomplete-erase quarantine. It validates
the mask after geometry, profile, and signed block address, and before active,
incomplete, and erase-budget state. The victim's own mask bit is irrelevant;
the later source policy determines whether that block may be selected. The
legacy `BeginEvacuation` delegates with an empty mask.

`CopyNextEvacuationPageExcluding` applies the same current block-destination
policy to one active copy. It checks configuration, profile, and active state
before mask width, then checks for live work and requires a full current plan
before applying the first move. Prefix cuts consume a destination, so the next
call replans with its new mask and state. The legacy copy entry point delegates
with an empty mask.

`LivePagesInBlock` reports occupancy from the current page-map status over one
aligned physical block. It counts only `Live` mappings, so stale, free, spent,
and fenced slots do not contribute. The read-only observation works for active
or incomplete blocks and does not depend on the erase ceiling or inspect page
bytes and program markers.

`OwnersInBlock` returns a detached list of the current logical owners in a
physical erase block, sorted by logical page. It uses the same map/profile and
signed-block validation order as `LivePagesInBlock`; a valid empty block returns
an empty list, while refusal returns `nil`. Owner identity follows the settled
logical-to-physical map across interrupted writes, copies, and erase cuts.

`ObserveBlock` returns the raw physical pages, completed erase count and
incomplete-erase flag for one block. The block page slice and every page byte
slice are detached. It preserves raw evidence for active and incomplete blocks;
readability and metadata-slot admission remain decisions for downstream code.

`ObserveReclamation` provides a compact detached view for a victim selector:
physical page roles in ordinal order, per-block erase counts and incomplete
flags, the block geometry and ceiling, and the active evacuation target. It
does not copy page bytes or expose logical owners. Geometry refusal precedes
the legacy-profile `ErrEraseDisabled` refusal; both return a zero observation.
The active flag distinguishes an active block zero from no active target.

`ProgramUnownedPage(block, page, payload, cut)` programs one caller-selected
page in an erase-block device without creating or changing a logical mapping.
Block/profile and signed block checks precede page-offset validation, then
incomplete and active-block refusals precede the Free-target check. The
admitted NAND program operation validates payload and cut. Before acceptance,
the receipt still identifies the absolute physical ordinal while the page
remains free. Any consumed result marks only that page stale, preserves exact
bytes and the program marker, and reports acknowledgement separately. A
completed erase makes the page available for another unowned program.

`Lookup` reports the current physical owner of a logical page without exposing
bytes or changing the device. It returns `(0, false, nil)` for an unallocated
logical page, while `(0, true, nil)` means that physical page zero is its live
owner. Invalid geometry is checked before the signed logical address. `Read`
uses the same lookup and still returns a detached byte slice.

`ObserveOwner` returns that current logical/physical pair plus an opaque,
process-local publication identity. `MatchesOwner` is true only while the same
publication remains current at the recorded coordinates. A successful
media-ready program or copy creates a fresh identity before acknowledgement;
aborted cuts, unrelated publications and block erases preserve other owners'
identities. Independently published sibling branches never share an identity,
even when their addresses and bytes happen to match.

```go
import (
    "fmt"

    ftlnandrelocation "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-nand-relocation"
    nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
    nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

device, err := ftlnandrelocation.NewWithEraseBlocks(1, 4, 2, 2, 3)
if err != nil {
    panic(err)
}
device, _, err = device.Write(
    0,
    []byte{0x10, 0x20},
    ftlnandrelocation.Cut{Kind: nandpageprogramcuts.AfterPrefix},
)
if err != nil {
    panic(err)
}
device, _, err = device.EraseBlock(
    0,
    ftlnandrelocation.EraseCut{Kind: nandblockerasecuts.AfterPrefix},
)
if err != nil {
    panic(err)
}
device, writeReceipt, err := device.Write(
    0,
    []byte{0x10, 0x20},
    ftlnandrelocation.Cut{Kind: nandpageprogramcuts.Complete},
)
if err != nil {
    panic(err)
}
_, eraseReceipt, err := device.EraseBlock(
    0,
    ftlnandrelocation.EraseCut{Kind: nandblockerasecuts.AfterMedia},
)
if err != nil {
    panic(err)
}
fmt.Println(writeReceipt.Physical, eraseReceipt.Released, eraseReceipt.Acknowledged)
// 2 true false
physical, present, err := device.Lookup(0)
if err != nil {
    panic(err)
}
fmt.Println(physical, present)
// 2 true
```

Runtime imports are limited to the standard library and the exact local Go
modules `ftl-page-map`, `nand-page-program-cuts`, `nand-block-erase-cuts`,
`ftl-evacuation-plan` and `ftl-erase-role-evidence`. Tests also use the admitted
`ftl-erase-victim` selector. Go applies `replace` directives only in the main
module, so each standalone consumer that compiles an import of this package
also declares the sibling `ftl-erase-role-evidence` module replacement. This
includes the five direct consumers and the allocation-retirement and
metadata-fences test/build closure.
The bounded fixture readers reject duplicate decoded keys, unknown fields,
invalid UTF-8, trailing
values and documents above 64 KiB or depth 32. They check every declared
state and receipt against separate media, mapping and erase metadata models
before invoking runtime replay. The block vector has four cases for lost
acknowledgement, quarantine retries, multiple incomplete blocks, capacity,
erase ceiling and lowest-slot allocation after release.

`CopyLivePage` rechecks a planned source and exact destination before it
reserves only that destination. It uses the source's current bytes and the
same program/finish path as a host write. A prefix cut spends the destination
without changing the mapping. Media completion publishes the copy while
retaining the old source bytes as stale. `BeforeAccept` leaves the destination
eligible for retry. The eight-case copy vector is validated against the
independent model before runtime replay.

These commands run from the module directory with Go 1.26.1 and
`GOTOOLCHAIN=local`.

```sh
go test -count=1 ./... -cover
go test -race -count=1 ./...
go vet ./...
go test -run=^$ -fuzz=FuzzParseRelocationFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzRelocationHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzParseBlockReclamationFixture -fuzztime=2s -fuzzminimizetime=512x -parallel=2
go test -run=^$ -fuzz=FuzzBlockReclamationHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzParseLivePageCopyFixture -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzLivePageCopyHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzWriteExcludingHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzParseEvacuationFixture -fuzztime=2s -fuzzminimizetime=512x -parallel=2
go test -run=^$ -fuzz=FuzzEvacuationHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzLivePagesInBlockHistory -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzProgramUnownedPageHistories -fuzztime=2s -parallel=2
go test -run=^$ -fuzz=FuzzCaptureEraseIntent -fuzztime=100000x -parallel=2
GO=/path/to/go python3 tools/check_mutations.py
```

The original DB05b suite still explores its 11 and 96 state geometries and
10,000 seeded write/read commands. The new finite block graphs close at 75
and 616 states for profiles 1/2 with one-page blocks and 1/4 with two-page
blocks, each at erase ceiling one. A separate seeded run matches 10,000 mixed
block commands. All valid prefixes and block addresses are exercised, along
with reads and writes after each cut. The canonical block fixture is
model-checked before runtime replay. A zero-prefix erase preserves raw media
and the map. Its quarantine flag changes which pages can be allocated.

Go 1.26.1 on Linux/amd64 passed native tests, race, vet, 45,000 fixture fuzz
inputs (`-fuzzminimizetime=512x`) and 110,000 history fuzz inputs. The suite measured 94.9% runtime
statement coverage including the bounded-graph and seeded-history tests.
Seven disposable mutants were killed by named assertion witnesses. Independent
checks included Linux/386 execution under QEMU and a Windows/amd64 test-binary
cross-compile.

The independent review matched 84,138 transitions over two closed graphs of
211 and 2,063 states, all 33 fixture steps, and a 64/128 device with 65,536-byte
pages. It ran 45,029 fixture inputs and 110,000 history inputs, reproduced all
seven mutation controls, and executed the example above as a Go program.

DB05c3b1 added eight model-first copy-vector cases and exhaustive copy-cut
comparisons in the native suite. The independent review matched 81,421 copy
transitions over 10,516 reachable states in two four-page geometries. It also
compared all eight vector declarations, five portable-integer refusals and
five copy cuts at the 128-page geometry with 65,536-byte pages.

Go 1.26.1 on Linux/amd64 passed the build front with 95.2% statement coverage,
race and vet. The three fixture fuzz targets each completed 45,000 inputs, and
the three history targets each completed 110,000. All ten deliberate runtime
faults reached their named assertions after clean controls. The spent-page
control checks the specific unavailable admission error. The lower allocation
mask independently prevents reuse of a spent page. Linux/386 tests passed
under QEMU and the Windows/amd64 test binary cross-compiled.

DB-E57 corrected a false failure in the block fixture fuzz harness. The
model refuses an altered page marker before replay. An independently
mutated runtime failed during replay of a model-accepted canonical seed.
[DB-E57 spec](../../../specs/DB-E57-block-reclamation-fixture-fuzz-verdict.md)

The original DB05b revision `4172d858c3ae` measured 95.8% runtime statement
coverage. It passed 45,000 fixture and 110,000 history fuzz inputs and four
mutation controls. Its independent media and owner model matched 65,536
writes, 5,813 published mappings and all 17 fixture steps. Its checks included
the full 64/128 geometry with 65,536-byte pages, Linux/386 execution under QEMU
and a Windows/amd64 test-binary cross-compile.

DB05c3b2 adds `BeginEvacuation`, derived `EvacuationProgress`,
`CopyNextEvacuationPage`, `FinishEvacuation` and `CancelEvacuation`. One active
target fences host writes and direct erases. Every page copy uses a fresh plan
from current ownership, so a prefix-spent destination is not reused. Partial
erase keeps the target locked until the full extent is released, including
when acknowledgement is lost. Every refusal returns the original device. Commands that report a receipt
return its zero value when refused.

The `ftl-evacuation.json` vector declares a complete portable snapshot, the
active target and progress after each command. Accepted read, write, copy and
erase outputs are explicit. The entire vector is checked against the
independent model before runtime replay. The native tests close 529 and 2,497
states for the 1/4 and 2/4 profiles with 2-page blocks and erase ceiling one,
and check 10,000 seeded mixed commands. Coverage-guided campaigns completed
45,000 fixture parser inputs and 110,000 controller-history inputs. Four
additional clean-controlled mutations cover active-target allocation,
replanning after a spent prefix, live-target erase refusal and lost-ack media
release.

The final package BUILD measured 96.0% statement coverage. Race and vet passed.
Linux/386 tests passed under QEMU, and the Windows/amd64 test binary
cross-compiled. All 15 named mutation controls reached their intended
witnesses after clean controls.


The independent review matched 770,996 state-graph transitions over 16,664
reachable states, plus 41 targeted steps. It compared the exact 27 shared
snapshots and their 21 operation results. The largest probe used 128
65,536-byte pages and retried after a 65,535-byte program prefix. The four
fixture fuzz targets completed 45,000 inputs each, and the four stateful
targets completed 110,000 each. Five portable-fixture refusal cases and
the earlier block-reclamation fuzz witness also passed.

DB05d2b2a adds `WriteExcluding` without changing `Write` observations. The
bounded write-only graph closes at 16 states/512 transitions for 1/2 geometry
and 232 states/29,696 transitions for the 1/4 block profile, with every
external mask, two payload values and all four program outcomes. Native tests
also check address/mask precedence, all-blocked atomic refusal, live reads
through externally fenced pages, and combined caller, active-evacuation and
incomplete-erase fences. A clean-controlled mutation checks that caller fences
remain part of the combined mask. The new write-exclusion history fuzzer
completed 110,000 inputs, and all 15 clean-controlled mutation witnesses
passed, including the three allocation-fence omission controls.

The independent write-fence model checked 187,568 commands across
155,312 cases. The six starting contexts cover legacy media, normal
block media, a live mapping, an active evacuation, an incomplete
erase, and both internal fences at once. It checked exact receipts,
map status, every physical byte and logical read, caller inputs and
preceding device values. Both command lists passed on Linux with
96.1% runtime statement coverage. Linux/386 execution under QEMU and
Windows/amd64 test compilation passed. The 15 named mutation controls
and 110,000 history fuzz inputs passed on the admitted runtime.

DB05d2b2b1 adds the read-only device retirement view. Its independent
stepwise model checks 24,208 source/mask queries over 3,026 closed states in
the 1/4 and 2/4 small-device graphs. It also checks 56,416 fixed-seed plans
after 18,432 write setup commands across 519 cases. Tests cover zero-budget
empty and all-live sources, active and incomplete source refusals, combined
native and caller destination fences, caller-mask preservation, and detached
plans. For a source that is both active and incomplete, incomplete-erase
refusal takes precedence after mask validation. Four new clean-controlled
mutations cover both internal fences, caller exclusions, and read-only
planning at an exhausted erase budget. Together with the 15 earlier controls,
all 19 killed their intended mutations. The new history fuzzer completed
110,000 inputs. Linux/amd64 BUILD passed with 96.4% statement coverage, race
and vet checks passed, Linux/386 tests executed under QEMU, and Windows/amd64
test compilation passed.

DB05d2b2c1's occupancy view was checked over every state reachable in two
closed device graphs: 529 states for 1/4 geometry and 2,497 states for 2/4.
The tests interleave occupancy queries with 8,192 fixed-seed write commands,
and the history fuzzer completed 110,000 inputs. Native BUILD passed with
96.8% package statement coverage. Race, vet, Linux/386 under QEMU, and
Windows/amd64 cross-compilation passed. The full set of 21 clean-controlled
mutations includes upper-bound and Live-only classification witnesses.

DB05d2c4 adds `OwnersInBlock`, a detached current-owner list ordered by logical
page. Its oracle checks 3,333 inventory queries across 1,111 nodes in all
length-0..3 histories over ten write/cut commands, 65,536 fixed-seed inventory
queries, and the 64-owner/128-page maximum geometry. Targeted tests cover signed
refusal precedence, reverse physical placement, stale ownership, publication
and acknowledgement boundaries, active/incomplete blocks, and detached
predecessor inventories. The owner-history fuzzer completed 100,000 inputs.
Native BUILD passed with 97.0% package statement coverage, race, and vet. Linux
owner tests ran under QEMU/386, Windows/amd64 test compilation passed, and all
29 clean-controlled package mutation witnesses were killed.

DB05d2c6a adds process-local `OwnerObservation` stamps for each media-ready
write or live-page copy. `MatchesOwner` checks the current logical and physical
coordinates along with the private stamp. A fresh-symbol oracle checked all
1,111 length-0..3 branch nodes over ten write/cut commands and 444 publication
events, retaining observations along each ancestor path. Seeded histories
checked 256 x 32 writes with 770 media publications. Targeted cases cover
equal-byte reuse of the same address after erase, interrupted and completed
erase preservation, copy cuts, sibling write/copy branches with equal
coordinates, independent devices, common ancestors, refused writes, and
concurrent readers. The owner-history fuzzer completed 100,000 inputs. Native
BUILD passed with 95.9% package statement coverage, race and vet. Linux/386
owner tests passed under QEMU, Windows/amd64 test compilation passed, and all
33 clean-controlled mutation witnesses were killed.

DB05d2c6b2b1 adds `Device.InvalidateOwner`, which accepts only a current
opaque publication observation and removes that exact logical mapping. It
clears only the invalidated owner's identity and preserves the old physical
bytes, program marker, erase history, active evacuation, and all other owner
identities. Tests cover stale, zero, and tampered observations, write and copy
publication cuts, interrupted erase, zero-budget blocks, and detached
predecessors. Bounded, seeded, and fuzz histories compare device snapshots
against the independent media model. The bounded branch check visits 107
nodes with 14 successful invalidations; 256 seeded 32-step histories and a
100,000-input coverage-guided run also passed. BUILD passed with 95.8%
statement coverage, race and vet. Linux/386 tests ran under QEMU,
Windows/amd64 test compilation passed, and all 36 clean-controlled mutations
were killed.

DB05e2b2d adds `ProgramUnownedPage` for caller-selected physical pages. Its
independent byte-array model checks all program cuts, zero and one-byte
prefixes, all-ones payloads, absolute receipt ordinals, map/owner preservation,
active and incomplete block refusal, stale/live targets, erase reuse and
detached bytes. A fixed seeded history runs 5,000 mixed host-write, erase and
unowned-program commands. The bounded history fuzz smoke completed 884 inputs.
The package mutation runner now includes three unowned-program witnesses in
addition to the existing controls. All 43 clean-controlled mutations passed.

`ObserveMedia` captures the complete block-enabled physical image in ascending
block order. Its geometry and erase profile accompany exact page bytes,
program markers, completed erase generations, and incomplete-erase flags. It
shares the deep-copy path with `ObserveBlock`, keeps active and incomplete
blocks visible, and does not expose logical mappings or owner identities.

`CaptureEraseIntent` records the stale-page mask before a physical erase can
clear marker evidence. It applies the same stale/free eligibility predicate as
`EraseBlock`, rejects an incomplete block, checks remaining erase
budget, and then requires a nonzero caller-selected sequence. The returned
`RetainedIntent` carries the target block, completed generation and a detached
mask in physical-page order. Capture is read-only and can include a drained
active evacuation target; the controller must retain its operation fence and
persist this value before issuing a media command.

The tests enumerate 119 nonempty stale/free layouts across 2–6-page blocks.
Each captured mask survives a maximum legal prefix erase, passes the separate
role qualifier, and restores the same page-map roles from the resulting raw
media. They also check drained and live active targets, another block during an
active evacuation, all-ones stale markers, sequence and budget order,
generation advancement, detached outputs, a 5,000-case seeded model and guided
fuzzing.

`RestoreQuiescent` reopens a block-enabled device from a detached
`MediaObservation` and `ftl-page-map.Snapshot`. It checks the bounded block
geometry, restores the settled map, then validates the erase profile, every
block, page width, erased-byte invariant, and page-role/program-marker parity
before copying physical bytes. A successful restore begins with no active
evacuation and gives each current logical owner a fresh process-local identity.

`RestoreWithInterruptedErases` uses the same ordered validation and ownership
rules while admitting feasible partial erase prefixes. In each incomplete
block, all current owners must already be stale and stale-page markers must
describe a clear prefix followed by a programmed suffix. Free pages remain
clear and do not affect prefix inference. The exact incomplete flag and erase
count survive restoration, so allocation, copy, and unowned programming keep
the whole block fenced. Shorter retries preserve progress; a completed erase
releases the block even if its acknowledgement is lost.

```go
media, err := device.ObserveMedia()
if err != nil {
    return err
}
mapSnapshot := device.Snapshot().Map
reopened, err := RestoreQuiescent(media, mapSnapshot)
```

The caller supplies both views from one quiescent recovery boundary and keeps
them stable during the call. A refusal returns the zero device. Tests compare
the constructor with a separate role/marker model, restore real write/copy/erase
histories, and check fresh owner identities and detached bytes at maximum
geometry. The retained bounded model checks 1,500+ role/marker/media
combinations; 300 seeded eight-write histories round-trip. A fixed 10-second
coverage-guided run completed 191,668 inputs. Native BUILD reported 96.6%
statement coverage, and race and vet passed. Five disposable clean-controlled
mutations for block shape, incomplete erase, erased bytes, role markers, and
fresh identities were all killed.

The interrupted-erase restore tests check 2,592 small role/marker combinations,
5,000 seeded raw observations and a byte-bounded guided history target. They
also replay a real prefix/restore/retry/completion sequence, exercise multiple
incomplete blocks, and verify that stale owner identities do not survive
reopening. The package build passed with 96.8% statement coverage, race and vet;
45,000 guided inputs and three clean-controlled interrupted-restore mutations
passed.
