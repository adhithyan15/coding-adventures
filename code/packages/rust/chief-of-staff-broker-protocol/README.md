# chief-of-staff-broker-protocol

The frames between the Chief supervisor and one agent's broker (D18S S-K7,
step 6 P2.6d-1).

A broker is one process per agent. It holds that agent's channel keys and
nothing else. The supervisor relays the agent's channel requests to it, and
answers the narrow storage callbacks it makes while serving them. This crate
is every byte that crosses between the two. It holds no keys and opens
nothing.

```text
supervisor                                     broker
Bootstrap { binding, key slots }  ─────────►   read key descriptors 3..3+n
                                  ◄─────────   Ready { public halves }
Request(Publish)                  ─────────►
                                  ◄─────────   Callback(ReserveAppend)
CallbackResult(Reserved header)   ─────────►   encrypt and sign
                                  ◄─────────   Callback(CommitAppend)
CallbackResult(Committed)         ─────────►
                                  ◄─────────   Response(Published)
```

## Frames

Each frame is a 4-byte big-endian length followed by a body of 1 byte to
1 MiB. The body starts with `D18K`, the version byte `1`, and a kind byte:

| Kind | Direction | Frame |
|---|---|---|
| `0x01` | to broker | `Bootstrap`: the resolved pipeline binding, and what each inherited key descriptor holds |
| `0x02` | to broker | `Request`: one host request, already checked and rate-limited |
| `0x03` | to broker | `CallbackResult`: an answer or a refusal, naming the callback it answers |
| `0x04` | to broker | `Terminate` |
| `0x81` | from broker | `Ready`: the loaded keys' public halves |
| `0x82` | from broker | `Callback`: one storage operation for the request in flight |
| `0x83` | from broker | `Response`: the answer to the request in flight |

A frame sent in the wrong direction is refused by its kind alone.

## Callbacks

There are eight callbacks, each shaped for one operation; there is no
generic get or put:

- `LoadDefinition`
- `ReadReceiverPage`: messages plus the receiver's grants
- `Acknowledge`
- `LoadMissingGrants`
- `SaveGrants`
- `ReserveAppend`: a plaintext *hash*, never the plaintext
- `CommitAppend`
- `AbandonAppend`

None of them names who is asking. The daemon knows that from the pipe the
callback arrived on. Each callback carries the digest of the definition the
broker is working from, so it never acts on stale membership.

## Bounds

| Field | Bound |
|---|---|
| frame body | 1 MiB |
| encoded host binding | 32 KiB |
| key slots | 256 (two per write channel, one per read channel) |
| messages per page | 64 whole messages, at most 960 KiB in total |
| grants, or receiver indices, per callback | 1024 |
| content type | 1 to 1024 bytes of UTF-8 |
| callbacks per request | 16 (enforced by the daemon) |

Decoding is total:
- every count and length is checked before anything is allocated;
- every nested record is decoded by its owning crate's own codec;
- trailing bytes, an unknown kind and a wrong version are all refused.

`Debug` on a callback or a reply prints only the operation, never a
message or a grant.

## Usage

```rust,ignore
use chief_of_staff_broker_protocol::{encode_to_broker, write_frame, ToBroker};

write_frame(&mut broker_stdin, &encode_to_broker(&ToBroker::Terminate)?)?;
```

```sh
cargo test -p chief-of-staff-broker-protocol
```
