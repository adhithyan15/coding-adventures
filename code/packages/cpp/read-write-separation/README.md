# read-write-separation (C++)

Capability read/write-separation (RWS) analysis, header-only in pure ISO C++17
(namespace `ca::read_write_separation`) — a faithful port of the Rust
`read-write-separation` crate.

## What it checks

An agent declares a manifest of **capabilities**, each a `(category, action,
target)` triple. The RWS principle forbids one agent from simultaneously
holding an **untrusted input** and an **external actuation**, or **overlapping
read/write** access to one resource — either lets untrusted data drive a
side effect.

Each capability has a **Flavor** (Ingestion / Actuation / Internal) and a
**Trust** (Trusted / Untrusted), inferred from the category/action when unset.

## Usage

```cpp
#include "read_write_separation.hpp"
namespace rws = ca::read_write_separation;
using rws::Capability;

std::vector<Capability> caps = {
    Capability("net", "connect", "imap.gmail.com:993")
        .with_flavor(rws::Flavor::Ingestion),
    Capability("fs", "write", "/tmp/outbox/message.txt"),
};

if (auto v = rws::validate_manifest(caps)) {
    // v->untrusted_inputs, v->actuations, v->message
}

rws::Summary s = rws::summarize_manifest(caps);   // counts + has_rws_risk() …
```

## Divergence from the Rust crate

Rust returns `Result<(), RwsViolation>`; this port's `validate_manifest` returns
`std::optional<Violation>` (empty == valid). Capabilities use value semantics
with a fluent `with_flavor` / `with_trust` / `with_justification` builder.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
