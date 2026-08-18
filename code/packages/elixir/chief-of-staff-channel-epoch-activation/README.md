# coding_adventures_chief_of_staff_channel_epoch_activation

The Elixir implementation of **D18T**, the durable channel epoch-activation
profile — the transaction that makes a rotated channel key actually *current*.
This completes the six-language D18T surface.

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
  nothing published afterwards can be decrypted;
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

```elixir
alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Store, as: EpochStore

store = EpochStore.open(backend, custody, channel_id)

# Create a D18T-aware channel: the CMK enters custody before any public state.
state = EpochStore.create_epoch_channel(store, definition, initial_cmk)

# Rotate. The trusted D18Q plan plus the target roster go in; nothing public
# changes until custody has selected this candidate.
"prepared" = EpochStore.prepare_rotation(store, definition, target_roster, rotation)

# Resume after a crash. Replays the exact selected bundle; never picks another.
"idempotent" = EpochStore.recover_preparation(store, definition, new_epoch)

# Commit the transition on the shared CAS.
"activated" = EpochStore.activate_prepared_epoch(store, definition, new_epoch)

# Publish against whatever epoch is currently active.
reservation = EpochStore.reserve_publish_using_active_epoch(store, definition, request, plaintext)
```

`open/3` refuses custody whose `durable?/1` reports `false`. Use
`open_for_testing/3` with `Custody.InMemory` in tests — that module honestly
declares itself non-durable, so it cannot be wired into a real channel by
accident.

## What the errors will and will not tell you

Every failure is an `ActivationError` carrying one of the 19 stable codes in
`error_codes/0`, and the exception message is *exactly* the code — no channel
bytes, no epoch numbers, no key material. Match on `.code` rather than on
message text.

Wire failures are deliberately undifferentiated: truncation, a bad version, and
trailing bytes all report `corrupt_record`, because telling a forger how far
they got is itself a leak.

## Secret handling

`EpochKeyHandle` and `PreparedEpoch` derive `Inspect` with an empty field list,
so neither can print anything under `inspect/1`. CMK comparison delegates to
`CodingAdventures.CtCompare.ct_eq_fixed/2` rather than being reimplemented here.

`secret_erasure_capability/0` reports `not_enforceable` — immutable,
garbage-collected BEAM values cannot promise physical overwrite of every runtime
copy. The Rust reference reports `guaranteed`; Elixir does not claim that, and
the fixture test asserts the difference rather than papering over it.

## Conformance

The tests consume the canonical Rust-authored manifest directly — no local
regeneration, no shelling out to another language. The strongest of them
rebuilds the rotation candidate from the manifest's labelled test-only secrets
using Elixir's own D18Q and D18T code and requires byte-for-byte equality with
the plan and grant Rust authored. If Elixir disagrees with Rust about a single
octet, it fails.

Also proven here: v1→v2 migration preserving sequence and in-flight
reservations, crash-replay from custody alone, competing candidates where
exactly one is selected, publication and activation contending on the shared CAS
in both directions, invariant 3 verified against a backend that accepts writes
but diverges on reads, tampered custody bundles rejected, grants signed by
another originator rejected, and destruction wiping secrets while public history
stays append-only.

## Development

```sh
mix deps.get
mix test --cover
```
