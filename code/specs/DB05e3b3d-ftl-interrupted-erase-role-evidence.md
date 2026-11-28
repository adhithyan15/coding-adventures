# DB05e3b3d — Retained page roles for an interrupted erase

An interrupted erase can clear the program marker on a stale page.
The ordered media observation retains bytes, markers, generation and
the incomplete flag. Recovery also needs the page roles that existed
when the erase was issued.
[Raw interrupted restore](DB05e3b3c-ftl-restore-interrupted-erases.md) · [Terminal media qualification](DB05e3b2c-ftl-terminal-image-qualification.md)

## Reproduced history

Create two blocks with three two-byte pages each and one logical page.
Program unowned page zero in the first run and unowned page one in
the second. Interrupt an erase of block zero after two pages in each
run. Both media observations have the same page bytes and markers,
both retain generation zero and the incomplete flag, and both have
empty logical ownership. Their original page maps differ. In the
first run the first page is stale. In the second run the second page
is stale. The simulator's raw capture reproduced both observations
and role vectors.

## Single-block qualification

Use a small pure package to reconstruct the ordered physical roles
for one incomplete block from an admitted erase intent. The recovery
stream supplies the expected nonzero intent sequence. A retained
intent supplies its sequence, block ordinal, completed-erase
generation and one stale flag per physical page. The current block
observation supplies its ordinal, generation, incomplete flag and
one program marker per physical page. The caller keeps inputs stable
until return.

Validate supported block dimensions and product through 128 physical
pages and the configured erase-count ceiling. Check the observed
block coordinate, incomplete flag and remaining erase budget.
Require exact marker and stale array lengths. Match the selected
intent sequence, block and generation. The retained mask must
contain a stale page. A set program marker requires the corresponding
stale flag. Return detached `Stale` and `Free` roles in physical
order, with a typed refusal and an empty result for invalid evidence.

Pass the recovered roles to the raw restore constructor for full
prefix and byte consistency. The stream scanner selects the active
erase intent, and the protected recovery layer retains the block
fence until durable completion. Capture of the original mask under
the block fence and its durable record format are separate slices.

Verify the two reproduced histories with their own retained masks.
Enumerate small marker and stale masks, competing evidence at the
same generation, wrong block or generation, zero and maximum sequence,
budget exhaustion, role detachment and refusal precedence. Compare
with a separate oracle. Check normal native fronts, bounded guided
fuzz and clean-controlled mutations for dropping the sequence check,
accepting a programmed free page, and forgetting to detach the roles.

## Local verification

The qualifier in `30448037587` matched 43,680 decisions from a separate
mask-subset oracle, including 2,172 detached accepted outcomes. The
independent tests checked ordered refusals with multiple faults, zero
allocations on those refusals and the maximum geometry and generation.

The emitted Go recipe passed from a path containing spaces with 100%
statement coverage, race and vet. The native profile also passed 100,000
guided inputs, three clean-controlled mutations and focused QEMU Linux 386
execution. The Windows amd64 test binary cross-compiled. The graph gained
one package and one Linux recipe while all previous rows matched.
