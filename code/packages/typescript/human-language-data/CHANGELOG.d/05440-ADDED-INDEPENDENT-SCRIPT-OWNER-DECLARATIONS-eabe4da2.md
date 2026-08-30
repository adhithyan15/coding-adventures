### Added — Independent script-owner declarations

- Added 143 strict, per-glyph declarations for Japanese, Perso-Arabic, Tamil, and Urdu-Nastaliq so shard validation detects clean inventory-owner deletion without introducing a shared script-sized manifest.
- Bound declaration paths to fixed language, script, kind, and Unicode identity while rejecting unsafe filesystem entries, malformed or non-canonical JSON, dangerous keys, and identity collisions.
