# chief-of-staff-channel-epoch-activation (Go)

The Go implementation of **D18T**, the durable channel epoch-activation profile
— the transaction that makes a rotated channel key actually *current*.

Spec: [`code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md`](../../../specs/D18T-chief-of-staff-durable-epoch-activation-profile.md)

## Where this sits in the stack

```
D18P channel-store   durable messages, grants, cursors, sequence reservations
D18Q channel-crypto  mints the next CMK and seals one grant per receiver
D18T (this package)  makes epoch E+1 current, crash-safely, without racing publication
```

D18P can store. D18Q can rotate. Neither can *activate* — and the naive
implementation of activation has two ways to lose data:

- write "epoch E+1 is current", then crash before the new CMK is durable, and
  nobody can decrypt anything published afterwards;
- activate E+1 while a concurrent publisher is reserving a slot at E, and the
  message ends up encrypted under a key its header disagrees with.

## The one idea worth remembering

**The active epoch lives in the same versioned record as the pending publish
reservation.** A separate mutable "epoch head" would not be conforming: two
independent compare-and-swap operations cannot exclude each other, so a
publisher could reserve against the old epoch in the window between activation
reading the head and writing it. One record means one revision means one CAS,
and exactly one of {publish, activate} wins.

The second idea: **custody is claimed before anything becomes public.** The
secret CMK and the complete public recovery bundle are stored together,
atomically, by a single custody call — before the plan or any grant is written.
A crash before that call leaves no candidate at all; a crash after it is fully
recoverable from custody plus the public store, with no byte regenerated.

## Usage

```go
store, err := epochactivation.NewStore(backend, custody, channelID)

// Create a D18T-aware channel: the CMK enters custody before any public state.
state, err := store.CreateEpochChannel(definition, initialCMK)

// Rotate. The trusted D18Q plan plus the target roster go in; nothing public
// changes until custody has selected this candidate.
outcome, err := store.PrepareRotation(definition, targetRoster, rotationPlan)

// Resume after a crash. Replays the exact selected bundle; never picks another.
outcome, err = store.RecoverPreparation(definition, newEpoch)

// Commit the transition on the shared CAS.
activated, err := store.ActivatePreparedEpoch(definition, newEpoch)

// Publish against whatever epoch is currently active.
reservation, err := store.ReservePublishUsingActiveEpoch(definition, request, plaintext)
```

`NewStore` refuses custody that reports `Durable() == false`. Use
`NewStoreForTesting` with `InMemoryKeyCustody` in tests — that type honestly
declares itself non-durable, so it cannot be wired into a real channel by
accident.

## What the errors will and will not tell you

Every failure is one of the 19 stable codes in `EpochActivationErrorCodes`, and
the error's message is *exactly* the code — no channel bytes, no epoch numbers,
no key material. Use `epochactivation.IsCode(err, epochactivation.ErrPendingAppend)`
rather than string matching.

Wire failures are deliberately undifferentiated: truncation, a bad version, and
trailing bytes all report `corrupt_record`, because telling a forger how far
they got is itself a leak.

## Secret handling

`EpochKeyHandle` and `PreparedEpoch` render as `[REDACTED]` under both `%v` and
`%#v`. CMK comparisons run in constant time. `SecretErasureCapability()` reports
`best_effort` — Go clears owned slices on controlled destruction, but value
copies, the garbage collector, and intermediates inside the repository crypto
primitives are outside this package's reach. The Rust reference reports
`guaranteed`; Go does not claim that, and the fixture test asserts the
difference rather than papering over it.

## Conformance

The tests consume the canonical Rust-authored manifest directly — no local
regeneration, no shelling out to another language. The strongest of them
rebuilds the rotation candidate from the manifest's labelled test-only secrets
using Go's own D18Q and D18T code and requires byte-for-byte equality with the
plan and grant Rust authored. If Go disagrees with Rust about a single octet,
it fails.

Also proven here: v1→v2 migration preserving sequence and in-flight
reservations, crash-replay from custody alone, competing candidates where
exactly one is selected, publication and activation contending on the shared
CAS in both directions, bounded 16-attempt CAS giving up rather than forcing a
write, tampered custody bundles rejected, grants signed by another originator
rejected, and destruction wiping secrets while public history stays append-only.

## Development

```sh
go test ./... -v -cover
go vet ./...
```
