### Changed — curriculum extension membership to path owners (#15729)

- Make tagged path lessons the single owner of unambiguous extension
  membership and derive the unchanged public extension arrays at load time.
- Preserve explicit arrays for ambiguous shapes and reject malformed, unknown,
  unattached, duplicate, or dual-owned membership.
