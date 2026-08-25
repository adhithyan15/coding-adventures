### Changed - preserve open-ended published task lengths (#12241)

- Allow a task length to carry only a minimum or only a maximum when that is all
  the awarding body publishes; at least one finite non-negative bound remains
  mandatory.
- Keep closed ranges backward compatible and reject reversed or wholly unknown
  bounds.
- Require the existing `notPublished` boundary note for an open length, so a
  missing maximum remains evidence rather than becoming an accidental default.

