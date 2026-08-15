# chief-of-staff-channel-epoch-activation

Portable D18T durable epoch activation orchestrator.

This package closes the transaction boundary between the existing D18P durable
channel store, D18Q key-grant planning, and injected originator key custody. It
selects one complete successor bundle in custody before publishing any public
record, replays the exact immutable activation plan and grants after a crash,
and advances the active epoch with the same state-record CAS used by message
publication.

## Guarantees

- exact `D18S` version 2 state and `D18T` version 1 activation-plan codecs;
- migration from D18P state without clearing reservations or substituting keys;
- custody-first candidate selection with redacted handles and a durable
  production boundary;
- byte-identical plan/grant recovery across every public-write crash boundary;
- bounded activation and publish CAS loops with stable D18T errors;
- current-epoch message reservation, encryption, commit, and abandonment;
- append-only public history and explicit secret destruction after retirement;
- direct consumption of the canonical Rust-authored D18T fixture manifest.

`InMemoryKeyCustody` is deterministic test infrastructure and reports itself as
non-durable. `EpochActivationStore.open()` rejects it; production callers must
inject restart-safe custody. Tests must opt in with `openForTesting()`.

## Composition

```text
D18Q RotationPlan -> custody selection -> D18T plan + D18G replay
                                          |
D18P D18S v2 publish reservation <--------+--------> activation CAS
```

## Dependencies

- chief-of-staff-channel-store
- chief-of-staff-channel-crypto
- sha256

The package also adds two narrow primitives to the shared crypto package:
receiver-key-free D18G signature verification and D18F creation through an
injected non-exportable signer callback.

## Development

```bash
# Run tests
bash BUILD
```
