# Chief of Staff Channel Epoch Activation (Python)

Portable Python implementation of the normative D18T durable epoch-activation
profile. Public plans and grants live in the D18P backend; originator CMKs remain
behind an injected atomic custody boundary. `InMemoryKeyCustody` is deliberately
non-durable and accepted only by `open_for_testing`.

The package consumes the canonical Rust-authored fixture manifest directly and
provides crash-safe preparation replay, revision-CAS activation, active-epoch
publish reservation, prospective revocation, and retained public destruction
history.
