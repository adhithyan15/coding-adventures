### Changed — full-mock papers are explicit artifacts (#17690)

- Give every built Bengali, Hindi, Malayalam, and Sanskrit full mock an
  explicit, safe-relative learner-facing paper path, separate from its rubric
  and answer key.
- Check declared papers through the assessment artifact gate, including
  symlink/junction escapes, and refuse to pin missing papers in the older
  artifact ceiling.
- Record the 198 legacy omissions in exact per-track, non-growing pins without
  silently increasing the grandfathered artifact ceiling; new omissions fail.
- Cover missing and dangling papers, unsafe paths, key-as-paper substitution,
  ancestor symlinks, pin migration, and the generator's refusal to grow debt.
