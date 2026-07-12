# read-write-separation (C)

Capability read/write-separation (RWS) analysis, in pure ISO C17 — a faithful
port of the Rust `read-write-separation` crate.

## What it checks

An agent declares a manifest of **capabilities**, each a `(category, action,
target)` triple — e.g. `("net", "connect", "smtp.gmail.com:465")`. The RWS
principle forbids one agent from simultaneously holding:

- an **untrusted input** *and* an **external actuation** — untrusted data must
  not be able to drive a side-effecting action; or
- **overlapping read and write** access to the same resource.

Each capability has a **flavor** (Ingestion / Actuation / Internal) and a
**trust** (Trusted / Untrusted). When unset these are inferred: `net connect`
defaults to an untrusted actuation, `fs read` of a `package:`-internal target is
trusted, and so on.

## API

```c
#include "read_write_separation.h"

RwsCapability caps[2];
rws_capability_init(&caps[0], "net", "connect", "imap.gmail.com:993");
rws_capability_set_flavor(&caps[0], RWS_FLAVOR_INGESTION);
rws_capability_init(&caps[1], "fs", "write", "/tmp/outbox/message.txt");

RwsViolation v;
if (rws_validate(caps, 2, &v) == RWS_VIOLATION) {
    /* v.untrusted_inputs[0], v.actuations[0], v.message */
    rws_violation_release(&v);
}
rws_capability_release(&caps[0]);
rws_capability_release(&caps[1]);
```

`rws_summarize` returns aggregate counts (`RwsSummary`) with `has_rws_risk` /
`has_same_resource_overlap` helpers; `rws_classify` resolves a single
capability's flavor/trust/input flags.

## Divergence from the Rust crate

Rust returns `Result<(), RwsViolation>` holding cloned capabilities; this port
returns an `RwsStatus` and the `RwsViolation` borrows pointers into the analyzed
array (keep it alive) plus an owned message. OOM surfaces as `RWS_ERR_NOMEM`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).
