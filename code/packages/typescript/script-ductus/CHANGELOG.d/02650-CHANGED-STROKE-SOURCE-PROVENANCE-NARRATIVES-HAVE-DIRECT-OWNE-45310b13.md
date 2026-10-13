### Changed — Stroke-source provenance narratives have direct owners

- Moved the Bengali LipiTk, Gurmukhi Sikho, Malayalam Thooval/Moag/Jayasree,
  Kannada Chimple, Chinese Hanzi Writer, Latin Grundschrift/UJIpenchars2, and
  Persian/Urdu POH-Db narratives from the shared package README into regular
  source-owned files under `docs/stroke-sources/`.
- Kept the package-wide no-citation/no-pen-path rule, usage, ledger workflow,
  and security boundary in the README. The directory listing is discovery, so
  a future source adds one file without editing a manifest, index, or aggregate.
- Added a filesystem-level regression gate for strict filenames, regular files,
  a real non-symlinked owner root, reserved index/manifest names, one CommonMark
  level-1 heading, a stable shared README, and two independently discoverable
  parallel owner additions.
- Made the existing uppercase pin-name rejection fixture portable to
  case-insensitive filesystems by removing its lowercase twin before writing
  the deliberately invalid spelling.
