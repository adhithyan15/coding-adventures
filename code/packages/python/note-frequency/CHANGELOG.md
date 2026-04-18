# Changelog

## [0.1.0] - 2026-04-18

### Added
- `Note` dataclass for parsed note labels
- `parse_note()` for strings like `A4`, `C#5`, and `Db3`
- `note_to_frequency()` helper using 12-tone equal temperament
- validation for malformed note strings and unsupported accidentals
- tests covering octave changes, enharmonic equivalents, and invalid input

