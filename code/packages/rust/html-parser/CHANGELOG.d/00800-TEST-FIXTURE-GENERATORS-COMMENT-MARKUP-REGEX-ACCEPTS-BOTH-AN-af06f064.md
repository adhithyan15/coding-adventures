- Test fixture generators: `COMMENT_MARKUP` regex accepts both `-->` and
  `--!>` comment end forms, and tag-axis classifier scans pass `re.I` even
  though their inputs are already lowercased.  Both changes silence codeql
  `py/bad-tag-filter`; runtime behaviour is unchanged.

## [0.1.0] - 2026-05-02

### Added
