# Chief of Staff Channel Store (Elixir)

This package implements the normative D18P durable-channel profile for Elixir. It provides exact D18C definitions, D18H reservations, D18S sequence state, D18A receiver cursors, deterministic storage keys, an injected atomic CAS backend, crash recovery, permanent append gaps, independent acknowledgements, opaque receiver grants, irreversible destruction, and structurally separate originator and receiver endpoints.

D18M encryption and signature handling are delegated to `chief-of-staff-channel-crypto`. Sealed-grant generation, opening, and rotation remain owned by D18G/#141; this package only persists and supplies opaque grant bodies to an injected receiver key provider.

Run the conformance suite with:

```sh
mix deps.get
mix test --cover
```
