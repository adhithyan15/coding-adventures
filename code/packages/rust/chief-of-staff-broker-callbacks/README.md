# chief-of-staff-broker-callbacks

The daemon's side of an agent broker's storage callbacks (D18S S-K7, step 6
P2.6d-1). It holds no channel key, and treats every callback as hostile.

A broker holds one agent's keys, and the daemon keeps storage. While the
broker serves a request, it asks for exactly the storage that request
needs. `CallbackServer::serve` answers, in three steps:

1. **Does the callback fit the request in flight?** It must have the same
   request id and the same channel, an operation allowed for that kind of
   request, and budget left (16 per request). A Publish gets one
   reservation, then one commit or one abandon of exactly that sequence.

   No means a `Violation`: an honest broker never does this, so the caller
   ends the broker.
2. **Is it authorized right now?** The binding is re-resolved on every
   callback through `BindingResolver`. The channel must be bound in the
   right direction (read channels get receiver operations, write channels
   originator operations), and the definition must be active, name this
   agent, and match the digest the broker sent.

   No means a `Refusal`: an ordinary answer, which the broker maps to the
   host's failure code.
3. **The operation**, with key-free checks on anything stored:
   - **Grants:** this originator's signature, the current epoch, and a
     receiver that is in the definition; checked all-or-nothing, and
     stored write-once.
   - **Messages:** the pending header (which this daemon minted, with its
     own message id and timestamp), and the originator's signature.

It cannot check whether a ciphertext decrypts; that needs the channel key.
D18S P2.6d records what follows from that.

## Identity

No callback says who is asking. Identity is the binding of the pipe the
callback came in on, and unwiring the pipeline revokes a running broker at
its next callback (S-K2).

## Tests

`tests/callbacks.rs` plays an honest broker, and then a dishonest one:
- a message published and received entirely through callbacks;
- no plaintext in storage;
- an empty channel reads as empty;
- every violation;
- every refusal: direction, impostor, unwired, stale, destroyed;
- grant checks, all-or-nothing and write-once;
- a forged commit;
- the page byte budget, including a single message too large for any page.

Every check was mutation-tested: removing it fails a test.

```sh
cargo test -p chief-of-staff-broker-callbacks
```
