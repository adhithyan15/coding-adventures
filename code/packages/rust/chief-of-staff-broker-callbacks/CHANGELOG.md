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
- Receiver pages are bounded by bytes (960 KiB of whole messages) as well
  as by count, and carry the receiver's grants for every epoch in the page.
