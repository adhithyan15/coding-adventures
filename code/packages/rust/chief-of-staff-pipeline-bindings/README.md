# chief-of-staff-pipeline-bindings

`chief-of-staff-pipeline-bindings` persists the manifest-blind launch authority
needed by D18 Chief hosts. One record binds an exact durable host registration
and package hash to a pipeline UUID, the pipeline's agent identity, canonical
named channel UUIDs, and optional Level 1 model settings.

Wiring fails closed unless the host is currently registered and every channel
definition is active and authorizes that agent in the requested direction.
Each channel UUID also receives an immutable create-if-absent pipeline claim, so
the same durable channel can never be reused across isolated pipelines. Claims
are intentionally retained after unwiring because channel definitions are
irreversibly destroyed and UUIDs must never be recycled.

Launch resolution repeats the registration, claim, lifecycle, and membership
checks. A stale record therefore cannot authorize a replaced package, destroyed
channel, changed topology, or cross-pipeline channel.
Callers that service the host data plane resolve the complete current binding,
including the pipeline's authorized agent identity; child launch delivery keeps
using the narrower channel/model-only view.
Unwiring is revision-CAS guarded while the host record exists and idempotent
after it is absent; immutable channel claims remain as non-authorizing audit state.

## Validation

```sh
cargo test -p chief-of-staff-pipeline-bindings -- --nocapture
cargo clippy -p chief-of-staff-pipeline-bindings --all-targets -- -D warnings
cargo doc -p chief-of-staff-pipeline-bindings --no-deps
```
