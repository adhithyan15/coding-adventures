### Changed - the `levels ATTAINED` line names its tracks

- `renderCurriculumGapReport` printed `${count} tracks at ${level}`. The count
  had been zero for every track since the gate was written, so the populated
  branch of that expression had never once been executed — and it showed:
  `1 tracks at pre-A1` is a broken plural that does not say which track. It now
  reads `1 track at pre-A1 (spanish)`. `summary.attainedByLevel[l]` and the
  number of `tracks` whose `attained` is `l` are the same figure by
  construction, so naming them adds information without loosening the count.

