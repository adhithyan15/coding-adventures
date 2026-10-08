# chief-of-staff-broker-e2e

End-to-end tests for D18S P2.6d-2b, which gives every agent its own
channel broker. Not published: this crate exists so the three processes
involved can be tested together.

## What runs

```
 test (supervisor) ──spawns──▶ broker-e2e-broker   (keys on fds 3..3+n)
        │                           ▲  callbacks: reserve, commit, abandon
        │                           │
        └──────spawns──▶ broker-e2e-host ──Publish──▶ supervisor ──▶ broker
```

1. The test builds a `ProcessHostSupervisor` with `ChannelBrokers`. The
   broker binary is pinned by its SHA-256, and the keys are files in a
   scratch directory with mode 0600.
2. `start` launches the agent's broker first. The broker gets exactly its
   key descriptors and reports its public keys in `Ready`, and the launcher
   checks them against the channel definitions. Only then is the host
   spawned.
3. The host's channel requests go to its broker. Every storage operation
   the broker needs comes back to the supervisor as a callback, so the
   broker never touches storage itself.
4. Each side's end ends the other.

## Why separate binaries

`broker-e2e-broker` calls `chief_of_staff_agent_broker::serve_process`,
the same function the production binary runs. It is a separate binary only
because Cargo builds `CARGO_BIN_EXE_*` paths for the testing crate's own
binaries. `broker-e2e-host` is a host scripted by markers in its package:

| Marker         | Behaviour                                              |
|----------------|--------------------------------------------------------|
| `EXIT_AT_ONCE` | exit right after saying Ready                          |
| `PUBLISH`      | publish once, write the response to the named file     |
| (none)         | say Ready, then wait to be stopped                     |

## Running

```sh
cargo test -p chief-of-staff-broker-e2e
```

The tests are Linux-only: the verified launch (`execveat` on the hashed
descriptor) exists only there so far.
