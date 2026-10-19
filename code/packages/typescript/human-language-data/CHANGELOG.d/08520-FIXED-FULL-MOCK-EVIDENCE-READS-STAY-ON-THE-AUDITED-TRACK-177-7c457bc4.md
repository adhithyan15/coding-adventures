### Fixed — full-mock evidence reads stay on the audited track (#17706)

- Read declared manifest, paper, rubric, and answer-key bytes through pinned
  regular-file handles beneath each assessment track on Windows and Unix.
- Fail the required artifact gate when a concurrent ancestor swap makes a
  previously present file indeterminate; never count outside-track bytes as
  complete mock evidence.
- Reuse the existing JSON, unsafe-key, and per-part content validation on the
  bytes returned by that boundary, with cross-platform swap regression tests.
