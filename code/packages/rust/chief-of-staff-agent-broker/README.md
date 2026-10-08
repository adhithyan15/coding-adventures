# chief-of-staff-agent-broker

One agent's broker (D18S S-K7, step 6 P2.6d-1): the only process that holds
that agent's channel keys.

The Chief daemon used to serve every agent's channel operations itself,
from one map of every agent's keys. A bug in that code was a bug that
exposed everyone. Now each agent gets a broker process holding *its own*
keys and nothing else:
- the supervisor relays the agent's Receive, Publish and Acknowledge
  requests to it;
- the broker reaches storage only through the daemon's per-operation
  callbacks (`chief-of-staff-broker-callbacks`), and the daemon holds no
  channel key.

```text
supervisor ── Bootstrap, Request, CallbackResult ──►  broker (fds 3..3+n: keys)
           ◄── Ready, Callback, Response ───────────
```

## The process

1. It suppresses its own core dumps (P2.6a), before any key exists.
2. It reads `Bootstrap` from stdin: the resolved pipeline binding, and
   which key each inherited descriptor holds.
3. It checks that descriptors 3 to 3+n are open and 3+n+1 is not: exactly
   the keys, nothing else.
4. It reads each key, re-checking the owner-only policy on the descriptor
   itself, and closes the descriptor.
5. It reports `Ready` with each key's public half, so the supervisor can
   check the keys against the channel definitions.
6. It serves one request at a time until `Terminate`.

Any failure exits non-zero, writes nothing, and the supervisor ends the
agent.

## What it checks, and what it no longer trusts

- **Key slots** must match the binding exactly:
  - one receiver key per read channel;
  - one signing seed and one channel key per write channel;
  - no duplicates;
  - nothing unbound;
  - no all-zero key.
- **Its own place in each definition:** it must be the originator or a
  receiver, with the public key its own key implies.
- **Every header the daemon reserves:**
  - it is the channel, originator, content type, plaintext hash and epoch
    the broker asked for, with a valid message id;
  - its sequence is above any it has encrypted under before. The sequence
    is the nonce, so this refuses nonce reuse for the broker's lifetime.

  Otherwise it gives the reservation back (`AbandonAppend`) and fails the
  request.
- **Every callback reply** must answer the callback it was asked;
  anything else is fatal.
- **A refused commit** is followed by an abandon of that reservation, so
  the channel is not left stuck behind it.
- **Descriptors:** at start it scans every descriptor number up to the
  open-file limit (capped at 65,536). Anything open above the key slots
  stops it.

## What it does not defend against

The daemon. The daemon supplies the channel definition, including the
receiver list, so a compromised daemon could add a receiver and have the
broker seal the channel key to a key of its choosing. That is within the
design: the daemon can open every key file anyway (D18S P2.6d, key
custody). Key custody here decides which address space holds the keys. It
is not a defence against the supervisor. What the broker does defend
against is a daemon that replays or alters a reserved header, as above.

## Behaviour

It matches the old in-daemon dispatcher's checks and failure codes. Two
deliberate differences:
- **Delivery receipts are per broker.** One agent that never acknowledges
  can only fill its own 4096, not everyone's.
- **Receipts die with the broker.** The host receives again, as it would
  after a daemon restart.

## Tests

- `tests/engine.rs`: two brokers, publisher and receiver, each with only
  its own keys, carry a message end to end through the real callback
  server. Also:
  - refusals that need no callback;
  - wrong keys;
  - a destroyed channel;
  - a missing grant;
  - a lying daemon (four header lies, and a rewound sequence);
  - a reply to the wrong callback;
  - every key-slot rule.
- `tests/broker_process.rs` (Unix): the real binary on inherited
  descriptors, publishing through framed callbacks. It refuses:
  - an extra descriptor;
  - a missing slot;
  - group-readable, short, long or all-zero key files;
  - a broken first frame;
  - a reply to the wrong callback;
  - a second Bootstrap.

  It also checks that the running broker's core limit reads 0/0.

```sh
cargo test -p chief-of-staff-agent-broker
```
