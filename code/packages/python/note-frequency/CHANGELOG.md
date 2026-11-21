# Changelog

## [0.2.0] - 2026-11-20

### Security & VER15 Hardening
- Replaced regex parsing (`re`) with a strict O(N) character-by-character parser (**CWE-1333**)
- Added `MAX_NOTE_STRING_LENGTH = 32` input length cap prior to parsing (**CWE-400**)
- Enforced strict integer validation (rejecting `bool`, `float`, `NaN`, `Inf`) and bounded octave range `[-100, 100]` (**CWE-20 / CWE-190**)
- Added `validate_frequency()`, `frequency_with_reference()`, and `note_to_frequency_with_reference()` with finite positive range enforcement (**CWE-20**)
- Added `required_capabilities.json` (`capabilities: []`) and `note_frequency.warrant`
- Added `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (SplitMix64 PRNG)

## [0.1.0] - 2026-04-18

### Added
- `Note` dataclass for parsed note labels
- `parse_note()` for strings like `A4`, `C#5`, and `Db3`
- `note_to_frequency()` helper using 12-tone equal temperament
- validation for malformed note strings and unsupported accidentals
- tests covering octave changes, enharmonic equivalents, and invalid input
