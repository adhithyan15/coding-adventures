### Changed — remaining Indian-track changelogs now have stable owners

- Migrated Bengali, Gujarati, Kannada, Marathi, Marwadi, Punjabi, Sanskrit,
  Tamil, Telugu, and Urdu release notes to strict append-only `CHANGELOG.d/`
  fragments.
- Preserved all 294 historical sections byte-for-byte, pinned by their original
  sizes and SHA-256 hashes.
- Extended CI's deletion and aggregate-resurrection guards to every new owner
  directory and render target.
