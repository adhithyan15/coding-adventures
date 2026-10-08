# Changelog

## Unreleased

### Added

- `CallbackServer`, `InFlight`, `BindingResolver` and `Violation` (D18S
  P2.6d-1): the daemon answers an agent broker's eight storage callbacks
  without holding a key.
  - **Scoping:** request, channel, operation, a budget of 16, and the
    reserve-then-commit-or-abandon order.
  - **Authorization:** the binding is re-resolved each time, then
    direction, definition, membership and digest are checked.
  - **Storage checks:** grants and messages are checked without keys
    before anything is written.
- Receiver pages are bounded by count and by bytes:
  - their messages fit under the data-plane response cap, so they always
    decrypt into a response the host can take;
  - messages plus the grants they carry fit within 960 KiB.

  A page carries the receiver's grants for every epoch in it.
- The append is an explicit state machine:
  - grant lookups and grant saves at most once each, before the
    reservation;
  - one reservation;
  - one commit attempt;
  - a refused commit can still be abandoned, once.
